use chrono::{DateTime, Utc};
use thiserror::Error;

use learn_core::{
    model::{Card, CardId, DeckId, ReviewGrade},
    scheduler::{AppliedReview, MemoryState, Scheduler, SchedulerError},
    time::Clock,
};
use storage::repository::{CardRepository, ReviewLogRecord, ReviewPersistence, StorageError};

const SECONDS_PER_DAY: f64 = 86_400.0;

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReviewServiceError {
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

//
// ─── REVIEW RESULT ─────────────────────────────────────────────────────────────
//

/// Result of processing a review: selected schedule, memory update, and log.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewResult {
    pub applied: AppliedReview,
}

/// Result of a persisted review: updated card, applied outcome, and log ID.
///
/// This struct encapsulates the card after review, the ID of the persisted review log,
/// and the detailed review result.
#[derive(Debug, Clone, PartialEq)]
pub struct PersistedReview {
    pub card: Card,
    pub log_id: i64,
    pub result: ReviewResult,
}

//
// ─── ELAPSED DAYS ──────────────────────────────────────────────────────────────
//

/// Compute elapsed days between the last review and the current review time.
///
/// - Returns `0.0` for brand-new cards (no prior review).
/// - Returns a negative value if the review is backdated; this will surface as
///   `SchedulerError::InvalidElapsedDays` when passed to the scheduler.
///
/// ```ignore
/// # use chrono::Utc;
/// # use learn_services::review_service::compute_elapsed_days;
/// let now = Utc::now();
/// assert_eq!(compute_elapsed_days(None, now), 0.0);
///
/// let earlier = now - chrono::Duration::days(1);
/// assert!(compute_elapsed_days(Some(now), earlier) < 0.0);
/// ```
#[must_use]
pub fn compute_elapsed_days(
    last_review_at: Option<DateTime<Utc>>,
    reviewed_at: DateTime<Utc>,
) -> f64 {
    match last_review_at {
        Some(last) => {
            let seconds = reviewed_at.signed_duration_since(last).num_seconds();

            // NOTE: `num_seconds()` returns `i64`. Converting to `f64` may lose
            // precision for extremely large durations, but review intervals in
            // this app are bounded to human timescales.
            #[allow(clippy::cast_precision_loss)]
            let seconds_f = seconds as f64;

            seconds_f / SECONDS_PER_DAY
        }
        None => 0.0,
    }
}

//
// ─── SERVICE ───────────────────────────────────────────────────────────────────
//

/// Coordinates applying a user's review to a card using the scheduler.
pub struct ReviewService {
    clock: Clock,
    scheduler: Scheduler,
}

impl ReviewService {
    /// Create a new review service using default FSRS scheduler and real-time clock.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` if the underlying scheduler fails to initialize.
    pub fn new() -> Result<Self, ReviewServiceError> {
        Ok(Self {
            clock: Clock::default(),
            scheduler: Scheduler::new()?,
        })
    }

    /// Create a review service with a custom scheduler (still uses default clock).
    #[must_use]
    pub fn with_scheduler(scheduler: Scheduler) -> Self {
        Self {
            clock: Clock::default(),
            scheduler,
        }
    }

    /// Override the clock (usually for deterministic testing).
    #[must_use]
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Current time according to the service's clock.
    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Apply a user's grade to a card, updating its scheduling state and returning the log/outcome.
    ///
    /// - Computes elapsed days from the card's last review when available; uses 0 for new cards.
    /// - Uses `Scheduler::apply_review` to produce the next schedule and memory update.
    ///
    /// # Errors
    ///
    /// Propagates scheduler errors produced by the underlying FSRS scheduler.
    ///
    /// ```ignore
    /// # use chrono::Utc;
    /// # use learn_services::review_service::ReviewService;
    /// # use learn_core::model::{content::ContentDraft, CardId, DeckId, ReviewGrade};
    /// let mut card = {
    ///     let prompt = ContentDraft::text_only("Q").validate(Utc::now(), None, None).unwrap();
    ///     let answer = ContentDraft::text_only("A").validate(Utc::now(), None, None).unwrap();
    ///     let now = Utc::now();
    ///     learn_core::model::Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap()
    /// };
    ///
    /// let service = ReviewService::new().unwrap();
    /// let reviewed_at = service.now();
    /// let result = service
    ///     .review_card(&mut card, ReviewGrade::Good, reviewed_at)
    ///     .unwrap();
    /// assert_eq!(result.applied.log.card_id, card.id());
    /// ```
    pub fn review_card(
        &self,
        card: &mut Card,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
    ) -> Result<ReviewResult, ReviewServiceError> {
        let previous_state: Option<MemoryState> = card.memory_state();
        let elapsed_days = compute_elapsed_days(card.last_review_at(), reviewed_at);

        let applied = self.scheduler.apply_review(
            card.id(),
            previous_state.as_ref(),
            grade,
            reviewed_at,
            elapsed_days,
        )?;

        card.apply_review_with_phase(grade, &applied.outcome, reviewed_at);

        Ok(ReviewResult { applied })
    }

    /// Apply a review to an in-memory card and persist the update + log atomically.
    ///
    /// If persistence fails, the card is rolled back to its original state.
    ///
    /// # Errors
    ///
    /// Returns scheduler errors for invalid elapsed time or FSRS failures.
    /// Returns storage errors if persistence fails.
    pub async fn review_card_persisted(
        &self,
        card: &mut Card,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
        reviews: &dyn ReviewPersistence,
    ) -> Result<(ReviewResult, i64), ReviewServiceError> {
        let original = card.clone();

        let result = self.review_card(card, grade, reviewed_at)?;

        let record = ReviewLogRecord::from_applied(
            card.deck_id(),
            &result.applied.log,
            &result.applied.outcome,
        );

        match reviews.apply_review(card, record).await {
            Ok(id) => Ok((result, id)),
            Err(err) => {
                *card = original;
                Err(err.into())
            }
        }
    }

    /// Load a card, apply a review, and persist the updated card and review log atomically.
    ///
    /// Uses the service clock for `reviewed_at` to keep time deterministic.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the card is missing.
    /// Returns scheduler errors for invalid elapsed time or FSRS failures.
    /// Returns storage errors if persistence fails.
    pub async fn review_card_persisted_by_id(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        cards: &dyn CardRepository,
        reviews: &dyn ReviewPersistence,
        grade: ReviewGrade,
    ) -> Result<PersistedReview, ReviewServiceError> {
        let mut card = cards
            .get_cards(deck_id, &[card_id])
            .await?
            .into_iter()
            .next()
            .ok_or(StorageError::NotFound)?;

        let reviewed_at = self.now();
        let (result, log_id) = self
            .review_card_persisted(&mut card, grade, reviewed_at, reviews)
            .await?;

        Ok(PersistedReview {
            card,
            log_id,
            result,
        })
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::{
        model::{CardId, Deck, DeckId, DeckSettings, content::ContentDraft},
        scheduler::Scheduler,
        time::fixed_now,
    };
    use storage::repository::{DeckRepository, InMemoryRepository, ReviewLogRepository};

    fn build_card() -> Card {
        let prompt = ContentDraft::text_only("What is 2+2?")
            .validate(fixed_now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("4")
            .validate(fixed_now(), None, None)
            .unwrap();
        let now = fixed_now();
        Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap()
    }

    fn build_deck() -> Deck {
        Deck::new(
            DeckId::new(1),
            "Test",
            None,
            DeckSettings::default_for_adhd(),
            fixed_now(),
        )
        .unwrap()
    }

    #[test]
    fn review_new_card_updates_state_and_log() {
        let mut card = build_card();
        let fixed = fixed_now();
        let service = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(fixed));

        let reviewed_at = service.now();
        let result = service
            .review_card(&mut card, ReviewGrade::Good, reviewed_at)
            .unwrap();

        assert_eq!(result.applied.log.card_id, card.id());
        assert_eq!(result.applied.log.grade, ReviewGrade::Good);
        assert_eq!(card.review_count(), 1);
        assert_eq!(card.last_review_at(), Some(fixed));
        assert!(card.next_review_at() >= fixed);
    }

    #[test]
    fn review_existing_card_uses_elapsed_days() {
        let mut card = build_card();
        let now = fixed_now();
        let scheduler = Scheduler::new().unwrap();

        // First review manually through scheduler
        let first = scheduler
            .apply_review(card.id(), None, ReviewGrade::Good, now, 0.0)
            .unwrap();
        card.apply_review(&first.outcome, now);

        // Second review through service, with clock advanced 3 days
        let advanced = now + chrono::Duration::days(3);
        let service = ReviewService::with_scheduler(scheduler).with_clock(Clock::fixed(advanced));

        let result = service
            .review_card(&mut card, ReviewGrade::Hard, service.now())
            .unwrap();

        assert!(result.applied.outcome.scheduled_days >= 1.0);
        assert_eq!(card.review_count(), 2);
    }

    #[test]
    fn review_with_backdated_timestamp_errors() {
        let mut card = build_card();
        let now = fixed_now();

        // First review at time = now
        let service1 = ReviewService::new().unwrap().with_clock(Clock::fixed(now));
        service1
            .review_card(&mut card, ReviewGrade::Good, service1.now())
            .unwrap();

        // Second review uses earlier time -> negative elapsed
        let earlier = now - chrono::Duration::days(1);
        let service2 = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(earlier));

        let err = service2
            .review_card(&mut card, ReviewGrade::Good, earlier)
            .unwrap_err();

        assert!(matches!(
            err,
            ReviewServiceError::Scheduler(SchedulerError::InvalidElapsedDays { .. })
        ));
    }

    #[test]
    fn compute_elapsed_days_respects_backdating_and_initial() {
        let now = fixed_now();
        assert_eq!(compute_elapsed_days(None, now), 0.0);

        let earlier = now - chrono::Duration::days(2);
        let later = now + chrono::Duration::days(2);

        assert!(compute_elapsed_days(Some(now), earlier) < 0.0);
        assert!(compute_elapsed_days(Some(now), later) > 0.0);
    }

    #[tokio::test]
    async fn review_card_persisted_updates_card_and_log() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card = build_card();
        repo.upsert_card(&card).await.unwrap();

        let service = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(fixed_now()));
        let result = service
            .review_card_persisted_by_id(deck.id(), card.id(), &repo, &repo, ReviewGrade::Hard)
            .await
            .unwrap();

        assert_eq!(result.card.review_count(), 1);
        let logs = repo.logs_for_card(deck.id(), card.id()).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].id, Some(result.log_id));
        assert_eq!(logs[0].grade, ReviewGrade::Hard);
    }
}

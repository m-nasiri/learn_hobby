use chrono::{DateTime, Utc};
use thiserror::Error;

use learn_core::{
    model::{Card, ReviewGrade},
    scheduler::{AppliedReview, MemoryState, Scheduler, SchedulerError},
    time::Clock,
};

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq)]
#[non_exhaustive]
pub enum ReviewServiceError {
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
}

//
// ─── REVIEW RESULT ─────────────────────────────────────────────────────────────
//

/// Result of processing a review: selected schedule, memory update, and log.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewResult {
    pub applied: AppliedReview,
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
        #[allow(clippy::cast_precision_loss)]
        Some(last) => reviewed_at.signed_duration_since(last).num_seconds() as f64 / 86_400.0,
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
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::{
        model::{CardId, DeckId, content::ContentDraft},
        scheduler::Scheduler,
        time::fixed_now,
    };

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

    #[test]
    fn review_new_card_updates_state_and_log() {
        let mut card = build_card();
        let fixed = fixed_now();
        let service = ReviewService::new().unwrap().with_clock(Clock::fixed(fixed));

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
        let service1 = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(now));
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
}

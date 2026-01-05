use chrono::{DateTime, Utc};
use learn_core::{
    model::{Card, CardId, DeckId, DeckSettings, ReviewGrade},
    scheduler::{AppliedReview, MemoryState, Scheduler},
    time::Clock,
};
use storage::repository::{CardRepository, ReviewLogRecord, ReviewPersistence, StorageError};

const SECONDS_PER_DAY: f64 = 86_400.0;
use crate::error::ReviewServiceError;

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewResult {
    pub applied: AppliedReview,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistedReview {
    pub card: Card,
    pub log_id: i64,
    pub result: ReviewResult,
}

#[must_use]
pub fn compute_elapsed_days(
    last_review_at: Option<DateTime<Utc>>,
    reviewed_at: DateTime<Utc>,
) -> f64 {
    match last_review_at {
        Some(last) => {
            let seconds = reviewed_at.signed_duration_since(last).num_seconds();
            #[allow(clippy::cast_precision_loss)]
            let seconds_f = seconds as f64;
            seconds_f / SECONDS_PER_DAY
        }
        None => 0.0,
    }
}

fn apply_lapse_min_interval(
    applied: &mut AppliedReview,
    reviewed_at: DateTime<Utc>,
    min_interval_days: u32,
) {
    if min_interval_days == 0 {
        return;
    }
    let min_days = f64::from(min_interval_days);
    if applied.outcome.scheduled_days < min_days {
        applied.outcome.scheduled_days = min_days;
        applied.outcome.next_review =
            reviewed_at + chrono::Duration::days(i64::from(min_interval_days));
    }
}

pub struct ReviewService {
    clock: Clock,
    scheduler: Scheduler,
}

impl ReviewService {
    /// Create a new review service using default scheduler and clock.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` if scheduler initialization fails.
    pub fn new() -> Result<Self, ReviewServiceError> {
        Ok(Self {
            clock: Clock::default(),
            scheduler: Scheduler::new()?,
        })
    }

    #[must_use]
    pub fn with_scheduler(scheduler: Scheduler) -> Self {
        Self {
            clock: Clock::default(),
            scheduler,
        }
    }

    #[must_use]
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Apply a grade to a card and return the scheduler output.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` on scheduler failures.
    pub fn review_card(
        &self,
        card: &mut Card,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
    ) -> Result<ReviewResult, ReviewServiceError> {
        let defaults = DeckSettings::default_for_adhd();
        self.review_card_with_settings(card, grade, reviewed_at, &defaults)
    }

    /// Apply a grade using deck settings and return the scheduler output.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` on scheduler failures.
    pub fn review_card_with_settings(
        &self,
        card: &mut Card,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
        settings: &DeckSettings,
    ) -> Result<ReviewResult, ReviewServiceError> {
        let mut previous_state: Option<MemoryState> = card.memory_state();
        let is_lapse = matches!(
            card.phase(),
            learn_core::model::CardPhase::Reviewing | learn_core::model::CardPhase::Relearning
        ) && grade == ReviewGrade::Again;
        if is_lapse && !settings.preserve_stability_on_lapse() {
            previous_state = None;
        }

        let elapsed_days = compute_elapsed_days(card.last_review_at(), reviewed_at);
        let mut applied = self.scheduler.apply_review(
            card.id(),
            previous_state.as_ref(),
            grade,
            reviewed_at,
            elapsed_days,
        )?;

        if is_lapse {
            apply_lapse_min_interval(
                &mut applied,
                reviewed_at,
                settings.lapse_min_interval_secs(),
            );
        }

        card.apply_review_with_phase(grade, &applied.outcome, reviewed_at);

        Ok(ReviewResult { applied })
    }

    /// Apply a grade, persist the updated card and review log atomically.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` on scheduler failures.
    /// Returns `ReviewServiceError::Storage` on persistence failures.
    pub async fn review_card_persisted(
        &self,
        card: &mut Card,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
        reviews: &dyn ReviewPersistence,
    ) -> Result<(ReviewResult, i64), ReviewServiceError> {
        let defaults = DeckSettings::default_for_adhd();
        self.review_card_persisted_with_settings(card, grade, reviewed_at, &defaults, reviews)
            .await
    }

    /// Apply a grade with settings, persist the updated card and review log atomically.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` on scheduler failures.
    /// Returns `ReviewServiceError::Storage` on persistence failures.
    pub async fn review_card_persisted_with_settings(
        &self,
        card: &mut Card,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
        settings: &DeckSettings,
        reviews: &dyn ReviewPersistence,
    ) -> Result<(ReviewResult, i64), ReviewServiceError> {
        let original = card.clone();
        let result = self.review_card_with_settings(card, grade, reviewed_at, settings)?;

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

    /// Load a card by ID, apply a grade, and persist the update atomically.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Storage` if the card is missing or persistence fails.
    /// Returns `ReviewServiceError::Scheduler` on scheduler failures.
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

    /// Persist a batch of already-applied reviews.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Storage` if any persistence call fails.
    pub async fn persist_applied_reviews(
        &self,
        deck_id: DeckId,
        items: impl IntoIterator<Item = (Card, AppliedReview)>,
        reviews: &dyn ReviewPersistence,
    ) -> Result<Vec<i64>, ReviewServiceError> {
        let mut ids = Vec::new();

        for (card, applied) in items {
            let record = ReviewLogRecord::from_applied(deck_id, &applied.log, &applied.outcome);
            let id = reviews.apply_review(&card, record).await?;
            ids.push(id);
        }

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use learn_core::model::{Card, CardId, ContentDraft, DeckId};
    use learn_core::time::fixed_now;

    fn build_card(now: DateTime<Utc>) -> Card {
        let prompt = ContentDraft::text_only("Q")
            .validate(now, None, None)
            .unwrap();
        let answer = ContentDraft::text_only("A")
            .validate(now, None, None)
            .unwrap();
        Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap()
    }

    #[test]
    fn lapse_min_interval_clamps_again_outcome() {
        let now = fixed_now();
        let mut card = build_card(now);
        let defaults = DeckSettings::default_for_adhd();
        let service = ReviewService::new().unwrap().with_clock(Clock::Fixed(now));

        service
            .review_card_with_settings(&mut card, ReviewGrade::Good, now, &defaults)
            .unwrap();
        let second_review = now + chrono::Duration::days(1);
        service
            .review_card_with_settings(&mut card, ReviewGrade::Good, second_review, &defaults)
            .unwrap();

        let lapse_settings = DeckSettings::new(5, 30, 5, true, true, 3 * 86_400).unwrap();
        let lapse_review = now + chrono::Duration::days(2);
        let result = service
            .review_card_with_settings(&mut card, ReviewGrade::Again, lapse_review, &lapse_settings)
            .unwrap();

        let scheduled = result.applied.outcome.next_review - lapse_review;
        assert!(scheduled >= chrono::Duration::days(3));
    }

    #[test]
    fn lapse_without_preserve_stability_uses_new_card_schedule() {
        let now = fixed_now();
        let service = ReviewService::new().unwrap().with_clock(Clock::Fixed(now));
        let mut card = build_card(now);

        let defaults = DeckSettings::default_for_adhd();
        let first_review = now;
        let second_review = now + chrono::Duration::days(1);
        let lapse_review = now + chrono::Duration::days(2);
        service
            .review_card_with_settings(&mut card, ReviewGrade::Good, first_review, &defaults)
            .unwrap();
        service
            .review_card_with_settings(&mut card, ReviewGrade::Good, second_review, &defaults)
            .unwrap();

        let lapse_settings = DeckSettings::new(5, 30, 5, true, false, 86_400).unwrap();
        let result = service
            .review_card_with_settings(&mut card, ReviewGrade::Again, lapse_review, &lapse_settings)
            .unwrap();

        assert_eq!(result.applied.outcome.elapsed_days, 0.0);
    }
}

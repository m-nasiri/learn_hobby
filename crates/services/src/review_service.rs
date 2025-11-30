use chrono::{DateTime, Utc};
use thiserror::Error;

use learn_core::{
    model::{Card, ReviewGrade},
    scheduler::{AppliedReview, MemoryState, Scheduler, SchedulerError},
};

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq)]
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

/// Compute elapsed days between the last review and the current review time.
///
/// - Returns `0.0` for brand-new cards (no prior review).
/// - Returns a negative value if the review is backdated; this will surface as
///   `SchedulerError::InvalidElapsedDays` when passed to the scheduler.
///
/// # Examples
///
/// ```
/// # use chrono::Utc;
/// # use services::review_service::compute_elapsed_days;
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
    scheduler: Scheduler,
}

impl ReviewService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
        }
    }

    #[must_use]
    pub fn with_scheduler(scheduler: Scheduler) -> Self {
        Self { scheduler }
    }

    /// Apply a user's grade to a card, updating its scheduling state and returning the log/outcome.
    ///
    /// - Computes elapsed days from the card's last review when available; uses 0 for new cards.
    /// - Uses `Scheduler::apply_review` to produce the next schedule and memory update.
    ///
    /// # Errors
    ///
    /// Propagates scheduler errors (invalid elapsed days, FSRS failures).
    ///
    /// # Examples
    ///
    /// Backdated reviews surface an error:
    ///
    /// ```
    /// # use chrono::Utc;
    /// # use services::review_service::ReviewService;
    /// # use learn_core::model::{content::ContentDraft, CardId, DeckId, ReviewGrade};
    /// let mut card = {
    ///     let prompt = ContentDraft::text_only("Q").validate(Utc::now(), None, None).unwrap();
    ///     let answer = ContentDraft::text_only("A").validate(Utc::now(), None, None).unwrap();
    ///     let now = Utc::now();
    ///     learn_core::model::Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap()
    /// };
    ///
    /// let service = ReviewService::new();
    /// let reviewed_at = Utc::now();
    /// service.review_card(&mut card, ReviewGrade::Good, reviewed_at).unwrap();
    ///
    /// // Backdate the next review: this yields an error because elapsed is negative.
    /// let backdated = reviewed_at - chrono::Duration::days(1);
    /// assert!(service.review_card(&mut card, ReviewGrade::Good, backdated).is_err());
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

        card.apply_review(&applied.outcome, reviewed_at);

        Ok(ReviewResult { applied })
    }
}

impl Default for ReviewService {
    fn default() -> Self {
        Self::new()
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
    };

    fn build_card() -> Card {
        let prompt = ContentDraft::text_only("What is 2+2?")
            .validate(Utc::now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("4")
            .validate(Utc::now(), None, None)
            .unwrap();
        let now = Utc::now();
        Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap()
    }

    #[test]
    fn review_new_card_updates_state_and_log() {
        let mut card = build_card();
        let service = ReviewService::new();
        let reviewed_at = Utc::now();

        let result = service
            .review_card(&mut card, ReviewGrade::Good, reviewed_at)
            .unwrap();

        assert_eq!(result.applied.log.card_id, card.id());
        assert_eq!(result.applied.log.grade, ReviewGrade::Good);
        assert_eq!(card.review_count(), 1);
        assert_eq!(card.last_review_at(), Some(reviewed_at));
        assert!(card.next_review_at() >= reviewed_at);
    }

    #[test]
    fn review_existing_card_uses_elapsed_days() {
        let mut card = build_card();
        let scheduler = Scheduler::new();
        let first_review_at = Utc::now();
        let first = scheduler
            .apply_review(card.id(), None, ReviewGrade::Good, first_review_at, 0.0)
            .unwrap();
        card.apply_review(&first.outcome, first_review_at);

        let service = ReviewService::with_scheduler(scheduler);
        let second_at = first_review_at + chrono::Duration::days(3);

        let result = service
            .review_card(&mut card, ReviewGrade::Hard, second_at)
            .unwrap();

        // Should respect elapsed time and not shrink interval below 1 day.
        assert!(result.applied.outcome.scheduled_days >= 1.0);
        assert_eq!(card.review_count(), 2);
    }

    #[test]
    fn review_with_backdated_timestamp_errors() {
        let mut card = build_card();
        let service = ReviewService::new();
        let reviewed_at = Utc::now();

        // First review establishes last_review_at
        service
            .review_card(&mut card, ReviewGrade::Good, reviewed_at)
            .unwrap();

        // Backdated review (negative elapsed) should propagate scheduler error
        let earlier = reviewed_at - chrono::Duration::days(1);
        let err = service
            .review_card(&mut card, ReviewGrade::Good, earlier)
            .unwrap_err();

        assert!(matches!(
            err,
            ReviewServiceError::Scheduler(SchedulerError::InvalidElapsedDays { .. })
        ));
    }

    #[test]
    fn compute_elapsed_days_respects_backdating_and_initial() {
        let now = Utc::now();
        assert_eq!(compute_elapsed_days(None, now), 0.0);

        let earlier = now - chrono::Duration::days(2);
        let later = now + chrono::Duration::days(2);

        // backdated review produces negative elapsed
        assert!(compute_elapsed_days(Some(now), earlier) < 0.0);
        // forward-dated review produces positive elapsed
        assert!(compute_elapsed_days(Some(now), later) > 0.0);
    }
}

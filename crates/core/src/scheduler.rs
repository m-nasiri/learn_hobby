use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::{CardId, ReviewGrade, ReviewLog, ReviewOutcome};

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SchedulerError {
    #[error("FSRS scheduling failed: {0}")]
    FsrsError(String),
    #[error("optimal retention must be in (0, 1], got {provided}")]
    InvalidRetention { provided: f32 },
    #[error("elapsed days must be non-negative and finite, got {provided}")]
    InvalidElapsedDays { provided: f64 },
}

//
// ─── MEMORY STATE ──────────────────────────────────────────────────────────────
//

/// Serializable card memory state used by FSRS.
///
/// Store this with each `Card` to maintain accurate scheduling across reviews.
///
/// # Fields
///
/// * `stability` - How long the memory will last (higher = longer retention)
/// * `difficulty` - How hard the card is (0-10, higher = harder to remember)
///
/// # Examples
///
/// ```
/// # use learn_core::scheduler::MemoryState;
/// let state = MemoryState::new(5.0, 3.5);
/// assert_eq!(state.stability, 5.0);
/// assert_eq!(state.difficulty, 3.5);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryState {
    pub stability: f64,
    pub difficulty: f64,
}

impl MemoryState {
    #[must_use]
    pub fn new(stability: f64, difficulty: f64) -> Self {
        Self {
            stability,
            difficulty,
        }
    }

    #[must_use]
    pub fn from_outcome(outcome: &ReviewOutcome) -> Self {
        Self {
            stability: outcome.stability,
            difficulty: outcome.difficulty,
        }
    }
}

//
// ─── SCHEDULED STATES ──────────────────────────────────────────────────────────
//

/// All possible next review states for a card.
///
/// After scheduling a review, select the appropriate outcome based on
/// the user's rating using the `.select(grade)` method.
///
/// # Fields
///
/// * `card_id` - The card being scheduled
/// * `again` - User failed to recall (shortest interval)
/// * `hard` - User recalled with difficulty
/// * `good` - User recalled correctly
/// * `easy` - User recalled instantly (longest interval)
///
/// # Examples
///
/// ```
/// # use learn_core::scheduler::Scheduler;
/// # use learn_core::model::{CardId, ReviewGrade};
/// let scheduler = Scheduler::new();
/// let now = chrono::Utc::now();
/// let states = scheduler.schedule_new_card(CardId::new(1), now)?;
///
/// // Select outcome based on user's rating
/// let outcome = states.select(ReviewGrade::Good);
/// # Ok::<(), learn_core::scheduler::SchedulerError>(())
/// ```
#[derive(Debug, Clone)]
pub struct ScheduledStates {
    pub card_id: CardId,
    pub again: ReviewOutcome,
    pub hard: ReviewOutcome,
    pub good: ReviewOutcome,
    pub easy: ReviewOutcome,
}

impl ScheduledStates {
    #[must_use]
    pub fn select(&self, grade: ReviewGrade) -> &ReviewOutcome {
        match grade {
            ReviewGrade::Again => &self.again,
            ReviewGrade::Hard => &self.hard,
            ReviewGrade::Good => &self.good,
            ReviewGrade::Easy => &self.easy,
        }
    }
}

//
// ─── SCHEDULER ─────────────────────────────────────────────────────────────────
//

/// FSRS-based scheduler for spaced repetition.
///
/// The `Scheduler` uses the FSRS algorithm to calculate optimal review intervals
/// based on card difficulty and memory stability. It supports configurable
/// retention rates (default 0.9 = 90% recall probability).
///
/// # ADHD-Friendly Design
///
/// - Default 90% retention balances learning efficiency with manageable review frequency
/// - Provides immediate feedback on next review timing
/// - Adapts intervals based on actual performance
///
/// # Examples
///
/// ```
/// # use learn_core::scheduler::Scheduler;
/// # use learn_core::model::{CardId, ReviewGrade};
/// let scheduler = Scheduler::new();
/// let now = chrono::Utc::now();
/// let states = scheduler.schedule_new_card(CardId::new(1), now)?;
/// let outcome = states.select(ReviewGrade::Good);
///
/// println!("Next review in {} days", outcome.scheduled_days);
/// # Ok::<(), learn_core::scheduler::SchedulerError>(())
/// ```
pub struct Scheduler {
    fsrs: fsrs::FSRS,
    optimal_retention: f32,
}

/// Outcome of applying a review: log entry, chosen schedule, and updated memory state.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedReview {
    pub log: ReviewLog,
    pub outcome: ReviewOutcome,
    pub memory: MemoryState,
}

impl Scheduler {
    /// Create scheduler with default parameters and 0.9 retention.
    #[must_use]
    pub fn new() -> Self {
        Self::with_retention(0.9)
    }

    /// Create scheduler with custom desired retention without panicking.
    ///
    /// # Errors
    ///
    /// - `InvalidRetention` if `optimal_retention` is not in `(0, 1]`
    /// - `FsrsError` if FSRS initialization fails
    pub fn try_with_retention(optimal_retention: f32) -> Result<Self, SchedulerError> {
        if !(0.0..=1.0).contains(&optimal_retention) || optimal_retention == 0.0 {
            return Err(SchedulerError::InvalidRetention {
                provided: optimal_retention,
            });
        }

        let fsrs = fsrs::FSRS::new(Some(&[]))
            .map_err(|e| SchedulerError::FsrsError(e.to_string()))?;

        Ok(Self {
            fsrs,
            optimal_retention,
        })
    }

    /// Create scheduler with custom desired retention.
    ///
    /// # Panics
    ///
    /// Panics if FSRS initialization fails (should not happen with default parameters).
    #[must_use]
    pub fn with_retention(optimal_retention: f32) -> Self {
        Self::try_with_retention(optimal_retention)
            .expect("FSRS initialization with default parameters should not fail")
    }

    /// Schedule a brand-new card (no previous state).
    ///
    /// Returns all four possible next states (again/hard/good/easy).
    /// Select one based on the user's rating.
    ///
    /// # Errors
    ///
    /// Returns `SchedulerError::FsrsError` if FSRS scheduling fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use learn_core::scheduler::Scheduler;
    /// # use learn_core::model::{CardId, ReviewGrade};
    /// let scheduler = Scheduler::new();
    /// let now = chrono::Utc::now();
    /// let states = scheduler.schedule_new_card(CardId::new(1), now)?;
    /// let outcome = states.select(ReviewGrade::Good);
    /// # Ok::<(), learn_core::scheduler::SchedulerError>(())
    /// ```
    pub fn schedule_new_card(
        &self,
        card_id: CardId,
        reviewed_at: DateTime<Utc>,
    ) -> Result<ScheduledStates, SchedulerError> {
        let next = self
            .fsrs
            .next_states(None, self.optimal_retention, 0)
            .map_err(|e| SchedulerError::FsrsError(e.to_string()))?;

        Ok(ScheduledStates {
            card_id,
            again: self.to_outcome(&next.again, reviewed_at, 0.0),
            hard: self.to_outcome(&next.hard, reviewed_at, 0.0),
            good: self.to_outcome(&next.good, reviewed_at, 0.0),
            easy: self.to_outcome(&next.easy, reviewed_at, 0.0),
        })
    }

    /// Schedule a review for an existing card.
    ///
    /// Takes the card's current `MemoryState` and calculates next review states
    /// based on how much time has elapsed since the last review.
    ///
    /// # Arguments
    ///
    /// * `card_id` - Card being reviewed
    /// * `state` - Current memory state (stability & difficulty from last review)
    /// * `elapsed_days` - Days since last review (calculate from `ReviewLog` timestamps)
    ///
    /// # Errors
    ///
    /// Returns `SchedulerError::FsrsError` if FSRS scheduling fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use learn_core::scheduler::{Scheduler, MemoryState};
    /// # use learn_core::model::{CardId, ReviewGrade};
    /// let scheduler = Scheduler::new();
    ///
    /// // First review
    /// let now = chrono::Utc::now();
    /// let states = scheduler.schedule_new_card(CardId::new(1), now)?;
    /// let outcome = states.select(ReviewGrade::Good);
    ///
    /// // Save the memory state with the card
    /// let memory = MemoryState::new(outcome.stability, outcome.difficulty);
    ///
    /// // Later: next review (3 days elapsed)
    /// let states = scheduler.schedule_review(CardId::new(1), &memory, 3.0, now)?;
    /// # Ok::<(), learn_core::scheduler::SchedulerError>(())
    /// ```
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn schedule_review(
        &self,
        card_id: CardId,
        state: &MemoryState,
        elapsed_days: f64,
        reviewed_at: DateTime<Utc>,
    ) -> Result<ScheduledStates, SchedulerError> {
        if !elapsed_days.is_finite() || elapsed_days < 0.0 {
            return Err(SchedulerError::InvalidElapsedDays { provided: elapsed_days });
        }

        let fsrs_state = fsrs::MemoryState {
            stability: state.stability as f32,
            difficulty: state.difficulty as f32,
        };

        let next = self
            .fsrs
            .next_states(
                Some(fsrs_state),
                self.optimal_retention,
                elapsed_days.round() as u32,
            )
            .map_err(|e| SchedulerError::FsrsError(e.to_string()))?;

        Ok(ScheduledStates {
            card_id,
            again: self.to_outcome(&next.again, reviewed_at, elapsed_days),
            hard: self.to_outcome(&next.hard, reviewed_at, elapsed_days),
            good: self.to_outcome(&next.good, reviewed_at, elapsed_days),
            easy: self.to_outcome(&next.easy, reviewed_at, elapsed_days),
        })
    }

    /// Convert domain grade to FSRS rating (1–4).
    #[must_use]
    pub fn grade_to_rating(grade: ReviewGrade) -> u8 {
        match grade {
            ReviewGrade::Again => 1,
            ReviewGrade::Hard => 2,
            ReviewGrade::Good => 3,
            ReviewGrade::Easy => 4,
        }
    }

    /// Convert an FSRS `ItemState` into your `ReviewOutcome`.
    #[allow(clippy::cast_possible_truncation, clippy::unused_self)]
    fn to_outcome(
        &self,
        item: &fsrs::ItemState,
        now: DateTime<Utc>,
        elapsed_days: f64,
    ) -> ReviewOutcome {
        // fsrs-rs example rounds interval and clamps to >= 1 day
        let interval_days = item.interval.round().max(1.0);
        let next_review = now + Duration::days(interval_days as i64);

        ReviewOutcome::new(
            next_review,
            f64::from(item.memory.stability),
            f64::from(item.memory.difficulty),
            elapsed_days,
            f64::from(interval_days),
        )
    }

    #[must_use]
    pub fn optimal_retention(&self) -> f32 {
        self.optimal_retention
    }

    /// Apply a user's review and return the selected schedule, memory update, and log entry.
    ///
    /// - For brand-new cards, pass `None` for `previous_state` (elapsed days ignored).
    /// - For existing cards, provide the stored `MemoryState` and the elapsed days since last review.
    ///
    /// # Errors
    ///
    /// Returns `SchedulerError::InvalidElapsedDays` if elapsed is negative or non-finite.
    /// Returns `SchedulerError::FsrsError` if FSRS scheduling fails.
    pub fn apply_review(
        &self,
        card_id: CardId,
        previous_state: Option<&MemoryState>,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
        elapsed_days: f64,
    ) -> Result<AppliedReview, SchedulerError> {
        let states = match previous_state {
            Some(state) => self.schedule_review(card_id, state, elapsed_days, reviewed_at)?,
            None => self.schedule_new_card(card_id, reviewed_at)?,
        };

        let outcome = states.select(grade).clone();
        let memory = MemoryState::from_outcome(&outcome);
        let log = ReviewLog::new(card_id, grade, reviewed_at);

        Ok(AppliedReview {
            log,
            outcome,
            memory,
        })
    }
}

impl Default for Scheduler {
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

    #[test]
    fn scheduler_default_retention() {
        let s = Scheduler::new();
        assert!((s.optimal_retention() - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn grade_to_rating_mapping() {
        assert_eq!(Scheduler::grade_to_rating(ReviewGrade::Again), 1);
        assert_eq!(Scheduler::grade_to_rating(ReviewGrade::Hard), 2);
        assert_eq!(Scheduler::grade_to_rating(ReviewGrade::Good), 3);
        assert_eq!(Scheduler::grade_to_rating(ReviewGrade::Easy), 4);
    }

    #[test]
    fn schedule_new_card_produces_all_states() {
        let s = Scheduler::new();
        let now = Utc::now();
        let states = s.schedule_new_card(CardId::new(1), now).unwrap();

        assert_eq!(states.card_id, CardId::new(1));

        // intervals should be monotonic again <= hard <= good <= easy
        assert!(states.again.scheduled_days <= states.hard.scheduled_days);
        assert!(states.hard.scheduled_days <= states.good.scheduled_days);
        assert!(states.good.scheduled_days <= states.easy.scheduled_days);

        // all scheduled days should be >= 1 because we clamp.
        assert!(states.again.scheduled_days >= 1.0);
        assert!(states.hard.scheduled_days >= 1.0);
        assert!(states.good.scheduled_days >= 1.0);
        assert!(states.easy.scheduled_days >= 1.0);
    }

    #[test]
    fn schedule_review_increases_interval_on_good() {
        let s = Scheduler::new();

        let first = s
            .schedule_new_card(CardId::new(1), Utc::now())
            .unwrap();
        let first_good = first.good.clone();

        let mem = MemoryState::new(first_good.stability, first_good.difficulty);

        let later = s
            .schedule_review(CardId::new(1), &mem, 3.0, Utc::now())
            .unwrap();
        let later_good = later.good.clone();

        assert!(later_good.scheduled_days >= first_good.scheduled_days);
        assert!(later_good.stability >= first_good.stability);
    }

    #[test]
    fn again_is_shorter_than_good_after_review() {
        let s = Scheduler::new();

        let first = s
            .schedule_new_card(CardId::new(1), Utc::now())
            .unwrap();
        let first_good = first.good.clone();

        let mem = MemoryState::new(first_good.stability, first_good.difficulty);

        let next = s
            .schedule_review(CardId::new(1), &mem, 4.0, Utc::now())
            .unwrap();

        assert!(next.again.scheduled_days < next.good.scheduled_days);
    }

    #[test]
    fn schedule_review_rejects_negative_elapsed() {
        let s = Scheduler::new();
        let outcome = s
            .schedule_new_card(CardId::new(1), Utc::now())
            .unwrap()
            .good
            .clone();
        let memory = MemoryState::from_outcome(&outcome);

        let err = s
            .schedule_review(CardId::new(1), &memory, -1.0, Utc::now())
            .unwrap_err();
        assert!(matches!(
            err,
            SchedulerError::InvalidElapsedDays { provided } if provided == -1.0
        ));
    }

    #[test]
    fn try_with_retention_rejects_invalid_values() {
        assert!(matches!(
            Scheduler::try_with_retention(0.0),
            Err(SchedulerError::InvalidRetention { .. })
        ));
        assert!(matches!(
            Scheduler::try_with_retention(1.5),
            Err(SchedulerError::InvalidRetention { .. })
        ));
    }

    #[test]
    fn memory_state_from_outcome_round_trips() {
        let s = Scheduler::new();
        let states = s
            .schedule_new_card(CardId::new(1), Utc::now())
            .unwrap();
        let outcome = states.good.clone();

        let memory = MemoryState::from_outcome(&outcome);
        assert_eq!(memory.stability, outcome.stability);
        assert_eq!(memory.difficulty, outcome.difficulty);
    }

    #[test]
    fn apply_review_new_card_returns_log_and_memory() {
        let s = Scheduler::new();
        let now = Utc::now();

        let applied = s
            .apply_review(CardId::new(1), None, ReviewGrade::Good, now, 0.0)
            .unwrap();

        assert_eq!(applied.log.card_id, CardId::new(1));
        assert_eq!(applied.log.grade, ReviewGrade::Good);
        assert_eq!(applied.log.reviewed_at, now);
        assert_eq!(applied.memory.stability, applied.outcome.stability);

        let direct = s
            .schedule_new_card(CardId::new(1), now)
            .unwrap();
        assert_eq!(
            applied.outcome.scheduled_days,
            direct.good.scheduled_days
        );
    }

    #[test]
    fn apply_review_existing_card_matches_schedule_review() {
        let s = Scheduler::new();
        let first = s
            .schedule_new_card(CardId::new(1), Utc::now())
            .unwrap();
        let memory = MemoryState::from_outcome(&first.good);

        let elapsed = 2.7;
        let now = Utc::now();

        let applied = s
            .apply_review(
                CardId::new(1),
                Some(&memory),
                ReviewGrade::Hard,
                now,
                elapsed,
            )
            .unwrap();
        let direct = s
            .schedule_review(CardId::new(1), &memory, elapsed, now)
            .unwrap();

        assert_eq!(applied.outcome.scheduled_days, direct.hard.scheduled_days);
        assert!(applied.outcome.scheduled_days >= 1.0);
    }

    #[test]
    fn apply_review_rejects_invalid_elapsed_days() {
        let s = Scheduler::new();
        let first = s
            .schedule_new_card(CardId::new(1), Utc::now())
            .unwrap();
        let memory = MemoryState::from_outcome(&first.good);

        let err = s
            .apply_review(
                CardId::new(1),
                Some(&memory),
                ReviewGrade::Good,
                Utc::now(),
                f64::NAN,
            )
            .unwrap_err();

        assert!(matches!(err, SchedulerError::InvalidElapsedDays { .. }));
    }

    #[test]
    fn schedule_new_monotonic_for_various_retentions() {
        for retention in [0.7_f32, 0.9_f32, 1.0_f32] {
            let s = Scheduler::try_with_retention(retention).unwrap();
            let states = s
                .schedule_new_card(CardId::new(99), Utc::now())
                .unwrap();
            assert!(states.again.scheduled_days <= states.hard.scheduled_days);
            assert!(states.hard.scheduled_days <= states.good.scheduled_days);
            assert!(states.good.scheduled_days <= states.easy.scheduled_days);
        }
    }

    #[test]
    fn select_picks_correct_outcome() {
        let s = Scheduler::new();
        let states = s
            .schedule_new_card(CardId::new(1), Utc::now())
            .unwrap();

        assert_eq!(
            states.select(ReviewGrade::Good).scheduled_days,
            states.good.scheduled_days
        );
        assert_eq!(
            states.select(ReviewGrade::Again).scheduled_days,
            states.again.scheduled_days
        );
    }
}

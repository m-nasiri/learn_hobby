use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::ids::CardId;

//
// ─── ERRORS ───────────────────────────────────────────────────────────────────
//

/// Errors that can occur during review operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReviewError {
    #[error("invalid review grade value: {0}")]
    InvalidGrade(u8),
}

//
// ─── REVIEW GRADE ─────────────────────────────────────────────────────────────
//

/// Four-level difficulty rating for card reviews.
///
/// Grades map to the FSRS algorithm's difficulty levels:
/// - `Again`: Failed to recall, card needs immediate review
/// - `Hard`: Recalled with significant difficulty
/// - `Good`: Recalled correctly with appropriate effort
/// - `Easy`: Recalled instantly with no effort
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewGrade {
    /// Failed to recall the answer. Card will be shown again soon.
    Again,
    /// Recalled with significant difficulty. Interval increases slowly.
    Hard,
    /// Recalled correctly with appropriate effort. Standard interval increase.
    Good,
    /// Recalled instantly. Interval increases significantly.
    Easy,
}

impl ReviewGrade {
    /// Converts a numeric grade (0-3) to a `ReviewGrade`.
    ///
    /// # Errors
    ///
    /// Returns `ReviewError::InvalidGrade` if the value is not in the range 0-3.
    pub fn from_u8(value: u8) -> Result<Self, ReviewError> {
        match value {
            0 => Ok(Self::Again),
            1 => Ok(Self::Hard),
            2 => Ok(Self::Good),
            3 => Ok(Self::Easy),
            _ => Err(ReviewError::InvalidGrade(value)),
        }
    }

    /// Maps this grade to the FSRS 1-4 rating scale.
    #[must_use]
    pub fn to_fsrs_rating(self) -> u8 {
        match self {
            ReviewGrade::Again => 1,
            ReviewGrade::Hard => 2,
            ReviewGrade::Good => 3,
            ReviewGrade::Easy => 4,
        }
    }
}

//
// ─── REVIEW LOG ───────────────────────────────────────────────────────────────
//

/// Record of a single card review event.
///
/// Stores which card was reviewed, when, and what grade was given.
/// Used for tracking study history and analytics.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewLog {
    pub card_id: CardId,
    pub reviewed_at: DateTime<Utc>,
    pub grade: ReviewGrade,
}

impl ReviewLog {
    #[must_use]
    pub fn new(card_id: CardId, grade: ReviewGrade, reviewed_at: DateTime<Utc>) -> Self {
        Self {
            card_id,
            reviewed_at,
            grade,
        }
    }
}

//
// ─── REVIEW OUTCOME ──────────────────────────────────────────────────────────
//

/// Output from the FSRS scheduling algorithm.
///
/// Contains the calculated next review time and memory metrics for a card.
/// These values are computed by the FSRS algorithm based on review history.
///
/// # Fields
///
/// - `next_review`: When the card should be reviewed next
/// - `stability`: Memory stability (higher = longer retention)
/// - `difficulty`: Card difficulty (0-10, higher = harder)
/// - `elapsed_days`: Days since last review
/// - `scheduled_days`: Days until next review (interval length)
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewOutcome {
    pub next_review: DateTime<Utc>,

    pub stability: f64,
    pub difficulty: f64,
    pub elapsed_days: f64,
    pub scheduled_days: f64,
}

impl ReviewOutcome {
    #[must_use]
    pub fn new(
        next_review: DateTime<Utc>,
        stability: f64,
        difficulty: f64,
        elapsed_days: f64,
        scheduled_days: f64,
    ) -> Self {
        Self {
            next_review,
            stability,
            difficulty,
            elapsed_days,
            scheduled_days,
        }
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_grade_conversion_works() {
        assert_eq!(ReviewGrade::from_u8(0).unwrap(), ReviewGrade::Again);
        assert_eq!(ReviewGrade::from_u8(3).unwrap(), ReviewGrade::Easy);
        let err = ReviewGrade::from_u8(5).unwrap_err();
        assert!(matches!(err, ReviewError::InvalidGrade(5)));
    }

    #[test]
    fn to_fsrs_rating_mapping_is_correct() {
        assert_eq!(ReviewGrade::Again.to_fsrs_rating(), 1);
        assert_eq!(ReviewGrade::Hard.to_fsrs_rating(), 2);
        assert_eq!(ReviewGrade::Good.to_fsrs_rating(), 3);
        assert_eq!(ReviewGrade::Easy.to_fsrs_rating(), 4);
    }

    #[test]
    fn log_creation_works() {
        let log = ReviewLog::new(CardId::new(10), ReviewGrade::Good, Utc::now());
        assert_eq!(log.card_id, CardId::new(10));
        assert_eq!(log.grade, ReviewGrade::Good);
    }

    #[test]
    fn outcome_creation_works() {
        let now = Utc::now();
        let out = ReviewOutcome::new(now, 3.0, 4.0, 1.0, 5.0);

        assert_eq!(out.next_review, now);
        assert_eq!(out.stability, 3.0);
    }
}

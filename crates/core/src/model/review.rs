use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::ids::CardId;

//
// ─── ERRORS ───────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReviewError {
    #[error("invalid review grade")]
    InvalidGrade,
}

//
// ─── REVIEW GRADE ─────────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewGrade {
    Again,
    Hard,
    Good,
    Easy,
}

impl ReviewGrade {
    pub fn from_u8(value: u8) -> Result<Self, ReviewError> {
        match value {
            0 => Ok(Self::Again),
            1 => Ok(Self::Hard),
            2 => Ok(Self::Good),
            3 => Ok(Self::Easy),
            _ => Err(ReviewError::InvalidGrade),
        }
    }
}

//
// ─── REVIEW LOG ───────────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewLog {
    pub card_id: CardId,
    pub reviewed_at: DateTime<Utc>,
    pub grade: ReviewGrade,
}

impl ReviewLog {
    pub fn new(card_id: CardId, grade: ReviewGrade, reviewed_at: DateTime<Utc>) -> Self {
        Self {
            card_id,
            grade,
            reviewed_at,
        }
    }
}

//
// ─── REVIEW OUTCOME ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewOutcome {
    pub next_review: DateTime<Utc>,

    pub stability: f64,
    pub difficulty: f64,
    pub elapsed_days: f64,
    pub scheduled_days: f64,
}

impl ReviewOutcome {
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
        assert!(ReviewGrade::from_u8(5).is_err());
    }

    #[test]
    fn log_creation_works() {
        let log = ReviewLog::new(CardId(10), ReviewGrade::Good, Utc::now());
        assert_eq!(log.card_id, CardId(10));
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

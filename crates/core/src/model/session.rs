use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::{DeckId, ReviewGrade, ReviewLog};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionSummaryError {
    #[error("completed_at is before started_at")]
    InvalidTimeRange,

    #[error("too many logs for a single session: {len}")]
    TooManyLogs { len: usize },

    #[error("total reviews ({total}) does not match grade counts ({sum})")]
    CountMismatch { total: u32, sum: u32 },
}

/// Aggregate summary for a completed review session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    deck_id: DeckId,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    total_reviews: u32,
    again: u32,
    hard: u32,
    good: u32,
    easy: u32,
}

impl SessionSummary {
    /// Rehydrate a session summary from persisted storage.
    ///
    /// # Errors
    ///
    /// Returns `SessionSummaryError::CountMismatch` if totals do not align.
    #[allow(clippy::too_many_arguments)]
    pub fn from_persisted(
        deck_id: DeckId,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        total_reviews: u32,
        again: u32,
        hard: u32,
        good: u32,
        easy: u32,
    ) -> Result<Self, SessionSummaryError> {
        if completed_at < started_at {
            return Err(SessionSummaryError::InvalidTimeRange);
        }
        let sum = again + hard + good + easy;
        if sum != total_reviews {
            return Err(SessionSummaryError::CountMismatch {
                total: total_reviews,
                sum,
            });
        }

        Ok(Self {
            deck_id,
            started_at,
            completed_at,
            total_reviews,
            again,
            hard,
            good,
            easy,
        })
    }

    /// Build a summary from a list of review logs.
    ///
    /// # Errors
    ///
    /// Returns `SessionSummaryError::InvalidTimeRange` if `completed_at` is before `started_at`.
    /// Returns `SessionSummaryError::TooManyLogs` if the log count cannot fit in `u32`.
    pub fn from_logs(
        deck_id: DeckId,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        logs: &[ReviewLog],
    ) -> Result<Self, SessionSummaryError> {
        if completed_at < started_at {
            return Err(SessionSummaryError::InvalidTimeRange);
        }
        let mut again = 0_u32;
        let mut hard = 0_u32;
        let mut good = 0_u32;
        let mut easy = 0_u32;

        for log in logs {
            match log.grade {
                ReviewGrade::Again => again = again.saturating_add(1),
                ReviewGrade::Hard => hard = hard.saturating_add(1),
                ReviewGrade::Good => good = good.saturating_add(1),
                ReviewGrade::Easy => easy = easy.saturating_add(1),
            }
        }

        let total_reviews = u32::try_from(logs.len())
            .map_err(|_| SessionSummaryError::TooManyLogs { len: logs.len() })?;

        Self::from_persisted(
            deck_id,
            started_at,
            completed_at,
            total_reviews,
            again,
            hard,
            good,
            easy,
        )
    }

    #[must_use]
    pub fn deck_id(&self) -> DeckId {
        self.deck_id
    }

    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    #[must_use]
    pub fn completed_at(&self) -> DateTime<Utc> {
        self.completed_at
    }

    #[must_use]
    pub fn total_reviews(&self) -> u32 {
        self.total_reviews
    }

    #[must_use]
    pub fn again(&self) -> u32 {
        self.again
    }

    #[must_use]
    pub fn hard(&self) -> u32 {
        self.hard
    }

    #[must_use]
    pub fn good(&self) -> u32 {
        self.good
    }

    #[must_use]
    pub fn easy(&self) -> u32 {
        self.easy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::fixed_now;

    #[test]
    fn summary_counts_grades() {
        let now = fixed_now();
        let logs = vec![
            ReviewLog::new(crate::model::CardId::new(1), ReviewGrade::Good, now),
            ReviewLog::new(crate::model::CardId::new(2), ReviewGrade::Again, now),
            ReviewLog::new(crate::model::CardId::new(3), ReviewGrade::Hard, now),
            ReviewLog::new(crate::model::CardId::new(4), ReviewGrade::Easy, now),
            ReviewLog::new(crate::model::CardId::new(5), ReviewGrade::Good, now),
        ];

        let summary = SessionSummary::from_logs(DeckId::new(10), now, now, &logs).unwrap();

        assert_eq!(summary.total_reviews(), 5);
        assert_eq!(summary.again(), 1);
        assert_eq!(summary.hard(), 1);
        assert_eq!(summary.good(), 2);
        assert_eq!(summary.easy(), 1);
    }
}

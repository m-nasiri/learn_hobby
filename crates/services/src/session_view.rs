use chrono::{DateTime, Utc};
use std::sync::Arc;

use learn_core::model::{DeckId, SessionSummary};
use storage::repository::SessionSummaryRepository;

use crate::Clock;
use crate::session_service::{SessionError, SessionService};

/// Storage identifier for a persisted session summary.
///
/// NOTE: This is currently `i64` to match `SQLite` row IDs.
pub type SessionSummaryId = i64;

/// Presentation-agnostic list item for a session summary.
///
/// This is intentionally **not** a UI view-model:
/// - no pre-formatted strings
/// - no localization assumptions
///
/// The UI may format timestamps (e.g., relative time, locale) as needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummaryListItem {
    pub id: SessionSummaryId,
    pub completed_at: DateTime<Utc>,

    pub total: u32,
    pub again: u32,
    pub hard: u32,
    pub good: u32,
    pub easy: u32,
}

impl SessionSummaryListItem {
    #[must_use]
    pub fn from_summary(id: SessionSummaryId, summary: &SessionSummary) -> Self {
        Self {
            id,
            completed_at: summary.completed_at(),
            total: summary.total_reviews(),
            again: summary.again(),
            hard: summary.hard(),
            good: summary.good(),
            easy: summary.easy(),
        }
    }
}

/// Presentation-facing session summary facade that hides repositories and time from the UI.
///
/// This service owns:
/// - the time source (`Clock`)
/// - repository access
///
/// It does **not** own UI formatting.
#[derive(Clone)]
pub struct SessionSummaryService {
    clock: Clock,
    summaries: Arc<dyn SessionSummaryRepository>,
}

impl SessionSummaryService {
    #[must_use]
    pub fn new(clock: Clock, summaries: Arc<dyn SessionSummaryRepository>) -> Self {
        Self { clock, summaries }
    }

    #[must_use]
    pub fn in_memory(clock: Clock) -> Self {
        Self::new(
            clock,
            Arc::new(storage::repository::InMemoryRepository::new()),
        )
    }

    /// Load recent summaries for a deck.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` on repository failures.
    pub async fn list_recent_summaries(
        &self,
        deck_id: DeckId,
        days: i64,
        limit: u32,
    ) -> Result<Vec<SessionSummaryListItem>, SessionError> {
        let now = self.clock.now();
        let rows = SessionService::list_recent_summary_rows(
            deck_id,
            self.summaries.as_ref(),
            now,
            days,
            limit,
        )
        .await?;

        Ok(rows
            .iter()
            .map(|row| SessionSummaryListItem::from_summary(row.id, &row.summary))
            .collect())
    }

    /// Fetch a session summary by ID.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` when repository access fails.
    pub async fn get_summary(&self, id: SessionSummaryId) -> Result<SessionSummary, SessionError> {
        SessionService::get_summary(id, self.summaries.as_ref()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use learn_core::model::{CardId, DeckId, ReviewGrade, ReviewLog, SessionSummary};
    use learn_core::time::fixed_now;
    use storage::repository::InMemoryRepository;

    #[test]
    fn list_item_is_presentation_agnostic() {
        let now = fixed_now();
        let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];
        let summary = SessionSummary::from_logs(DeckId::new(1), now, now, &logs).unwrap();

        let item = SessionSummaryListItem::from_summary(42, &summary);

        assert_eq!(item.id, 42);
        assert_eq!(item.completed_at, now);
        assert_eq!(item.total, 1);
        assert_eq!(item.good, 1);
    }

    #[tokio::test]
    async fn list_recent_summaries_filters_by_range() {
        let repo = InMemoryRepository::new();
        let deck_id = DeckId::new(1);
        let now = fixed_now();
        let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];

        let summary_recent = SessionSummary::from_logs(
            deck_id,
            now - chrono::Duration::days(2),
            now - chrono::Duration::days(1),
            &logs,
        )
        .unwrap();
        let summary_old = SessionSummary::from_logs(
            deck_id,
            now - chrono::Duration::days(10),
            now - chrono::Duration::days(9),
            &logs,
        )
        .unwrap();

        let _id_recent = repo.append_summary(&summary_recent).await.unwrap();
        let _id_old = repo.append_summary(&summary_old).await.unwrap();

        let svc = SessionSummaryService::new(Clock::Fixed(now), Arc::new(repo));
        let items = svc.list_recent_summaries(deck_id, 7, 10).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].completed_at, summary_recent.completed_at());
        assert_eq!(items[0].total, 1);
    }
}

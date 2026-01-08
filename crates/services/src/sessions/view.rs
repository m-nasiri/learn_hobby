use chrono::{DateTime, Utc};
use std::sync::Arc;

use learn_core::model::{DeckId, SessionSummary};
use storage::repository::SessionSummaryRepository;

use crate::Clock;
use super::queries::SessionQueries;
use crate::error::SessionError;

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

/// Latest summary per deck, preserving deck identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummaryDeckItem {
    pub deck_id: DeckId,
    pub id: SessionSummaryId,
    pub completed_at: DateTime<Utc>,

    pub total: u32,
    pub again: u32,
    pub hard: u32,
    pub good: u32,
    pub easy: u32,
}

impl SessionSummaryDeckItem {
    #[must_use]
    pub fn from_row(row: &storage::repository::SessionSummaryRow) -> Self {
        let summary = &row.summary;
        Self {
            deck_id: summary.deck_id(),
            id: row.id,
            completed_at: summary.completed_at(),
            total: summary.total_reviews(),
            again: summary.again(),
            hard: summary.hard(),
            good: summary.good(),
            easy: summary.easy(),
        }
    }
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

    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
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
        let rows = SessionQueries::list_recent_summary_rows(
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

    /// Load the latest summary per deck.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` on repository failures.
    pub async fn list_latest_summaries_by_deck(
        &self,
        deck_ids: &[DeckId],
    ) -> Result<Vec<SessionSummaryDeckItem>, SessionError> {
        let rows =
            SessionQueries::list_latest_summary_rows(deck_ids, self.summaries.as_ref()).await?;
        Ok(rows.iter().map(SessionSummaryDeckItem::from_row).collect())
    }

    /// Fetch a session summary by ID.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` when repository access fails.
    pub async fn get_summary(&self, id: SessionSummaryId) -> Result<SessionSummary, SessionError> {
        SessionQueries::get_summary(id, self.summaries.as_ref()).await
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

    #[tokio::test]
    async fn list_latest_summaries_by_deck_returns_latest_for_each_deck() {
        let repo = InMemoryRepository::new();
        let now = fixed_now();
        let deck_a = DeckId::new(1);
        let deck_b = DeckId::new(2);
        let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];

        let summary_a1 = SessionSummary::from_logs(deck_a, now, now, &logs).unwrap();
        let summary_a2 =
            SessionSummary::from_logs(deck_a, now, now + chrono::Duration::days(1), &logs)
                .unwrap();
        let earlier = now - chrono::Duration::days(1);
        let summary_b = SessionSummary::from_logs(deck_b, earlier, earlier, &logs).unwrap();

        let id_a1 = repo.append_summary(&summary_a1).await.unwrap();
        let id_a2 = repo.append_summary(&summary_a2).await.unwrap();
        let id_b = repo.append_summary(&summary_b).await.unwrap();

        let svc = SessionSummaryService::new(Clock::Fixed(now), Arc::new(repo));
        let items = svc
            .list_latest_summaries_by_deck(&[deck_a, deck_b])
            .await
            .unwrap();

        let mut by_deck = std::collections::HashMap::new();
        for item in items {
            by_deck.insert(item.deck_id, item.id);
        }

        assert_eq!(by_deck.get(&deck_a), Some(&id_a2));
        assert_eq!(by_deck.get(&deck_b), Some(&id_b));
        assert_ne!(by_deck.get(&deck_a), Some(&id_a1));
    }
}

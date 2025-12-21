use chrono::{DateTime, Utc};
use learn_core::model::{DeckId, SessionSummary};
use storage::repository::{SessionSummaryRepository, SessionSummaryRow};

use crate::session_service::{SessionError, SessionService};

/// UI-facing summary view for session cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummaryCardView {
    pub id: i64,

    /// Raw timestamp used for sorting/filtering in the UI.
    pub completed_at: DateTime<Utc>,

    /// Pre-formatted timestamp string for display.
    pub completed_at_str: String,

    pub total: u32,
    pub again: u32,
    pub hard: u32,
    pub good: u32,
    pub easy: u32,
}

impl SessionSummaryCardView {
    #[must_use]
    pub fn from_row(row: &SessionSummaryRow) -> Self {
        Self::from_summary(row.id, &row.summary)
    }

    #[must_use]
    pub fn from_summary(id: i64, summary: &SessionSummary) -> Self {
        let completed_at = summary.completed_at();
        Self {
            id,
            completed_at,
            completed_at_str: format_datetime(&completed_at),
            total: summary.total_reviews(),
            again: summary.again(),
            hard: summary.hard(),
            good: summary.good(),
            easy: summary.easy(),
        }
    }
}

/// Load recent session summaries and map them to UI-ready cards.
///
/// # Errors
///
/// Returns `SessionError::Storage` on repository failures.
pub async fn list_recent_summary_cards(
    deck_id: DeckId,
    summaries: &dyn SessionSummaryRepository,
    now: DateTime<Utc>,
    days: i64,
    limit: u32,
) -> Result<Vec<SessionSummaryCardView>, SessionError> {
    let rows = SessionService::list_recent_summary_rows(deck_id, summaries, now, days, limit).await?;
    Ok(rows.iter().map(SessionSummaryCardView::from_row).collect())
}

fn format_datetime(value: &DateTime<Utc>) -> String {
    value.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::model::{DeckId, ReviewGrade, ReviewLog, SessionSummary};
    use learn_core::time::fixed_now;
    use storage::repository::{InMemoryRepository, SessionSummaryRepository, SessionSummaryRow};

    #[test]
    fn view_formats_completed_at() {
        let now = fixed_now();
        let logs = vec![ReviewLog::new(
            learn_core::model::CardId::new(1),
            ReviewGrade::Good,
            now,
        )];
        let summary = SessionSummary::from_logs(DeckId::new(1), now, now, &logs).unwrap();
        let row = SessionSummaryRow::new(42, summary);
        let view = SessionSummaryCardView::from_row(&row);

        assert_eq!(view.id, 42);
        assert_eq!(view.completed_at, now);
        assert_eq!(view.completed_at_str, now.to_rfc3339());
        assert_eq!(view.total, 1);
        assert_eq!(view.good, 1);
    }

    #[tokio::test]
    async fn list_recent_summary_cards_maps_rows() {
        let repo = InMemoryRepository::new();
        let deck_id = DeckId::new(1);
        let now = fixed_now();
        let logs = vec![ReviewLog::new(
            learn_core::model::CardId::new(1),
            ReviewGrade::Good,
            now,
        )];

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

        let views = list_recent_summary_cards(deck_id, &repo, now, 7, 10)
            .await
            .unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].completed_at, summary_recent.completed_at());
        assert_eq!(views[0].total, 1);
    }
}

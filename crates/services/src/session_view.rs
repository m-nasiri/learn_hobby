use chrono::{DateTime, Utc};
use learn_core::model::SessionSummary;
use storage::repository::SessionSummaryRow;

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

fn format_datetime(value: &DateTime<Utc>) -> String {
    value.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::model::{DeckId, ReviewGrade, ReviewLog, SessionSummary};
    use learn_core::time::fixed_now;
    use storage::repository::SessionSummaryRow;

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
}

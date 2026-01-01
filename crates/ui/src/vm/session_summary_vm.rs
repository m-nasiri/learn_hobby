use services::SessionSummaryListItem;

use crate::vm::time_fmt::format_datetime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSummaryCardVm {
    pub id: i64,
    pub completed_at_str: String,

    pub total: u32,
    pub again: u32,
    pub hard: u32,
    pub good: u32,
    pub easy: u32,

    pub cards_label: String,
    pub again_pct: u32,
    pub hard_pct: u32,
    pub good_pct: u32,
    pub easy_pct: u32,
}

impl From<&SessionSummaryListItem> for SessionSummaryCardVm {
    fn from(item: &SessionSummaryListItem) -> Self {
        let total = item.total;
        let pct = |count: u32| if total == 0 { 0 } else { (count.saturating_mul(100)) / total };
        Self {
            id: item.id,
            completed_at_str: format_datetime(&item.completed_at),
            total: item.total,
            again: item.again,
            hard: item.hard,
            good: item.good,
            easy: item.easy,
            cards_label: format!("{total} Cards"),
            again_pct: pct(item.again),
            hard_pct: pct(item.hard),
            good_pct: pct(item.good),
            easy_pct: pct(item.easy),
        }
    }
}

#[must_use]
pub fn map_session_summary_cards(items: &[SessionSummaryListItem]) -> Vec<SessionSummaryCardVm> {
    items.iter().map(SessionSummaryCardVm::from).collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSummaryDetailVm {
    pub started_at_str: String,
    pub completed_at_str: String,
    pub total: u32,
    pub again: u32,
    pub hard: u32,
    pub good: u32,
    pub easy: u32,
}

#[must_use]
pub fn map_session_summary_detail(
    summary: &learn_core::model::SessionSummary,
) -> SessionSummaryDetailVm {
    SessionSummaryDetailVm {
        started_at_str: format_datetime(&summary.started_at()),
        completed_at_str: format_datetime(&summary.completed_at()),
        total: summary.total_reviews(),
        again: summary.again(),
        hard: summary.hard(),
        good: summary.good(),
        easy: summary.easy(),
    }
}

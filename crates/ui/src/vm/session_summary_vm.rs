use services::SessionSummaryListItem;
use services::session_view::SessionSummaryId;

use crate::vm::time_fmt::format_datetime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSummaryCardVm {
    pub id: SessionSummaryId,
    pub completed_at_str: String,

    pub total: u32,
    pub again: u32,
    pub hard: u32,
    pub good: u32,
    pub easy: u32,
}

impl From<&SessionSummaryListItem> for SessionSummaryCardVm {
    fn from(item: &SessionSummaryListItem) -> Self {
        Self {
            id: item.id,
            completed_at_str: format_datetime(item.completed_at),
            total: item.total,
            again: item.again,
            hard: item.hard,
            good: item.good,
            easy: item.easy,
        }
    }
}

#[must_use]
pub fn map_session_summary_cards(items: &[SessionSummaryListItem]) -> Vec<SessionSummaryCardVm> {
    items.iter().map(SessionSummaryCardVm::from).collect()
}

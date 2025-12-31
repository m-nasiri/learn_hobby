use chrono::Duration;
use learn_core::model::{CardId, DeckId, ReviewGrade, ReviewLog, SessionSummary};
use learn_core::time::fixed_now;
use storage::repository::{InMemoryRepository, SessionSummaryRepository, Storage, StorageError};

use super::test_harness::{ViewKind, setup_view_harness, setup_view_harness_with_summary_repo};

#[tokio::test(flavor = "current_thread")]
async fn home_view_smoke_renders_recent_count() {
    let mut harness = setup_view_harness(ViewKind::Home, "Default").await;
    let deck_id = harness.deck_id;
    let now = fixed_now();
    let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];
    let summary = SessionSummary::from_logs(deck_id, now, now, &logs).unwrap();
    let summary_old = SessionSummary::from_logs(
        deck_id,
        now - Duration::days(2),
        now - Duration::days(1),
        &logs,
    )
    .unwrap();

    harness
        .storage
        .session_summaries
        .append_summary(&summary)
        .await
        .expect("append summary");
    harness
        .storage
        .session_summaries
        .append_summary(&summary_old)
        .await
        .expect("append summary");

    harness.rebuild();
    let html = harness.render();
    let expected = format!("Recent sessions (7d): 2");
    assert!(html.contains(&expected), "missing {expected} in {html}");
    let deck_label = format!("Current deck: {deck_id:?}");
    assert!(html.contains(&deck_label), "missing {deck_label} in {html}");
}

#[tokio::test(flavor = "current_thread")]
async fn history_view_smoke_renders_summary_card() {
    let mut harness = setup_view_harness(ViewKind::History, "Default").await;
    let deck_id = harness.deck_id;
    let now = fixed_now();
    let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];
    let summary = SessionSummary::from_logs(deck_id, now, now, &logs).unwrap();

    harness
        .storage
        .session_summaries
        .append_summary(&summary)
        .await
        .expect("append summary");

    harness.rebuild();
    let html = harness.render();
    assert!(html.contains("Total:"), "missing summary text in {html}");
    assert!(html.contains("View"), "missing view link in {html}");
}

#[tokio::test(flavor = "current_thread")]
async fn summary_view_smoke_renders_details() {
    let repo = InMemoryRepository::new();
    let now = fixed_now();
    let deck_id = DeckId::new(1);
    let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];
    let summary = SessionSummary::from_logs(deck_id, now, now, &logs).unwrap();
    let summary_id = repo.append_summary(&summary).await.expect("append summary");

    let mut harness = setup_view_harness_with_summary_repo(
        ViewKind::Summary(summary_id),
        "Default",
        Storage::in_memory(),
        std::sync::Arc::new(repo),
    )
    .await;
    harness.rebuild();
    let html = harness.render();
    assert!(html.contains("Session Summary"), "missing title in {html}");
    assert!(html.contains("Total"), "missing total in {html}");
    assert!(html.contains("Good"), "missing good in {html}");
}

struct FailingSummaryRepo;

#[async_trait::async_trait]
impl SessionSummaryRepository for FailingSummaryRepo {
    async fn append_summary(&self, _summary: &SessionSummary) -> Result<i64, StorageError> {
        Err(StorageError::Connection("fail".to_string()))
    }

    async fn get_summary(&self, _id: i64) -> Result<SessionSummary, StorageError> {
        Err(StorageError::Connection("fail".to_string()))
    }

    async fn list_summaries(
        &self,
        _deck_id: DeckId,
        _completed_from: Option<chrono::DateTime<chrono::Utc>>,
        _completed_until: Option<chrono::DateTime<chrono::Utc>>,
        _limit: u32,
    ) -> Result<Vec<SessionSummary>, StorageError> {
        Err(StorageError::Connection("fail".to_string()))
    }

    async fn list_summary_rows(
        &self,
        _deck_id: DeckId,
        _completed_from: Option<chrono::DateTime<chrono::Utc>>,
        _completed_until: Option<chrono::DateTime<chrono::Utc>>,
        _limit: u32,
    ) -> Result<Vec<storage::repository::SessionSummaryRow>, StorageError> {
        Err(StorageError::Connection("fail".to_string()))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn home_view_smoke_renders_error_state() {
    let repo = std::sync::Arc::new(FailingSummaryRepo);
    let mut harness = setup_view_harness_with_summary_repo(
        ViewKind::Home,
        "Default",
        Storage::in_memory(),
        repo,
    )
    .await;
    harness.rebuild();
    let html = harness.render();
    assert!(html.contains("Something went wrong"), "missing error in {html}");
    assert!(html.contains("Retry"), "missing retry in {html}");
}

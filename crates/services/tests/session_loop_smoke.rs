use learn_core::model::{
    Card, CardId, ContentDraft, Deck, DeckId, DeckSettings, ReviewGrade, SessionSummary,
};
use learn_core::time::fixed_now;
use services::{Clock, SessionLoopService};
use storage::repository::{
    CardRepository, DeckRepository, InMemoryRepository, SessionSummaryRepository, SessionSummaryRow,
    StorageError,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

struct FlakySummaryRepo {
    inner: InMemoryRepository,
    fail_next: AtomicBool,
}

impl FlakySummaryRepo {
    fn new(inner: InMemoryRepository) -> Self {
        Self {
            inner,
            fail_next: AtomicBool::new(true),
        }
    }
}

#[async_trait]
impl SessionSummaryRepository for FlakySummaryRepo {
    async fn append_summary(&self, summary: &SessionSummary) -> Result<i64, StorageError> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(StorageError::Connection("forced failure".to_string()));
        }
        self.inner.append_summary(summary).await
    }

    async fn get_summary(&self, id: i64) -> Result<SessionSummary, StorageError> {
        self.inner.get_summary(id).await
    }

    async fn list_summaries(
        &self,
        deck_id: DeckId,
        completed_from: Option<chrono::DateTime<chrono::Utc>>,
        completed_until: Option<chrono::DateTime<chrono::Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummary>, StorageError> {
        self.inner
            .list_summaries(deck_id, completed_from, completed_until, limit)
            .await
    }

    async fn list_summary_rows(
        &self,
        deck_id: DeckId,
        completed_from: Option<chrono::DateTime<chrono::Utc>>,
        completed_until: Option<chrono::DateTime<chrono::Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummaryRow>, StorageError> {
        self.inner
            .list_summary_rows(deck_id, completed_from, completed_until, limit)
            .await
    }

    async fn list_latest_summary_rows(
        &self,
        deck_ids: &[DeckId],
    ) -> Result<Vec<SessionSummaryRow>, StorageError> {
        self.inner.list_latest_summary_rows(deck_ids).await
    }
}

#[tokio::test]
async fn session_loop_persists_summary() {
    let repo = InMemoryRepository::new();
    let deck_id = DeckId::new(1);
    let now = fixed_now();

    let deck = Deck::new(
        deck_id,
        "Smoke Deck",
        None,
        DeckSettings::default_for_adhd(),
        now,
    )
    .unwrap();
    repo.upsert_deck(&deck).await.unwrap();

    for id in 1..=3 {
        let prompt = ContentDraft::text_only(format!("Q{id}"))
            .validate(now, None, None)
            .unwrap();
        let answer = ContentDraft::text_only(format!("A{id}"))
            .validate(now, None, None)
            .unwrap();
        let card = Card::new(CardId::new(id), deck_id, prompt, answer, now, now).unwrap();
        repo.upsert_card(&card).await.unwrap();
    }

    let loop_svc = SessionLoopService::new(
        Clock::fixed(now),
        Arc::new(repo.clone()),
        Arc::new(repo.clone()),
        Arc::new(repo.clone()),
        Arc::new(repo.clone()),
    );

    let mut session = loop_svc.start_session(deck_id).await.unwrap();
    while !session.is_complete() {
        let _ = loop_svc
            .answer_current(&mut session, ReviewGrade::Good)
            .await
            .unwrap();
    }

    let summary_id = session.summary_id().expect("summary persisted");
    let summary = repo.get_summary(summary_id).await.unwrap();
    assert_eq!(summary.total_reviews(), 3);
}

#[tokio::test]
async fn session_loop_retry_persists_summary_after_failure() {
    let repo = InMemoryRepository::new();
    let summaries_repo = Arc::new(FlakySummaryRepo::new(InMemoryRepository::new()));
    let summaries: Arc<dyn SessionSummaryRepository + Send + Sync> = summaries_repo.clone();
    let deck_id = DeckId::new(1);
    let now = fixed_now();

    let deck = Deck::new(
        deck_id,
        "Retry Deck",
        None,
        DeckSettings::default_for_adhd(),
        now,
    )
    .unwrap();
    repo.upsert_deck(&deck).await.unwrap();

    let prompt = ContentDraft::text_only("Q1")
        .validate(now, None, None)
        .unwrap();
    let answer = ContentDraft::text_only("A1")
        .validate(now, None, None)
        .unwrap();
    let card = Card::new(CardId::new(1), deck_id, prompt, answer, now, now).unwrap();
    repo.upsert_card(&card).await.unwrap();

    let loop_svc = SessionLoopService::new(
        Clock::fixed(now),
        Arc::new(repo.clone()),
        Arc::new(repo.clone()),
        Arc::new(repo.clone()),
        summaries,
    );

    let mut session = loop_svc.start_session(deck_id).await.unwrap();
    let err = loop_svc
        .answer_current(&mut session, ReviewGrade::Good)
        .await
        .unwrap_err();
    assert!(matches!(err, services::SessionError::Storage(_)));
    assert!(session.is_complete());
    assert!(session.summary_id().is_none());

    let summary_id = loop_svc.finalize_summary(&mut session).await.unwrap();
    let summary = summaries_repo.get_summary(summary_id).await.unwrap();
    assert_eq!(summary.total_reviews(), 1);
}

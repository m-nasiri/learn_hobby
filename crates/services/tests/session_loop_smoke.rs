use learn_core::model::{Card, CardId, ContentDraft, Deck, DeckId, DeckSettings, ReviewGrade};
use learn_core::time::fixed_now;
use services::{Clock, SessionLoopService};
use storage::repository::{
    CardRepository, DeckRepository, InMemoryRepository, SessionSummaryRepository,
};

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
        std::sync::Arc::new(repo.clone()),
        std::sync::Arc::new(repo.clone()),
        std::sync::Arc::new(repo.clone()),
        std::sync::Arc::new(repo.clone()),
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

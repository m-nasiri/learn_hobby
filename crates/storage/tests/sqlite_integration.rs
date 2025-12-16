use chrono::Duration;
use learn_core::model::Card;
use learn_core::model::content::ContentDraft;
use learn_core::model::{CardId, CardPhase, DeckId, DeckSettings, MediaId, ReviewGrade};
use learn_core::time::fixed_now;
use storage::repository::{CardRepository, DeckRepository, ReviewLogRecord, ReviewLogRepository};
use storage::sqlite::SqliteRepository;

fn build_card(id: u64, deck_id: DeckId) -> Card {
    let prompt = ContentDraft::text_only("Q")
        .validate(fixed_now(), None, None)
        .unwrap();
    let answer = ContentDraft::text_only("A")
        .validate(fixed_now(), None, None)
        .unwrap();
    let now = fixed_now();
    Card::new(CardId::new(id), deck_id, prompt, answer, now, now).unwrap()
}

#[tokio::test]
async fn sqlite_roundtrip_persists_phase_and_reviews() {
    let repo = SqliteRepository::connect("sqlite:file:memdb_roundtrip?mode=memory&cache=shared")
        .await
        .expect("connect");
    repo.migrate().await.expect("migrate");

    let deck = learn_core::model::Deck::new(
        DeckId::new(1),
        "Test",
        None,
        DeckSettings::default_for_adhd(),
        fixed_now(),
    )
    .unwrap();
    repo.upsert_deck(&deck).await.unwrap();

    let mut card = build_card(1, deck.id());
    let outcome =
        learn_core::model::ReviewOutcome::new(fixed_now() + Duration::days(1), 1.0, 2.0, 0.0, 1.0);
    card.apply_review_with_phase(ReviewGrade::Good, &outcome, fixed_now());
    repo.upsert_card(&card).await.unwrap();

    let fetched = repo
        .get_cards(deck.id(), &[card.id()])
        .await
        .expect("fetch");
    assert_eq!(fetched.len(), 1);
    let fetched_card = &fetched[0];
    assert_eq!(fetched_card.phase(), CardPhase::Learning);
    assert_eq!(fetched_card.review_count(), 1);
}

#[tokio::test]
async fn sqlite_supports_due_new_and_logs() {
    let repo = SqliteRepository::connect("sqlite:file:memdb_due_new?mode=memory&cache=shared")
        .await
        .expect("connect");
    repo.migrate().await.expect("migrate");

    let deck = learn_core::model::Deck::new(
        DeckId::new(1),
        "Test",
        None,
        DeckSettings::default_for_adhd(),
        fixed_now(),
    )
    .unwrap();
    repo.upsert_deck(&deck).await.unwrap();

    let now = fixed_now();
    let prompt = learn_core::model::content::Content::from_persisted(
        "Q with media".to_string(),
        Some(MediaId::new(42)),
    )
    .unwrap();
    let answer =
        learn_core::model::content::Content::from_persisted("A with media".to_string(), None)
            .unwrap();
    let mut card = Card::from_persisted(
        CardId::new(1),
        deck.id(),
        prompt,
        answer,
        now,
        now,
        None,
        CardPhase::New,
        0,
        0.0,
        0.0,
    )
    .unwrap();
    repo.upsert_card(&card).await.unwrap();

    let new_cards = repo.new_cards(deck.id(), 10).await.unwrap();
    assert_eq!(new_cards.len(), 1);
    assert_eq!(new_cards[0].prompt().media_id(), Some(MediaId::new(42)));
    assert_eq!(new_cards[0].answer().media_id(), None);

    let later = now + Duration::minutes(1);
    let prompt2 =
        learn_core::model::content::Content::from_persisted("Q2".to_string(), None).unwrap();
    let answer2 =
        learn_core::model::content::Content::from_persisted("A2".to_string(), None).unwrap();
    let card2 = Card::from_persisted(
        CardId::new(2),
        deck.id(),
        prompt2,
        answer2,
        later,
        later,
        None,
        CardPhase::New,
        0,
        0.0,
        0.0,
    )
    .unwrap();
    repo.upsert_card(&card2).await.unwrap();

    let new_cards = repo.new_cards(deck.id(), 10).await.unwrap();
    assert_eq!(new_cards.len(), 2);
    assert_eq!(new_cards[0].id(), CardId::new(1));
    assert_eq!(new_cards[1].id(), CardId::new(2));

    let reviewed_at = now;
    // Force the card to be due immediately by scheduling the next review in the past.
    let outcome =
        learn_core::model::ReviewOutcome::new(now - Duration::hours(1), 1.0, 2.0, 0.5, 0.5);
    card.apply_review_with_phase(ReviewGrade::Good, &outcome, reviewed_at);
    repo.upsert_card(&card).await.unwrap();

    let due_cards = repo.due_cards(deck.id(), now, 10).await.unwrap();
    assert_eq!(due_cards.len(), 1);

    let card3 = Card::from_persisted(
        CardId::new(3),
        deck.id(),
        learn_core::model::content::Content::from_persisted("Q3".to_string(), None).unwrap(),
        learn_core::model::content::Content::from_persisted("A3".to_string(), None).unwrap(),
        now,
        now - Duration::hours(2),
        Some(reviewed_at),
        CardPhase::Reviewing,
        1,
        3.0,
        4.0,
    )
    .unwrap();
    repo.upsert_card(&card3).await.unwrap();

    let due_cards = repo.due_cards(deck.id(), now, 10).await.unwrap();
    assert_eq!(due_cards.len(), 2);
    assert_eq!(due_cards[0].id(), CardId::new(3));
    assert_eq!(due_cards[1].id(), CardId::new(1));

    let log = learn_core::model::ReviewLog::new(card.id(), ReviewGrade::Good, reviewed_at);
    let record = ReviewLogRecord::from_applied(deck.id(), &log, &outcome);
    let id = repo.append_log(record).await.unwrap();
    let logs = repo
        .logs_for_card(deck.id(), card.id())
        .await
        .expect("logs");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].id, Some(id));
    assert_eq!(logs[0].grade, ReviewGrade::Good);
    assert_eq!(logs[0].next_review_at, outcome.next_review);
}

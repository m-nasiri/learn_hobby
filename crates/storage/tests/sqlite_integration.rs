use chrono::Duration;
use learn_core::model::Card;
use learn_core::model::content::ContentDraft;
use learn_core::model::{CardId, CardPhase, DeckId, DeckSettings, ReviewGrade};
use learn_core::time::fixed_now;
use storage::repository::{CardRepository, DeckRepository};
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
    let repo = SqliteRepository::connect("sqlite::memory:?cache=shared")
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

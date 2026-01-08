use chrono::Duration;
use learn_core::model::Card;
use learn_core::model::content::ContentDraft;
use learn_core::model::{
    CardId, CardPhase, DeckId, DeckSettings, MediaId, ReviewGrade, ReviewLog, SessionSummary,
    TagName,
};
use learn_core::time::fixed_now;
use storage::repository::{
    CardRepository, DeckPracticeCounts, DeckRepository, ReviewLogRecord, ReviewLogRepository,
    SessionSummaryRepository,
};
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

#[tokio::test]
async fn sqlite_persists_session_summary() {
    let repo =
        SqliteRepository::connect("sqlite:file:memdb_session_summary?mode=memory&cache=shared")
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
    let logs = vec![
        ReviewLog::new(CardId::new(1), ReviewGrade::Good, now),
        ReviewLog::new(CardId::new(2), ReviewGrade::Again, now),
        ReviewLog::new(CardId::new(3), ReviewGrade::Hard, now),
    ];

    let summary = SessionSummary::from_logs(deck.id(), now, now, &logs).unwrap();
    let id = repo.append_summary(&summary).await.unwrap();

    let stored = repo.get_summary(id).await.unwrap();
    assert_eq!(stored.deck_id(), deck.id());
    assert_eq!(stored.total_reviews(), 3);
    assert_eq!(stored.good(), 1);
    assert_eq!(stored.again(), 1);
    assert_eq!(stored.hard(), 1);
}

#[tokio::test]
async fn sqlite_counts_practice_stats() {
    let repo = SqliteRepository::connect("sqlite:file:memdb_counts?mode=memory&cache=shared")
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
    let card1 = build_card(1, deck.id());
    repo.upsert_card(&card1).await.unwrap();

    let mut card2 = build_card(2, deck.id());
    let outcome_due =
        learn_core::model::ReviewOutcome::new(now - Duration::hours(1), 1.0, 2.0, 0.0, 1.0);
    card2.apply_review_with_phase(ReviewGrade::Good, &outcome_due, now);
    repo.upsert_card(&card2).await.unwrap();

    let mut card3 = build_card(3, deck.id());
    let outcome_future =
        learn_core::model::ReviewOutcome::new(now + Duration::hours(4), 1.0, 2.0, 0.0, 1.0);
    card3.apply_review_with_phase(ReviewGrade::Good, &outcome_future, now);
    repo.upsert_card(&card3).await.unwrap();

    let tag_language = TagName::new("Language").unwrap();
    let tag_grammar = TagName::new("Grammar").unwrap();
    repo.set_tags_for_card(deck.id(), card1.id(), &[tag_language.clone()])
        .await
        .unwrap();
    repo.set_tags_for_card(deck.id(), card2.id(), &[tag_language.clone(), tag_grammar.clone()])
        .await
        .unwrap();
    repo.set_tags_for_card(deck.id(), card3.id(), &[tag_grammar.clone()])
        .await
        .unwrap();

    let counts = repo.deck_practice_counts(deck.id(), now).await.unwrap();
    assert_eq!(
        counts,
        DeckPracticeCounts {
            total: 3,
            due: 1,
            new: 1,
        }
    );

    let tag_counts = repo.list_tag_practice_counts(deck.id(), now).await.unwrap();
    let language = tag_counts
        .iter()
        .find(|item| item.name == tag_language)
        .unwrap();
    assert_eq!(language.total, 2);
    assert_eq!(language.new, 1);
    assert_eq!(language.due, 1);

    let grammar = tag_counts
        .iter()
        .find(|item| item.name == tag_grammar)
        .unwrap();
    assert_eq!(grammar.total, 2);
    assert_eq!(grammar.new, 0);
    assert_eq!(grammar.due, 1);
}

#[tokio::test]
async fn sqlite_lists_deck_practice_counts() {
    let repo = SqliteRepository::connect("sqlite:file:memdb_counts_multi?mode=memory&cache=shared")
        .await
        .expect("connect");
    repo.migrate().await.expect("migrate");

    let deck1 = learn_core::model::Deck::new(
        DeckId::new(1),
        "Deck 1",
        None,
        DeckSettings::default_for_adhd(),
        fixed_now(),
    )
    .unwrap();
    let deck2 = learn_core::model::Deck::new(
        DeckId::new(2),
        "Deck 2",
        None,
        DeckSettings::default_for_adhd(),
        fixed_now(),
    )
    .unwrap();
    repo.upsert_deck(&deck1).await.unwrap();
    repo.upsert_deck(&deck2).await.unwrap();

    let now = fixed_now();
    let card1 = build_card(1, deck1.id());
    repo.upsert_card(&card1).await.unwrap();
    let mut card2 = build_card(2, deck1.id());
    let outcome_due =
        learn_core::model::ReviewOutcome::new(now - Duration::hours(1), 1.0, 2.0, 0.0, 1.0);
    card2.apply_review_with_phase(ReviewGrade::Good, &outcome_due, now);
    repo.upsert_card(&card2).await.unwrap();

    let card3 = build_card(3, deck2.id());
    repo.upsert_card(&card3).await.unwrap();
    let mut card4 = build_card(4, deck2.id());
    let outcome_future =
        learn_core::model::ReviewOutcome::new(now + Duration::hours(2), 1.0, 2.0, 0.0, 1.0);
    card4.apply_review_with_phase(ReviewGrade::Good, &outcome_future, now);
    repo.upsert_card(&card4).await.unwrap();

    let rows = repo
        .list_deck_practice_counts(&[deck1.id(), deck2.id()], now)
        .await
        .unwrap();
    let mut by_deck = std::collections::HashMap::new();
    for row in rows {
        by_deck.insert(row.deck_id, row.counts);
    }

    assert_eq!(
        by_deck.get(&deck1.id()),
        Some(&DeckPracticeCounts {
            total: 2,
            due: 1,
            new: 1,
        })
    );
    assert_eq!(
        by_deck.get(&deck2.id()),
        Some(&DeckPracticeCounts {
            total: 2,
            due: 0,
            new: 1,
        })
    );
}

#[tokio::test]
async fn sqlite_lists_session_summaries_by_range() {
    let repo = SqliteRepository::connect("sqlite:file:memdb_session_list?mode=memory&cache=shared")
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
    let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];

    let summary1 = SessionSummary::from_logs(deck.id(), now, now, &logs).unwrap();
    let summary2 =
        SessionSummary::from_logs(deck.id(), now, now + Duration::days(1), &logs).unwrap();
    let summary3 =
        SessionSummary::from_logs(deck.id(), now, now + Duration::days(2), &logs).unwrap();

    let id1 = repo.append_summary(&summary1).await.unwrap();
    let id2 = repo.append_summary(&summary2).await.unwrap();
    let id3 = repo.append_summary(&summary3).await.unwrap();

    // Same completed_at as summary3 to validate the `id DESC` tie-breaker.
    let summary4 =
        SessionSummary::from_logs(deck.id(), now, now + Duration::days(2), &logs).unwrap();
    let id4 = repo.append_summary(&summary4).await.unwrap();

    let listed = repo
        .list_summaries(
            deck.id(),
            Some(now + Duration::days(1)),
            Some(now + Duration::days(2)),
            10,
        )
        .await
        .unwrap();

    assert_eq!(listed.len(), 3);
    assert!(listed[0].completed_at() >= listed[1].completed_at());
    assert!(listed[1].completed_at() >= listed[2].completed_at());

    assert_eq!(listed[0].completed_at(), now + Duration::days(2));
    assert_eq!(listed[1].completed_at(), now + Duration::days(2));
    assert_eq!(listed[2].completed_at(), now + Duration::days(1));

    let rows = repo
        .list_summary_rows(
            deck.id(),
            Some(now + Duration::days(1)),
            Some(now + Duration::days(2)),
            10,
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 3);

    // Two summaries have the same completed_at (day 2); id DESC should put the later insert first.
    assert_eq!(rows[0].summary.completed_at(), now + Duration::days(2));
    assert_eq!(rows[1].summary.completed_at(), now + Duration::days(2));
    assert_eq!(rows[2].summary.completed_at(), now + Duration::days(1));

    assert_eq!(rows[0].id, id4);
    assert_eq!(rows[1].id, id3);
    assert_eq!(rows[2].id, id2);

    // (Optional sanity) ensure the out-of-range summary1 is not included.
    assert_ne!(rows[2].id, id1);
}

#[tokio::test]
async fn sqlite_lists_latest_summary_rows() {
    let repo =
        SqliteRepository::connect("sqlite:file:memdb_latest_summary?mode=memory&cache=shared")
            .await
            .expect("connect");
    repo.migrate().await.expect("migrate");

    let deck1 = learn_core::model::Deck::new(
        DeckId::new(1),
        "Deck 1",
        None,
        DeckSettings::default_for_adhd(),
        fixed_now(),
    )
    .unwrap();
    let deck2 = learn_core::model::Deck::new(
        DeckId::new(2),
        "Deck 2",
        None,
        DeckSettings::default_for_adhd(),
        fixed_now(),
    )
    .unwrap();
    repo.upsert_deck(&deck1).await.unwrap();
    repo.upsert_deck(&deck2).await.unwrap();

    let now = fixed_now();
    let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];
    let summary1 = SessionSummary::from_logs(deck1.id(), now, now, &logs).unwrap();
    let summary2 =
        SessionSummary::from_logs(deck1.id(), now, now + Duration::days(1), &logs).unwrap();
    let earlier = now - Duration::days(1);
    let summary3 = SessionSummary::from_logs(deck2.id(), earlier, earlier, &logs).unwrap();

    let id1 = repo.append_summary(&summary1).await.unwrap();
    let id2 = repo.append_summary(&summary2).await.unwrap();
    let id3 = repo.append_summary(&summary3).await.unwrap();

    let rows = repo
        .list_latest_summary_rows(&[deck1.id(), deck2.id()])
        .await
        .unwrap();
    let mut by_deck = std::collections::HashMap::new();
    for row in rows {
        by_deck.insert(row.summary.deck_id(), row.id);
    }

    assert_eq!(by_deck.get(&deck1.id()), Some(&id2));
    assert_eq!(by_deck.get(&deck2.id()), Some(&id3));
    assert_ne!(by_deck.get(&deck1.id()), Some(&id1));
}

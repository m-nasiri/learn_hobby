use dioxus::prelude::{ReadableExt, WritableExt};
use learn_core::model::DeckSettings;

use crate::vm::build_card_list_item;

use super::actions::EditorIntent;
use super::state::SaveRequest;
use super::test_harness::{set_fields, setup_editor_harness};

#[tokio::test(flavor = "current_thread")]
async fn editor_intents_smoke_create_edit_delete_undo() {
    let (mut harness, _deck_service, card_service, deck_id) =
        setup_editor_harness("Default").await;
    let dispatch = harness.dispatch();
    let state = harness.state();

    dispatch.call(EditorIntent::RequestNewCard);
    harness.drive();
    set_fields(&state, "What is Rust?", "A systems language.");
    dispatch.call(EditorIntent::Save(SaveRequest::new(false)));
    harness.drive();

    let cards = card_service.list_cards(deck_id, 10).await.expect("list cards");
    assert_eq!(cards.len(), 1);
    let created = cards[0].clone();

    let list_item = build_card_list_item(
        created.id(),
        created.prompt().text(),
        created.answer().text(),
    );
    dispatch.call(EditorIntent::RequestSelectCard(list_item));
    harness.drive();
    set_fields(
        &state,
        "What is Rust language?",
        "A systems programming language.",
    );
    dispatch.call(EditorIntent::Save(SaveRequest::new(false)));
    harness.drive();

    let cards = card_service.list_cards(deck_id, 10).await.expect("list edited");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].prompt().text(), "What is Rust language?");
    assert_eq!(cards[0].answer().text(), "A systems programming language.");

    dispatch.call(EditorIntent::Delete);
    harness.drive();
    let cards = card_service.list_cards(deck_id, 10).await.expect("list deleted");
    assert!(cards.is_empty());

    // Undo deletion by recreating the card content.
    dispatch.call(EditorIntent::RequestNewCard);
    harness.drive();
    set_fields(&state, "What is Rust?", "A systems language.");
    dispatch.call(EditorIntent::Save(SaveRequest::new(false)));
    harness.drive();

    let cards = card_service.list_cards(deck_id, 10).await.expect("list restored");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].prompt().text(), "What is Rust?");
    assert_eq!(cards[0].answer().text(), "A systems language.");
}

#[tokio::test(flavor = "current_thread")]
async fn editor_intents_smoke_duplicate_prompt_confirm() {
    let (mut harness, _deck_service, card_service, deck_id) =
        setup_editor_harness("Default").await;
    let dispatch = harness.dispatch();
    let state = harness.state();

    dispatch.call(EditorIntent::RequestNewCard);
    harness.drive();
    set_fields(&state, "What is Rust?", "A systems language.");
    dispatch.call(EditorIntent::Save(SaveRequest::new(false)));
    harness.drive();

    let cards = card_service.list_cards(deck_id, 10).await.expect("list cards");
    assert_eq!(cards.len(), 1);

    dispatch.call(EditorIntent::RequestNewCard);
    harness.drive();
    set_fields(&state, "What is Rust?", "Another answer.");
    dispatch.call(EditorIntent::Save(SaveRequest::new(false)));
    harness.drive();

    assert!((state.show_duplicate_modal)());
    let cards = card_service.list_cards(deck_id, 10).await.expect("after duplicate");
    assert_eq!(cards.len(), 1);

    dispatch.call(EditorIntent::ConfirmDuplicate);
    harness.drive();
    let cards = card_service.list_cards(deck_id, 10).await.expect("after confirm");
    assert_eq!(cards.len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn editor_intents_smoke_rename_and_switch_deck() {
    let (mut harness, deck_service, _card_service, deck_id) =
        setup_editor_harness("Default").await;
    let dispatch = harness.dispatch();
    let state = harness.state();

    let second_id = deck_service
        .create_deck(
            "Second".to_string(),
            None,
            DeckSettings::default_for_adhd(),
        )
        .await
        .expect("create second deck");

    dispatch.call(EditorIntent::RequestSelectDeck(second_id));
    harness.drive();
    assert_eq!(*state.selected_deck.read(), second_id);

    dispatch.call(EditorIntent::BeginRename("Second".to_string()));
    harness.drive();
    let mut rename_deck_name = state.rename_deck_name;
    rename_deck_name.set("Renamed".to_string());
    dispatch.call(EditorIntent::CommitRename);
    harness.drive();

    let decks = deck_service.list_decks(10).await.expect("list decks");
    let renamed = decks
        .iter()
        .find(|deck| deck.id() == second_id)
        .expect("renamed deck");
    assert_eq!(renamed.name(), "Renamed");

    dispatch.call(EditorIntent::RequestSelectDeck(deck_id));
    harness.drive();
    assert_eq!(*state.selected_deck.read(), deck_id);
}

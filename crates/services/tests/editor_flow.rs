use std::sync::Arc;

use learn_core::model::content::ContentDraft;
use learn_core::model::DeckSettings;
use learn_core::time::fixed_now;
use services::{CardService, Clock, DeckService};
use storage::repository::Storage;

#[tokio::test]
async fn editor_flow_create_edit_delete_undo() {
    let storage = Storage::sqlite("sqlite:file:memdb_editor_flow?mode=memory&cache=shared")
        .await
        .expect("connect sqlite");
    let clock = Clock::fixed(fixed_now());
    let deck_service = DeckService::new(clock, Arc::clone(&storage.decks));
    let card_service = CardService::new(clock, Arc::clone(&storage.cards));

    let deck_id = deck_service
        .create_deck(
            "Default".to_string(),
            None,
            DeckSettings::default_for_adhd(),
        )
        .await
        .expect("create deck");

    let original_prompt = ContentDraft::text_only("What is Rust?");
    let original_answer = ContentDraft::text_only("A systems language.");
    let card_id = card_service
        .create_card(deck_id, original_prompt.clone(), original_answer.clone())
        .await
        .expect("create card");

    card_service
        .update_card_content(
            deck_id,
            card_id,
            ContentDraft::text_only("What is Rust language?"),
            ContentDraft::text_only("A systems programming language."),
        )
        .await
        .expect("update card");

    let cards = card_service
        .list_cards(deck_id, 10)
        .await
        .expect("list cards");
    assert_eq!(cards.len(), 1);
    let updated = &cards[0];
    assert_eq!(updated.id(), card_id);
    assert_eq!(updated.prompt().text(), "What is Rust language?");
    assert_eq!(updated.answer().text(), "A systems programming language.");

    card_service
        .delete_card(deck_id, card_id)
        .await
        .expect("delete card");
    let cards = card_service
        .list_cards(deck_id, 10)
        .await
        .expect("list after delete");
    assert!(cards.is_empty());

    // Undo deletion by re-creating the card content.
    let restored_id = card_service
        .create_card(deck_id, original_prompt, original_answer)
        .await
        .expect("undo delete");
    let cards = card_service
        .list_cards(deck_id, 10)
        .await
        .expect("list after undo");
    assert_eq!(cards.len(), 1);
    let restored = &cards[0];
    assert_eq!(restored.id(), restored_id);
    assert_eq!(restored.prompt().text(), "What is Rust?");
    assert_eq!(restored.answer().text(), "A systems language.");
}

use learn_core::model::{Deck, DeckId};

/// UI-ready representation of a deck for selection controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckOptionVm {
    pub id: DeckId,
    pub label: String,
}

impl DeckOptionVm {
    #[must_use]
    pub fn new(id: DeckId, label: String) -> Self {
        Self { id, label }
    }
}

/// Convert domain decks into selection-friendly view models.
#[must_use]
pub fn map_deck_options(decks: &[Deck]) -> Vec<DeckOptionVm> {
    decks
        .iter()
        .map(|deck| {
            let label = format_deck_label(deck.name(), deck.description());
            DeckOptionVm::new(deck.id(), label)
        })
        .collect()
}

fn format_deck_label(name: &str, description: Option<&str>) -> String {
    match description {
        Some(desc) => format!("{name} - {desc}"),
        None => name.to_owned(),
    }
}

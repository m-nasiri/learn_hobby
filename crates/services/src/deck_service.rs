use std::sync::Arc;

use learn_core::model::{Deck, DeckId, DeckSettings};
use storage::repository::{DeckRepository, NewDeckRecord};

use crate::error::DeckServiceError;
use crate::Clock;

/// Orchestrates deck creation and persistence.
#[derive(Clone)]
pub struct DeckService {
    clock: Clock,
    decks: Arc<dyn DeckRepository>,
}

impl DeckService {
    #[must_use]
    pub fn new(clock: Clock, decks: Arc<dyn DeckRepository>) -> Self {
        Self { clock, decks }
    }

    /// Create a new deck with the given settings and persist it.
    ///
    /// # Errors
    ///
    /// Returns `DeckServiceError::Deck` for validation failures.
    /// Returns `DeckServiceError::Storage` if persistence fails.
    pub async fn create_deck(
        &self,
        name: String,
        description: Option<String>,
        settings: DeckSettings,
    ) -> Result<DeckId, DeckServiceError> {
        let now = self.clock.now();
        let deck = Deck::new(DeckId::new(1), name, description, settings, now)?;
        let deck_id = self
            .decks
            .insert_new_deck(NewDeckRecord::from_deck(&deck))
            .await?;
        Ok(deck_id)
    }

    /// List decks ordered by ID, up to the given limit.
    ///
    /// # Errors
    ///
    /// Returns `DeckServiceError::Storage` if repository access fails.
    pub async fn list_decks(&self, limit: u32) -> Result<Vec<Deck>, DeckServiceError> {
        let decks = self.decks.list_decks(limit).await?;
        Ok(decks)
    }
}

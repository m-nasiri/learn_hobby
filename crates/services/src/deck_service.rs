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

    /// Fetch a deck by ID.
    ///
    /// Returns `Ok(None)` when the deck does not exist.
    ///
    /// # Errors
    ///
    /// Returns `DeckServiceError::Storage` if repository access fails.
    pub async fn get_deck(&self, deck_id: DeckId) -> Result<Option<Deck>, DeckServiceError> {
        let deck = self.decks.get_deck(deck_id).await?;
        Ok(deck)
    }

    /// Rename a deck while preserving existing settings and metadata.
    ///
    /// # Errors
    ///
    /// Returns `DeckServiceError::Deck` if validation fails.
    /// Returns `DeckServiceError::Storage` if repository access fails.
    pub async fn rename_deck(
        &self,
        deck_id: DeckId,
        name: String,
    ) -> Result<(), DeckServiceError> {
        let deck = self
            .decks
            .get_deck(deck_id)
            .await?
            .ok_or(storage::repository::StorageError::NotFound)?;

        self.update_deck(
            deck_id,
            name,
            deck.description().map(str::to_owned),
            deck.settings().clone(),
        )
        .await
    }

    /// Update deck name, description, and settings.
    ///
    /// # Errors
    ///
    /// Returns `DeckServiceError::Deck` if validation fails.
    /// Returns `DeckServiceError::Storage` if repository access fails.
    pub async fn update_deck(
        &self,
        deck_id: DeckId,
        name: String,
        description: Option<String>,
        settings: DeckSettings,
    ) -> Result<(), DeckServiceError> {
        let deck = self
            .decks
            .get_deck(deck_id)
            .await?
            .ok_or(storage::repository::StorageError::NotFound)?;

        let updated = Deck::new(deck.id(), name, description, settings, deck.created_at())?;
        self.decks.upsert_deck(&updated).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use learn_core::time::fixed_now;
    use storage::repository::InMemoryRepository;

    #[tokio::test]
    async fn get_deck_returns_persisted_deck() {
        let repo = InMemoryRepository::new();
        let deck = Deck::new(
            DeckId::new(1),
            "Test",
            None,
            DeckSettings::default_for_adhd(),
            fixed_now(),
        )
        .unwrap();
        repo.upsert_deck(&deck).await.unwrap();

        let service = DeckService::new(Clock::Fixed(fixed_now()), std::sync::Arc::new(repo));
        let fetched = service.get_deck(deck.id()).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name(), "Test");
    }

    #[tokio::test]
    async fn update_deck_persists_daily_limits() {
        let repo = InMemoryRepository::new();
        let clock = Clock::Fixed(fixed_now());
        let service = DeckService::new(clock, std::sync::Arc::new(repo));

        let deck_id = service
            .create_deck(
                "Daily Limits".to_string(),
                None,
                DeckSettings::default_for_adhd(),
            )
            .await
            .unwrap();

        let updated_settings = DeckSettings::new(12, 55, 5, false).unwrap();
        service
            .update_deck(deck_id, "Daily Limits".to_string(), None, updated_settings)
            .await
            .unwrap();

        let refreshed = service.get_deck(deck_id).await.unwrap().unwrap();
        assert_eq!(refreshed.settings().new_cards_per_day(), 12);
        assert_eq!(refreshed.settings().review_limit_per_day(), 55);
        assert_eq!(refreshed.settings().micro_session_size(), 5);
        assert!(!refreshed.settings().protect_overload());
    }
}

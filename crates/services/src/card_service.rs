use std::sync::Arc;

use learn_core::model::{Card, CardError, CardId, CardPhase, ContentDraft, DeckId};
use storage::repository::{CardRepository, NewCardRecord};

use crate::error::CardServiceError;
use crate::Clock;

/// Orchestrates card creation and persistence.
#[derive(Clone)]
pub struct CardService {
    clock: Clock,
    cards: Arc<dyn CardRepository>,
}

impl CardService {
    #[must_use]
    pub fn new(clock: Clock, cards: Arc<dyn CardRepository>) -> Self {
        Self { clock, cards }
    }

    /// Create a new card from validated drafts and persist it.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Card` for validation failures.
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn create_card(
        &self,
        deck_id: DeckId,
        prompt: ContentDraft,
        answer: ContentDraft,
    ) -> Result<CardId, CardServiceError> {
        let now = self.clock.now();
        let prompt = prompt
            .validate(now, None, None)
            .map_err(CardError::InvalidPrompt)?;
        let answer = answer
            .validate(now, None, None)
            .map_err(CardError::InvalidAnswer)?;

        let record = NewCardRecord {
            deck_id,
            prompt_text: prompt.text().to_owned(),
            prompt_media_id: prompt.media_id().map(|m| m.value()),
            answer_text: answer.text().to_owned(),
            answer_media_id: answer.media_id().map(|m| m.value()),
            phase: CardPhase::New,
            created_at: now,
            next_review_at: now,
            last_review_at: None,
            review_count: 0,
            stability: None,
            difficulty: None,
        };

        let card_id = self.cards.insert_new_card(record).await?;
        Ok(card_id)
    }

    /// Persist an existing card update.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn save_card(&self, card: &Card) -> Result<(), CardServiceError> {
        self.cards.upsert_card(card).await?;
        Ok(())
    }

    /// List cards for a deck up to the given limit.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn list_cards(
        &self,
        deck_id: DeckId,
        limit: u32,
    ) -> Result<Vec<Card>, CardServiceError> {
        let cards = self.cards.list_cards(deck_id, limit).await?;
        Ok(cards)
    }

    /// Update a card's prompt/answer content while preserving scheduling state.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Card` for validation failures.
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn update_card_content(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        prompt: ContentDraft,
        answer: ContentDraft,
    ) -> Result<(), CardServiceError> {
        let now = self.clock.now();
        let prompt = prompt
            .validate(now, None, None)
            .map_err(CardError::InvalidPrompt)?;
        let answer = answer
            .validate(now, None, None)
            .map_err(CardError::InvalidAnswer)?;

        let mut cards = self.cards.get_cards(deck_id, &[card_id]).await?;
        let Some(card) = cards.pop() else {
            return Err(storage::repository::StorageError::NotFound.into());
        };

        let (stability, difficulty) = if card.review_count() == 0 {
            (0.0, 0.0)
        } else {
            let state = card
                .memory_state()
                .ok_or_else(|| CardError::InvalidPersistedState("missing memory state".into()))?;
            (state.stability, state.difficulty)
        };

        let updated = Card::from_persisted(
            card.id(),
            card.deck_id(),
            prompt,
            answer,
            card.created_at(),
            card.next_review_at(),
            card.last_review_at(),
            card.phase(),
            card.review_count(),
            stability,
            difficulty,
        )?;

        self.cards.upsert_card(&updated).await?;
        Ok(())
    }
}

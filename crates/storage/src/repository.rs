use async_trait::async_trait;
use chrono::{DateTime, Utc};
use learn_core::model::{content::ContentDraft, Card, CardError, CardId, CardPhase, Deck, DeckId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Errors surfaced by storage adapters.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StorageError {
    #[error("not found")]
    NotFound,

    #[error("conflict")]
    Conflict,

    #[error("connection error: {0}")]
    Connection(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Persisted shape for a card, including lifecycle phase.
///
/// This mirrors the domain `Card` so repositories can serialize/deserialize
/// without leaking storage concerns into the domain layer.
#[derive(Debug, Clone)]
pub struct CardRecord {
    pub id: CardId,
    pub deck_id: DeckId,
    pub prompt: String,
    pub answer: String,
    pub phase: CardPhase,
    pub created_at: DateTime<Utc>,
    pub next_review_at: DateTime<Utc>,
    pub last_review_at: Option<DateTime<Utc>>,
    pub review_count: u32,
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
}

impl CardRecord {
    #[must_use]
    pub fn from_card(card: &Card) -> Self {
        Self {
            id: card.id(),
            deck_id: card.deck_id(),
            prompt: card.prompt().text().to_owned(),
            answer: card.answer().text().to_owned(),
            phase: card.phase(),
            created_at: card.created_at(),
            next_review_at: card.next_review_at(),
            last_review_at: card.last_review_at(),
            review_count: card.review_count(),
            stability: card.memory_state().map(|m| m.stability),
            difficulty: card.memory_state().map(|m| m.difficulty),
        }
    }

    /// Convert the record back into a domain `Card`.
    ///
    /// # Errors
    ///
    /// Returns `CardError` if prompt/answer fail validation or phase cannot be applied.
    pub fn into_card(self) -> Result<Card, CardError> {
        let prompt = ContentDraft::text_only(self.prompt)
            .validate(self.created_at, None, None)
            .map_err(CardError::InvalidPrompt)?;
        let answer = ContentDraft::text_only(self.answer)
            .validate(self.created_at, None, None)
            .map_err(CardError::InvalidAnswer)?;

        // For brand-new cards (review_count == 0), stability/difficulty are semantically unset.
        // We allow `None` in storage and normalize to 0.0 for the persisted constructor.
        let (stability, difficulty) = if self.review_count == 0 {
            (0.0, 0.0)
        } else {
            (
                self.stability
                    .ok_or_else(|| CardError::InvalidPersistedState("missing stability".into()))?,
                self.difficulty
                    .ok_or_else(|| CardError::InvalidPersistedState("missing difficulty".into()))?,
            )
        };

        Card::from_persisted(
            self.id,
            self.deck_id,
            prompt,
            answer,
            self.created_at,
            self.next_review_at,
            self.last_review_at,
            self.phase,
            self.review_count,
            stability,
            difficulty,
        )
    }
}

/// Repository contract for decks and cards.
#[async_trait]
pub trait DeckRepository: Send + Sync {
    /// Persist or update a deck.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if the deck cannot be stored.
    async fn upsert_deck(&self, deck: &Deck) -> Result<(), StorageError>;

    /// Fetch a deck by ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if missing, or other storage errors.
    async fn get_deck(&self, id: DeckId) -> Result<Deck, StorageError>;
}

#[async_trait]
pub trait CardRepository: Send + Sync {
    /// Persist or update a card with phase information.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if the card cannot be stored.
    async fn upsert_card(&self, card: &Card) -> Result<(), StorageError>;

    /// Fetch cards for a deck by IDs.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if any are missing, or other storage errors.
    async fn get_cards(&self, deck_id: DeckId, ids: &[CardId]) -> Result<Vec<Card>, StorageError>;
}

/// Simple in-memory repository implementation for testing and prototyping.
#[derive(Clone, Default)]
pub struct InMemoryRepository {
    decks: Arc<Mutex<HashMap<DeckId, Deck>>>,
    cards: Arc<Mutex<HashMap<(DeckId, CardId), Card>>>,
}

impl InMemoryRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            decks: Arc::new(Mutex::new(HashMap::new())),
            cards: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl DeckRepository for InMemoryRepository {
    async fn upsert_deck(&self, deck: &Deck) -> Result<(), StorageError> {
        let mut guard = self
            .decks
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        guard.insert(deck.id(), deck.clone());
        Ok(())
    }

    async fn get_deck(&self, id: DeckId) -> Result<Deck, StorageError> {
        let guard = self
            .decks
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        guard.get(&id).cloned().ok_or(StorageError::NotFound)
    }
}

#[async_trait]
impl CardRepository for InMemoryRepository {
    async fn upsert_card(&self, card: &Card) -> Result<(), StorageError> {
        let mut guard = self
            .cards
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        guard.insert((card.deck_id(), card.id()), card.clone());
        Ok(())
    }

    async fn get_cards(&self, deck_id: DeckId, ids: &[CardId]) -> Result<Vec<Card>, StorageError> {
        let guard = self
            .cards
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let mut found = Vec::with_capacity(ids.len());
        for id in ids {
            match guard.get(&(deck_id, *id)) {
                Some(card) => found.push(card.clone()),
                None => return Err(StorageError::NotFound),
            }
        }
        Ok(found)
    }
}

/// Aggregates deck and card repositories behind trait objects for easy backend swapping.
#[derive(Clone)]
pub struct Storage {
    pub decks: Arc<dyn DeckRepository>,
    pub cards: Arc<dyn CardRepository>,
}

impl Storage {
    #[must_use]
    pub fn in_memory() -> Self {
        let repo = InMemoryRepository::new();
        let decks: Arc<dyn DeckRepository> = Arc::new(repo.clone());
        let cards: Arc<dyn CardRepository> = Arc::new(repo);
        Self { decks, cards }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::model::content::ContentDraft;
    use learn_core::model::{DeckSettings, ReviewGrade};
    use learn_core::time::fixed_now;

    fn build_deck(id: u64) -> Deck {
        Deck::new(
            DeckId::new(id),
            format!("Deck {id}"),
            None,
            DeckSettings::default_for_adhd(),
            fixed_now(),
        )
        .unwrap()
    }

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
    async fn round_trips_card_with_phase() {
        let repo = InMemoryRepository::new();
        let deck = build_deck(1);
        repo.upsert_deck(&deck).await.unwrap();

        let mut card = build_card(1, deck.id());
        // simulate a review to move phase forward
        let outcome = learn_core::model::ReviewOutcome::new(
            fixed_now() + chrono::Duration::days(1),
            1.0,
            2.0,
            0.0,
            1.0,
        );
        card.apply_review_with_phase(ReviewGrade::Good, &outcome, fixed_now());
        assert_eq!(card.phase(), CardPhase::Learning);

        repo.upsert_card(&card).await.unwrap();

        let fetched = repo.get_cards(deck.id(), &[card.id()]).await.unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].phase(), CardPhase::Learning);
        assert_eq!(fetched[0].review_count(), 1);
    }
}

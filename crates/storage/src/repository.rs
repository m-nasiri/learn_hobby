use async_trait::async_trait;
use chrono::{DateTime, Utc};
use learn_core::model::{
    Card, CardError, CardId, CardPhase, Deck, DeckId, DeckSettings, MediaId, ReviewGrade,
    ReviewLog, ReviewOutcome, SessionSummary, Tag, TagId, TagName, content::Content,
};
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
/// This is a storage-friendly representation used to round-trip the domain `Card`
/// while allowing nullable persisted state (e.g. stability/difficulty before first review).
#[derive(Debug, Clone)]
pub struct CardRecord {
    pub id: CardId,
    pub deck_id: DeckId,
    pub prompt_text: String,
    pub prompt_media_id: Option<u64>,
    pub answer_text: String,
    pub answer_media_id: Option<u64>,
    pub phase: CardPhase,
    pub created_at: DateTime<Utc>,
    pub next_review_at: DateTime<Utc>,
    pub last_review_at: Option<DateTime<Utc>>,
    pub review_count: u32,
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
}

/// Persisted shape for inserting a brand-new card (no ID yet).
#[derive(Debug, Clone)]
pub struct NewCardRecord {
    pub deck_id: DeckId,
    pub prompt_text: String,
    pub prompt_media_id: Option<u64>,
    pub answer_text: String,
    pub answer_media_id: Option<u64>,
    pub phase: CardPhase,
    pub created_at: DateTime<Utc>,
    pub next_review_at: DateTime<Utc>,
    pub last_review_at: Option<DateTime<Utc>>,
    pub review_count: u32,
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
}

/// Persisted shape for inserting a brand-new deck (no ID yet).
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct NewDeckRecord {
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub new_cards_per_day: u32,
    pub review_limit_per_day: u32,
    pub micro_session_size: u32,
    pub protect_overload: bool,
    pub preserve_stability_on_lapse: bool,
    pub lapse_min_interval_secs: u32,
    pub show_timer: bool,
    pub soft_time_reminder: bool,
    pub auto_advance_cards: bool,
    pub soft_time_reminder_secs: u32,
    pub auto_reveal_secs: u32,
    pub fsrs_target_retention: f32,
    pub fsrs_optimize_enabled: bool,
    pub fsrs_optimize_after: u32,
}

/// Aggregate card counts for a deck at a given time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeckPracticeCounts {
    pub total: u32,
    pub due: u32,
    pub new: u32,
}

/// Aggregate card counts for a tag scoped to a deck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPracticeCounts {
    pub name: TagName,
    pub total: u32,
    pub due: u32,
    pub new: u32,
}

impl NewDeckRecord {
    #[must_use]
    pub fn from_deck(deck: &Deck) -> Self {
        Self {
            name: deck.name().to_owned(),
            description: deck.description().map(ToString::to_string),
            created_at: deck.created_at(),
            new_cards_per_day: deck.settings().new_cards_per_day(),
            review_limit_per_day: deck.settings().review_limit_per_day(),
            micro_session_size: deck.settings().micro_session_size(),
            protect_overload: deck.settings().protect_overload(),
            preserve_stability_on_lapse: deck.settings().preserve_stability_on_lapse(),
            lapse_min_interval_secs: deck.settings().lapse_min_interval_secs(),
            show_timer: deck.settings().show_timer(),
            soft_time_reminder: deck.settings().soft_time_reminder(),
            auto_advance_cards: deck.settings().auto_advance_cards(),
            soft_time_reminder_secs: deck.settings().soft_time_reminder_secs(),
            auto_reveal_secs: deck.settings().auto_reveal_secs(),
            fsrs_target_retention: deck.settings().fsrs_target_retention(),
            fsrs_optimize_enabled: deck.settings().fsrs_optimize_enabled(),
            fsrs_optimize_after: deck.settings().fsrs_optimize_after(),
        }
    }
}

impl CardRecord {
    #[must_use]
    pub fn from_card(card: &Card) -> Self {
        Self {
            id: card.id(),
            deck_id: card.deck_id(),
            prompt_text: card.prompt().text().to_owned(),
            prompt_media_id: card.prompt().media_id().map(|m| m.value()),
            answer_text: card.answer().text().to_owned(),
            answer_media_id: card.answer().media_id().map(|m| m.value()),
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
        let prompt =
            Content::from_persisted(self.prompt_text, self.prompt_media_id.map(MediaId::new))
                .map_err(CardError::InvalidPrompt)?;
        let answer =
            Content::from_persisted(self.answer_text, self.answer_media_id.map(MediaId::new))
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

/// Persisted representation of a review event, including FSRS outputs.
#[derive(Debug, Clone)]
pub struct ReviewLogRecord {
    pub id: Option<i64>,
    pub deck_id: DeckId,
    pub card_id: CardId,
    pub grade: ReviewGrade,
    pub reviewed_at: DateTime<Utc>,
    pub elapsed_days: f64,
    pub scheduled_days: f64,
    pub stability: f64,
    pub difficulty: f64,
    pub next_review_at: DateTime<Utc>,
}

impl ReviewLogRecord {
    /// Build a storage record from a domain log and FSRS outcome.
    ///
    /// # Errors
    ///
    /// None. This helper only moves data between layers.
    #[must_use]
    pub fn from_applied(deck_id: DeckId, log: &ReviewLog, outcome: &ReviewOutcome) -> Self {
        Self {
            id: None,
            deck_id,
            card_id: log.card_id,
            grade: log.grade,
            reviewed_at: log.reviewed_at,
            elapsed_days: outcome.elapsed_days,
            scheduled_days: outcome.scheduled_days,
            stability: outcome.stability,
            difficulty: outcome.difficulty,
            next_review_at: outcome.next_review,
        }
    }

    #[must_use]
    pub fn with_id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }
}

/// Repository contract for decks and cards.
#[async_trait]
pub trait DeckRepository: Send + Sync {
    /// Insert a brand-new deck and return its assigned ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if the deck cannot be stored.
    async fn insert_new_deck(&self, deck: NewDeckRecord) -> Result<DeckId, StorageError>;

    /// Persist or update a deck.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if the deck cannot be stored.
    async fn upsert_deck(&self, deck: &Deck) -> Result<(), StorageError>;

    /// Fetch a deck by ID.
    ///
    /// Returns `Ok(None)` when the deck does not exist.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on storage failures.
    async fn get_deck(&self, id: DeckId) -> Result<Option<Deck>, StorageError>;

    /// List decks up to the given limit, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on storage failures.
    async fn list_decks(&self, limit: u32) -> Result<Vec<Deck>, StorageError>;
}

#[async_trait]
pub trait CardRepository: Send + Sync {
    /// Insert a brand-new card and return its assigned ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if persistence fails.
    async fn insert_new_card(&self, card: NewCardRecord) -> Result<CardId, StorageError>;

    /// Persist or update a card with phase information.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if the card cannot be stored.
    async fn upsert_card(&self, card: &Card) -> Result<(), StorageError>;

    /// Delete a card by ID within a deck.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if the card is missing.
    /// Returns `StorageError` on storage failures.
    async fn delete_card(&self, deck_id: DeckId, card_id: CardId) -> Result<(), StorageError>;

    /// Fetch cards for a deck by IDs.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if any are missing, or other storage errors.
    async fn get_cards(&self, deck_id: DeckId, ids: &[CardId]) -> Result<Vec<Card>, StorageError>;

    /// Fetch due cards for a deck up to the given limit, ordered by next review time.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn due_cards(
        &self,
        deck_id: DeckId,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<Card>, StorageError>;

    /// Fetch new (unreviewed) cards for a deck up to the given limit, ordered by creation time.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn new_cards(&self, deck_id: DeckId, limit: u32) -> Result<Vec<Card>, StorageError>;

    /// List cards for a deck up to the given limit.
    ///
    /// Results are ordered by `created_at` descending, then `id` descending.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn list_cards(&self, deck_id: DeckId, limit: u32) -> Result<Vec<Card>, StorageError>;

    /// Count cards created in a time range for a deck.
    ///
    /// The range is inclusive of `start` and exclusive of `end`.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn count_cards_created_between(
        &self,
        deck_id: DeckId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u32, StorageError>;

    /// Count cards currently in a mistake/relearning state.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn mistakes_count(&self, deck_id: DeckId) -> Result<u32, StorageError>;

    /// Reset learning state for all cards in a deck.
    ///
    /// Resets phase to `New`, clears review metadata, and sets `next_review_at` to `now`.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn reset_deck_learning(
        &self,
        deck_id: DeckId,
        now: DateTime<Utc>,
    ) -> Result<u64, StorageError>;

    /// Count total, new, and due cards for a deck at the given time.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn deck_practice_counts(
        &self,
        deck_id: DeckId,
        now: DateTime<Utc>,
    ) -> Result<DeckPracticeCounts, StorageError>;

    /// Count total, new, and due cards for each tag in a deck at the given time.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn list_tag_practice_counts(
        &self,
        deck_id: DeckId,
        now: DateTime<Utc>,
    ) -> Result<Vec<TagPracticeCounts>, StorageError>;

    /// List cards for a deck that match any of the provided tags.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn list_cards_by_tags(
        &self,
        deck_id: DeckId,
        tag_names: &[TagName],
    ) -> Result<Vec<Card>, StorageError>;

    /// Returns true if a card with the given prompt exists in the deck.
    ///
    /// Comparison is normalized (trimmed, case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on connection or serialization failure.
    async fn prompt_exists(
        &self,
        deck_id: DeckId,
        prompt_text: &str,
        exclude: Option<CardId>,
    ) -> Result<bool, StorageError>;

    /// List all tags available for a deck.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on storage failures.
    async fn list_tags_for_deck(&self, deck_id: DeckId) -> Result<Vec<Tag>, StorageError>;

    /// List tags attached to a card.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on storage failures.
    async fn list_tags_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
    ) -> Result<Vec<Tag>, StorageError>;

    /// Replace the tags for a card, creating new tags if needed.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on storage failures.
    async fn set_tags_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        tag_names: &[TagName],
    ) -> Result<Vec<Tag>, StorageError>;
}

#[async_trait]
pub trait ReviewLogRepository: Send + Sync {
    /// Append a review log, returning the assigned ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if the log cannot be persisted or serialized.
    async fn append_log(&self, log: ReviewLogRecord) -> Result<i64, StorageError>;

    /// Fetch all logs for a card, ordered by review time.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on storage failures or deserialization issues.
    async fn logs_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
    ) -> Result<Vec<ReviewLogRecord>, StorageError>;
}

#[async_trait]
pub trait ReviewPersistence: Send + Sync {
    /// Persist a card update and the associated review log atomically.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if persistence fails or if the log/card IDs mismatch.
    async fn apply_review(&self, card: &Card, log: ReviewLogRecord) -> Result<i64, StorageError>;
}

/// A session summary paired with its storage-assigned identifier.
///
/// This is useful for UI navigation (e.g. “open summary details”) without requiring
/// a separate lookup after listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummaryRow {
    pub id: i64,
    pub summary: SessionSummary,
}

impl SessionSummaryRow {
    #[must_use]
    pub fn new(id: i64, summary: SessionSummary) -> Self {
        Self { id, summary }
    }
}

#[async_trait]
pub trait SessionSummaryRepository: Send + Sync {
    /// Persist a session summary, returning the assigned ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if persistence fails.
    async fn append_summary(&self, summary: &SessionSummary) -> Result<i64, StorageError>;

    /// Fetch a session summary by ID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::NotFound` if missing.
    async fn get_summary(&self, id: i64) -> Result<SessionSummary, StorageError>;

    /// List session summaries for a deck within an optional time range.
    ///
    /// Results are ordered by `completed_at` descending, limited by `limit`.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on persistence failures.
    async fn list_summaries(
        &self,
        deck_id: DeckId,
        completed_from: Option<DateTime<Utc>>,
        completed_until: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummary>, StorageError>;

    /// List session summaries for a deck within an optional time range, preserving storage IDs.
    ///
    /// Results are ordered by `completed_at` descending, then `id` descending, limited by `limit`.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on persistence failures.
    async fn list_summary_rows(
        &self,
        deck_id: DeckId,
        completed_from: Option<DateTime<Utc>>,
        completed_until: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummaryRow>, StorageError>;
}

#[derive(Default)]
struct InMemState {
    decks: HashMap<DeckId, Deck>,
    cards: HashMap<CardId, Card>,
    tags: HashMap<TagId, Tag>,
    card_tags: HashMap<CardId, Vec<TagId>>,
    logs: Vec<ReviewLogRecord>,
    summaries: HashMap<i64, SessionSummary>,
    next_deck_id: u64,
    next_card_id: u64,
    next_tag_id: u64,
    next_log_id: i64,
    next_summary_id: i64,
}

/// Simple in-memory repository implementation for testing and prototyping.
#[derive(Clone, Default)]
pub struct InMemoryRepository {
    state: Arc<Mutex<InMemState>>,
}

impl InMemoryRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemState {
                next_deck_id: 1,
                next_card_id: 1,
                next_tag_id: 1,
                next_log_id: 1,
                next_summary_id: 1,
                ..InMemState::default()
            })),
        }
    }
}

fn limit_usize(limit: u32) -> usize {
    usize::try_from(limit).unwrap_or(usize::MAX)
}

fn normalize_prompt(text: &str) -> String {
    text.trim().to_lowercase()
}

#[async_trait]
impl DeckRepository for InMemoryRepository {
    async fn insert_new_deck(&self, deck: NewDeckRecord) -> Result<DeckId, StorageError> {
        let settings = DeckSettings::new(
            deck.new_cards_per_day,
            deck.review_limit_per_day,
            deck.micro_session_size,
            deck.protect_overload,
            deck.preserve_stability_on_lapse,
            deck.lapse_min_interval_secs,
            deck.show_timer,
            deck.soft_time_reminder,
            deck.auto_advance_cards,
            deck.soft_time_reminder_secs,
            deck.auto_reveal_secs,
            deck.fsrs_target_retention,
            deck.fsrs_optimize_enabled,
            deck.fsrs_optimize_after,
        )
        .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let id = guard.next_deck_id;
        guard.next_deck_id = id
            .checked_add(1)
            .ok_or_else(|| StorageError::Serialization("deck_id overflow".into()))?;

        let deck = Deck::new(
            DeckId::new(id),
            deck.name,
            deck.description,
            settings,
            deck.created_at,
        )
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let deck_id = deck.id();
        guard.decks.insert(deck_id, deck);
        Ok(deck_id)
    }

    async fn upsert_deck(&self, deck: &Deck) -> Result<(), StorageError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        if deck.id().value() >= guard.next_deck_id {
            guard.next_deck_id = deck
                .id()
                .value()
                .checked_add(1)
                .ok_or_else(|| StorageError::Serialization("deck_id overflow".into()))?;
        }
        guard.decks.insert(deck.id(), deck.clone());
        Ok(())
    }

    async fn get_deck(&self, id: DeckId) -> Result<Option<Deck>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        Ok(guard.decks.get(&id).cloned())
    }

    async fn list_decks(&self, limit: u32) -> Result<Vec<Deck>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let mut decks: Vec<Deck> = guard.decks.values().cloned().collect();
        decks.sort_by_key(|deck| deck.id().value());
        decks.truncate(limit_usize(limit));
        Ok(decks)
    }
}

#[async_trait]
impl CardRepository for InMemoryRepository {
    async fn insert_new_card(&self, card: NewCardRecord) -> Result<CardId, StorageError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let id = guard.next_card_id;
        guard.next_card_id = id
            .checked_add(1)
            .ok_or_else(|| StorageError::Serialization("card_id overflow".into()))?;
        let record = CardRecord {
            id: CardId::new(id),
            deck_id: card.deck_id,
            prompt_text: card.prompt_text,
            prompt_media_id: card.prompt_media_id,
            answer_text: card.answer_text,
            answer_media_id: card.answer_media_id,
            phase: card.phase,
            created_at: card.created_at,
            next_review_at: card.next_review_at,
            last_review_at: card.last_review_at,
            review_count: card.review_count,
            stability: card.stability,
            difficulty: card.difficulty,
        };
        let card = record
            .into_card()
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let id = card.id();
        guard.cards.insert(id, card);
        Ok(id)
    }

    async fn upsert_card(&self, card: &Card) -> Result<(), StorageError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        if card.id().value() >= guard.next_card_id {
            guard.next_card_id = card
                .id()
                .value()
                .checked_add(1)
                .ok_or_else(|| StorageError::Serialization("card_id overflow".into()))?;
        }
        guard.cards.insert(card.id(), card.clone());
        Ok(())
    }

    async fn delete_card(&self, deck_id: DeckId, card_id: CardId) -> Result<(), StorageError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        match guard.cards.get(&card_id) {
            Some(card) if card.deck_id() == deck_id => {
                guard.cards.remove(&card_id);
                guard.card_tags.remove(&card_id);
                guard
                    .logs
                    .retain(|log| !(log.deck_id == deck_id && log.card_id == card_id));
                Ok(())
            }
            _ => Err(StorageError::NotFound),
        }
    }

    async fn get_cards(&self, deck_id: DeckId, ids: &[CardId]) -> Result<Vec<Card>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let mut found = Vec::with_capacity(ids.len());
        for id in ids {
            match guard.cards.get(id) {
                Some(card) if card.deck_id() == deck_id => found.push(card.clone()),
                _ => return Err(StorageError::NotFound),
            }
        }
        Ok(found)
    }

    async fn due_cards(
        &self,
        deck_id: DeckId,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<Card>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut due: Vec<Card> = guard
            .cards
            .values()
            .filter(|c| c.deck_id() == deck_id && c.review_count() > 0 && c.next_review_at() <= now)
            .cloned()
            .collect();
        due.sort_by_key(|c| (c.next_review_at(), c.id().value()));
        due.truncate(limit_usize(limit));
        Ok(due)
    }

    async fn new_cards(&self, deck_id: DeckId, limit: u32) -> Result<Vec<Card>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut new_cards: Vec<Card> = guard
            .cards
            .values()
            .filter(|c| c.deck_id() == deck_id && c.review_count() == 0)
            .cloned()
            .collect();
        new_cards.sort_by_key(|c| (c.created_at(), c.id().value()));
        new_cards.truncate(limit_usize(limit));
        Ok(new_cards)
    }

    async fn list_cards(&self, deck_id: DeckId, limit: u32) -> Result<Vec<Card>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut cards: Vec<Card> = guard
            .cards
            .values()
            .filter(|c| c.deck_id() == deck_id)
            .cloned()
            .collect();
        // Match SQLite ordering: created_at DESC, id DESC
        cards.sort_by(|a, b| {
            b.created_at()
                .cmp(&a.created_at())
                .then_with(|| b.id().value().cmp(&a.id().value()))
        });
        cards.truncate(limit_usize(limit));
        Ok(cards)
    }

    async fn count_cards_created_between(
        &self,
        deck_id: DeckId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u32, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let count = guard
            .cards
            .values()
            .filter(|card| {
                card.deck_id() == deck_id
                    && card.created_at() >= start
                    && card.created_at() < end
            })
            .count();
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
    }

    async fn mistakes_count(&self, deck_id: DeckId) -> Result<u32, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let count = guard
            .cards
            .values()
            .filter(|card| card.deck_id() == deck_id && card.phase() == CardPhase::Relearning)
            .count();
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
    }

    async fn reset_deck_learning(
        &self,
        deck_id: DeckId,
        now: DateTime<Utc>,
    ) -> Result<u64, StorageError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let ids: Vec<CardId> = guard
            .cards
            .values()
            .filter(|card| card.deck_id() == deck_id)
            .map(Card::id)
            .collect();
        let mut updated = 0_u64;
        for id in ids {
            let Some(card) = guard.cards.get(&id) else {
                continue;
            };
            let prompt = card.prompt().clone();
            let answer = card.answer().clone();
            let created_at = card.created_at();
            let reset = Card::from_persisted(
                id,
                deck_id,
                prompt,
                answer,
                created_at,
                now,
                None,
                CardPhase::New,
                0,
                0.0,
                0.0,
            )
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
            guard.cards.insert(id, reset);
            updated = updated.saturating_add(1);
        }
        Ok(updated)
    }

    async fn deck_practice_counts(
        &self,
        deck_id: DeckId,
        now: DateTime<Utc>,
    ) -> Result<DeckPracticeCounts, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut total = 0u32;
        let mut due = 0u32;
        let mut new = 0u32;

        for card in guard.cards.values().filter(|c| c.deck_id() == deck_id) {
            total = total.saturating_add(1);
            if card.review_count() == 0 {
                new = new.saturating_add(1);
            } else if card.next_review_at() <= now {
                due = due.saturating_add(1);
            }
        }

        Ok(DeckPracticeCounts { total, due, new })
    }

    async fn list_tag_practice_counts(
        &self,
        deck_id: DeckId,
        now: DateTime<Utc>,
    ) -> Result<Vec<TagPracticeCounts>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut tags: Vec<Tag> = guard
            .tags
            .values()
            .filter(|tag| tag.deck_id() == deck_id)
            .cloned()
            .collect();
        tags.sort_by(|a, b| {
            a.name()
                .as_str()
                .cmp(b.name().as_str())
                .then_with(|| a.id().value().cmp(&b.id().value()))
        });

        let mut counts: HashMap<TagId, DeckPracticeCounts> = HashMap::new();
        for tag in &tags {
            counts.insert(
                tag.id(),
                DeckPracticeCounts {
                    total: 0,
                    due: 0,
                    new: 0,
                },
            );
        }

        for card in guard.cards.values().filter(|c| c.deck_id() == deck_id) {
            if let Some(tag_ids) = guard.card_tags.get(&card.id()) {
                for tag_id in tag_ids {
                    if let Some(entry) = counts.get_mut(tag_id) {
                        entry.total = entry.total.saturating_add(1);
                        if card.review_count() == 0 {
                            entry.new = entry.new.saturating_add(1);
                        } else if card.next_review_at() <= now {
                            entry.due = entry.due.saturating_add(1);
                        }
                    }
                }
            }
        }

        let mut out = Vec::with_capacity(tags.len());
        for tag in tags {
            let counts = counts.get(&tag.id()).copied().unwrap_or(DeckPracticeCounts {
                total: 0,
                due: 0,
                new: 0,
            });
            out.push(TagPracticeCounts {
                name: tag.name().clone(),
                total: counts.total,
                due: counts.due,
                new: counts.new,
            });
        }

        Ok(out)
    }

    async fn list_cards_by_tags(
        &self,
        deck_id: DeckId,
        tag_names: &[TagName],
    ) -> Result<Vec<Card>, StorageError> {
        if tag_names.is_empty() {
            return Ok(Vec::new());
        }

        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let tag_ids: Vec<TagId> = guard
            .tags
            .values()
            .filter(|tag| tag.deck_id() == deck_id)
            .filter(|tag| tag_names.iter().any(|name| name == tag.name()))
            .map(Tag::id)
            .collect();

        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut cards: Vec<Card> = guard
            .cards
            .values()
            .filter(|card| card.deck_id() == deck_id)
            .filter(|card| {
                guard
                    .card_tags
                    .get(&card.id())
                    .is_some_and(|ids| ids.iter().any(|id| tag_ids.contains(id)))
            })
            .cloned()
            .collect();

        // Default ordering: created_at DESC, id DESC (matches list_cards behavior)
        cards.sort_by(|a, b| {
            b.created_at()
                .cmp(&a.created_at())
                .then_with(|| b.id().value().cmp(&a.id().value()))
        });

        Ok(cards)
    }

    async fn prompt_exists(
        &self,
        deck_id: DeckId,
        prompt_text: &str,
        exclude: Option<CardId>,
    ) -> Result<bool, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let needle = normalize_prompt(prompt_text);
        if needle.is_empty() {
            return Ok(false);
        }

        let exists = guard.cards.values().any(|card| {
            card.deck_id() == deck_id
                && Some(card.id()) != exclude
                && normalize_prompt(card.prompt().text()) == needle
        });

        Ok(exists)
    }

    async fn list_tags_for_deck(&self, deck_id: DeckId) -> Result<Vec<Tag>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let mut tags: Vec<Tag> = guard
            .tags
            .values()
            .filter(|tag| tag.deck_id() == deck_id)
            .cloned()
            .collect();
        tags.sort_by(|a, b| {
            a.name()
                .as_str()
                .cmp(b.name().as_str())
                .then_with(|| a.id().value().cmp(&b.id().value()))
        });
        Ok(tags)
    }

    async fn list_tags_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
    ) -> Result<Vec<Tag>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let Some(card) = guard.cards.get(&card_id) else {
            return Err(StorageError::NotFound);
        };
        if card.deck_id() != deck_id {
            return Err(StorageError::NotFound);
        }

        let mut tags = Vec::new();
        if let Some(tag_ids) = guard.card_tags.get(&card_id) {
            for tag_id in tag_ids {
                if let Some(tag) = guard.tags.get(tag_id) {
                    tags.push(tag.clone());
                }
            }
        }
        tags.sort_by(|a, b| {
            a.name()
                .as_str()
                .cmp(b.name().as_str())
                .then_with(|| a.id().value().cmp(&b.id().value()))
        });
        Ok(tags)
    }

    async fn set_tags_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        tag_names: &[TagName],
    ) -> Result<Vec<Tag>, StorageError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let Some(card) = guard.cards.get(&card_id) else {
            return Err(StorageError::NotFound);
        };
        if card.deck_id() != deck_id {
            return Err(StorageError::NotFound);
        }

        let mut tag_ids = Vec::new();
        for name in tag_names {
            let existing = guard
                .tags
                .values()
                .find(|tag| tag.deck_id() == deck_id && tag.name() == name)
                .map(Tag::id);

            let tag_id = if let Some(id) = existing {
                id
            } else {
                let id = guard.next_tag_id;
                guard.next_tag_id = id
                    .checked_add(1)
                    .ok_or_else(|| StorageError::Serialization("tag_id overflow".into()))?;
                let tag_id = TagId::new(id);
                let tag = Tag::new(tag_id, deck_id, name.clone());
                guard.tags.insert(tag_id, tag);
                tag_id
            };

            if !tag_ids.contains(&tag_id) {
                tag_ids.push(tag_id);
            }
        }

        guard.card_tags.insert(card_id, tag_ids);
        let mut tags = Vec::new();
        if let Some(tag_ids) = guard.card_tags.get(&card_id) {
            for tag_id in tag_ids {
                if let Some(tag) = guard.tags.get(tag_id) {
                    tags.push(tag.clone());
                }
            }
        }
        Ok(tags)
    }
}

#[async_trait]
impl ReviewLogRepository for InMemoryRepository {
    async fn append_log(&self, mut log: ReviewLogRecord) -> Result<i64, StorageError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let id = guard.next_log_id;
        guard.next_log_id = id.saturating_add(1);
        log.id = Some(id);
        guard.logs.push(log);
        Ok(id)
    }

    async fn logs_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
    ) -> Result<Vec<ReviewLogRecord>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut logs: Vec<_> = guard
            .logs
            .iter()
            .filter(|log| log.deck_id == deck_id && log.card_id == card_id)
            .cloned()
            .collect();

        logs.sort_by_key(|l| l.reviewed_at);

        Ok(logs)
    }
}

#[async_trait]
impl ReviewPersistence for InMemoryRepository {
    async fn apply_review(
        &self,
        card: &Card,
        mut log: ReviewLogRecord,
    ) -> Result<i64, StorageError> {
        if log.card_id != card.id() || log.deck_id != card.deck_id() {
            return Err(StorageError::Conflict);
        }
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        guard.cards.insert(card.id(), card.clone());
        let id = guard.next_log_id;
        guard.next_log_id = id.saturating_add(1);
        log.id = Some(id);
        guard.logs.push(log);
        Ok(id)
    }
}

#[async_trait]
impl SessionSummaryRepository for InMemoryRepository {
    async fn append_summary(&self, summary: &SessionSummary) -> Result<i64, StorageError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        let id = guard.next_summary_id;
        guard.next_summary_id = id.saturating_add(1);
        guard.summaries.insert(id, summary.clone());
        Ok(id)
    }

    async fn get_summary(&self, id: i64) -> Result<SessionSummary, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        guard
            .summaries
            .get(&id)
            .cloned()
            .ok_or(StorageError::NotFound)
    }

    async fn list_summaries(
        &self,
        deck_id: DeckId,
        completed_from: Option<DateTime<Utc>>,
        completed_until: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummary>, StorageError> {
        let rows = self
            .list_summary_rows(deck_id, completed_from, completed_until, limit)
            .await?;
        Ok(rows.into_iter().map(|r| r.summary).collect())
    }

    async fn list_summary_rows(
        &self,
        deck_id: DeckId,
        completed_from: Option<DateTime<Utc>>,
        completed_until: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummaryRow>, StorageError> {
        let guard = self
            .state
            .lock()
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut rows: Vec<SessionSummaryRow> = guard
            .summaries
            .iter()
            .filter(|(_, summary)| summary.deck_id() == deck_id)
            .filter(|(_, summary)| completed_from.is_none_or(|from| summary.completed_at() >= from))
            .filter(|(_, summary)| {
                completed_until.is_none_or(|until| summary.completed_at() <= until)
            })
            .map(|(id, summary)| SessionSummaryRow::new(*id, summary.clone()))
            .collect();

        // Match SQLite ordering: completed_at DESC, id DESC
        rows.sort_by(|a, b| {
            b.summary
                .completed_at()
                .cmp(&a.summary.completed_at())
                .then_with(|| b.id.cmp(&a.id))
        });

        rows.truncate(limit_usize(limit));
        Ok(rows)
    }
}

/// Aggregates deck and card repositories behind trait objects for easy backend swapping.
#[derive(Clone)]
pub struct Storage {
    pub decks: Arc<dyn DeckRepository>,
    pub cards: Arc<dyn CardRepository>,
    pub review_logs: Arc<dyn ReviewLogRepository>,
    pub reviews: Arc<dyn ReviewPersistence>,
    pub session_summaries: Arc<dyn SessionSummaryRepository>,
}

impl Storage {
    #[must_use]
    pub fn in_memory() -> Self {
        let repo = InMemoryRepository::new();
        let decks: Arc<dyn DeckRepository> = Arc::new(repo.clone());
        let cards: Arc<dyn CardRepository> = Arc::new(repo.clone());
        let review_logs: Arc<dyn ReviewLogRepository> = Arc::new(repo.clone());
        let reviews: Arc<dyn ReviewPersistence> = Arc::new(repo.clone());
        let session_summaries: Arc<dyn SessionSummaryRepository> = Arc::new(repo);
        Self {
            decks,
            cards,
            review_logs,
            reviews,
            session_summaries,
        }
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

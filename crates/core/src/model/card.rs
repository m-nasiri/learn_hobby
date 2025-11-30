use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::{
    content::{Content, ContentValidationError},
    ids::{CardId, DeckId},
    review::ReviewOutcome,
};
use crate::scheduler::MemoryState;
//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CardError {
    #[error("invalid prompt content: {0}")]
    InvalidPrompt(#[source] ContentValidationError),

    #[error("invalid answer content: {0}")]
    InvalidAnswer(#[source] ContentValidationError),
}

//
// ─── CARD ──────────────────────────────────────────────────────────────────────
//

/// A flashcard with a prompt (question) and answer.
///
/// Cards are validated at construction to ensure both prompt and answer contain non-empty text.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    id: CardId,
    deck_id: DeckId,
    prompt: Content,
    answer: Content,
    created_at: DateTime<Utc>,
    next_review_at: DateTime<Utc>,
    last_review_at: Option<DateTime<Utc>>,
    review_count: u32,
    stability: f64,
    difficulty: f64,
}

impl Card {
    /// Creates a new Card.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this card
    /// * `deck_id` - ID of the deck this card belongs to
    /// * `prompt` - The question/prompt content (must have non-empty text)
    /// * `answer` - The answer content (must have non-empty text)
    /// * `created_at` - Timestamp when the card was created
    /// * `next_review_at` - Timestamp for the next review
    ///
    /// # Errors
    ///
    /// Returns `CardError::InvalidPrompt` if prompt text is empty.
    /// Returns `CardError::InvalidAnswer` if answer text is empty.
    ///
    /// Note: Content is already validated, so this performs additional domain-level validation.
    pub fn new(
        id: CardId,
        deck_id: DeckId,
        prompt: Content,
        answer: Content,
        created_at: DateTime<Utc>,
        next_review_at: DateTime<Utc>,
    ) -> Result<Self, CardError> {
        if prompt.text().trim().is_empty() {
            return Err(CardError::InvalidPrompt(ContentValidationError::EmptyText));
        }

        if answer.text().trim().is_empty() {
            return Err(CardError::InvalidAnswer(ContentValidationError::EmptyText));
        }

        Ok(Self {
            id,
            deck_id,
            prompt,
            answer,
            created_at,
            next_review_at,
            last_review_at: None,
            review_count: 0,
            stability: 0.0,
            difficulty: 0.0,
        })
    }

    // Accessors
    #[must_use]
    pub fn id(&self) -> CardId {
        self.id
    }

    #[must_use]
    pub fn deck_id(&self) -> DeckId {
        self.deck_id
    }

    #[must_use]
    pub fn prompt(&self) -> &Content {
        &self.prompt
    }

    #[must_use]
    pub fn answer(&self) -> &Content {
        &self.answer
    }

    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[must_use]
    pub fn next_review_at(&self) -> DateTime<Utc> {
        self.next_review_at
    }

    #[must_use]
    pub fn last_review_at(&self) -> Option<DateTime<Utc>> {
        self.last_review_at
    }

    #[must_use]
    pub fn review_count(&self) -> u32 {
        self.review_count
    }

    #[must_use]
    pub fn is_new(&self) -> bool {
        self.review_count == 0
    }

    #[must_use]
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        self.next_review_at <= now
    }

    #[must_use]
    pub fn memory_state(&self) -> Option<MemoryState> {
        if self.review_count == 0 {
            None
        } else {
            Some(MemoryState {
                stability: self.stability,
                difficulty: self.difficulty,
            })
        }
    }

    pub fn apply_review(&mut self, outcome: &ReviewOutcome, reviewed_at: DateTime<Utc>) {
        self.stability = outcome.stability;
        self.difficulty = outcome.difficulty;
        self.next_review_at = outcome.next_review;
        self.last_review_at = Some(reviewed_at);
        self.review_count = self.review_count.saturating_add(1);
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::content::ContentDraft;

    #[test]
    fn valid_card_creation() {
        let prompt = ContentDraft::text_only("What is 2+2?")
            .validate(Utc::now(), None, None)
            .unwrap();

        let answer = ContentDraft::text_only("4")
            .validate(Utc::now(), None, None)
            .unwrap();

        let now = Utc::now();
        let card = Card::new(CardId::new(10), DeckId::new(5), prompt, answer, now, now).unwrap();

        assert_eq!(card.id(), CardId::new(10));
        assert_eq!(card.deck_id(), DeckId::new(5));
        assert_eq!(card.prompt().text(), "What is 2+2?");
        assert_eq!(card.answer().text(), "4");
    }

    #[test]
    fn card_with_media() {
        use crate::model::content::{ImageMeta, MediaDraft, MediaUri};

        let media_uri = MediaUri::from_url("https://example.com/diagram.png").unwrap();
        let media_draft = MediaDraft::new_image(media_uri, None);
        let now = Utc::now();
        let meta = ImageMeta::new(800, 600).unwrap();

        let prompt = ContentDraft::with_media("Explain this diagram", media_draft)
            .validate(now, Some(meta), None)
            .unwrap();

        let answer = ContentDraft::text_only("This shows the water cycle")
            .validate(now, None, None)
            .unwrap();

        let card = Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap();

        assert!(card.prompt().has_media());
        assert!(!card.answer().has_media());
    }

    #[test]
    fn content_draft_rejects_empty_text() {
        // This test verifies ContentDraft validation prevents empty content
        let result = ContentDraft::text_only("").validate(Utc::now(), None, None);
        assert!(result.is_err());

        let result = ContentDraft::text_only("   ").validate(Utc::now(), None, None);
        assert!(result.is_err());
    }
}

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::{
    content::{Content, ContentValidationError},
    ids::{CardId, DeckId},
    review::{ReviewGrade, ReviewOutcome},
};
use crate::scheduler::MemoryState;

//
// ─── STATE MARKERS ─────────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct New;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Learning;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reviewing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Relearning;

/// Persist-able phase discriminator for cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CardPhase {
    New,
    Learning,
    Reviewing,
    Relearning,
}
//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CardError {
    #[error("invalid prompt content: {0}")]
    InvalidPrompt(#[source] ContentValidationError),

    #[error("invalid answer content: {0}")]
    InvalidAnswer(#[source] ContentValidationError),

    #[error("invalid persisted card state: {0}")]
    InvalidPersistedState(String),
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
    phase: CardPhase,
    created_at: DateTime<Utc>,
    next_review_at: DateTime<Utc>,
    last_review_at: Option<DateTime<Utc>>,
    review_count: u32,
    stability: f64,
    difficulty: f64,
}

/// Type-state wrapper for card lifecycle phases.
#[derive(Debug, Clone, PartialEq)]
pub struct CardState<S> {
    card: Card,
    state: std::marker::PhantomData<S>,
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
            phase: CardPhase::New,
            created_at,
            next_review_at,
            last_review_at: None,
            review_count: 0,
            stability: 0.0,
            difficulty: 0.0,
        })
    }

    /// Rehydrate a card from persisted state.
    ///
    /// # Errors
    ///
    /// Returns `CardError` if prompt/answer validation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn from_persisted(
        id: CardId,
        deck_id: DeckId,
        prompt: Content,
        answer: Content,
        created_at: DateTime<Utc>,
        next_review_at: DateTime<Utc>,
        last_review_at: Option<DateTime<Utc>>,
        phase: CardPhase,
        review_count: u32,
        stability: f64,
        difficulty: f64,
    ) -> Result<Self, CardError> {
        let mut card = Self::new(id, deck_id, prompt, answer, created_at, next_review_at)?;
        card.last_review_at = last_review_at;
        card.phase = phase;
        card.review_count = review_count;
        card.stability = stability;
        card.difficulty = difficulty;
        Ok(card)
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
    pub fn phase(&self) -> CardPhase {
        self.phase
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

    /// Apply a review outcome and advance phase based on the grade.
    ///
    /// Phase transition rules (can evolve with product needs):
    /// - `New` -> `Learning` on any grade.
    /// - `Learning` -> `Reviewing` on Hard/Good/Easy; stay `Learning` on Again.
    /// - `Reviewing` -> `Relearning` on Again; stay `Reviewing` otherwise.
    /// - `Relearning` -> `Reviewing` on Hard/Good/Easy; stay `Relearning` on Again.
    pub fn apply_review_with_phase(
        &mut self,
        grade: ReviewGrade,
        outcome: &ReviewOutcome,
        reviewed_at: DateTime<Utc>,
    ) {
        self.apply_review(outcome, reviewed_at);

        self.phase = match (self.phase, grade) {
            (CardPhase::New, _) | (CardPhase::Learning, ReviewGrade::Again) => CardPhase::Learning,
            (
                CardPhase::Learning | CardPhase::Reviewing | CardPhase::Relearning,
                ReviewGrade::Hard | ReviewGrade::Good | ReviewGrade::Easy,
            ) => CardPhase::Reviewing,
            (CardPhase::Reviewing | CardPhase::Relearning, ReviewGrade::Again) => {
                CardPhase::Relearning
            }
        };
    }
}

#[allow(dead_code)]
impl CardState<New> {
    #[must_use]
    pub fn new(card: Card) -> Self {
        Self {
            card,
            state: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn card(&self) -> &Card {
        &self.card
    }

    #[must_use]
    pub fn from_persisted(card: Card) -> Self {
        debug_assert_eq!(card.phase(), CardPhase::New);
        Self::new(card)
    }

    #[must_use]
    pub fn into_inner(self) -> Card {
        self.card
    }

    #[must_use]
    pub fn phase(&self) -> CardPhase {
        self.card.phase
    }

    #[must_use]
    pub fn start_learning(self) -> CardState<Learning> {
        let mut card = self.card;
        card.phase = CardPhase::Learning;
        CardState {
            card,
            state: std::marker::PhantomData,
        }
    }
}

#[allow(dead_code)]
impl CardState<Learning> {
    #[must_use]
    pub fn card(&self) -> &Card {
        &self.card
    }

    #[must_use]
    pub fn card_mut(&mut self) -> &mut Card {
        &mut self.card
    }
    #[must_use]
    pub fn from_persisted(card: Card) -> Self {
        debug_assert_eq!(card.phase(), CardPhase::Learning);
        Self {
            card,
            state: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn into_inner(self) -> Card {
        self.card
    }

    #[must_use]
    pub fn phase(&self) -> CardPhase {
        self.card.phase
    }

    #[must_use]
    pub fn graduate(self) -> CardState<Reviewing> {
        let mut card = self.card;
        card.phase = CardPhase::Reviewing;
        CardState {
            card,
            state: std::marker::PhantomData,
        }
    }
}

#[allow(dead_code)]
impl CardState<Reviewing> {
    #[must_use]
    pub fn card(&self) -> &Card {
        &self.card
    }

    #[must_use]
    pub fn card_mut(&mut self) -> &mut Card {
        &mut self.card
    }
    #[must_use]
    pub fn from_persisted(card: Card) -> Self {
        debug_assert_eq!(card.phase(), CardPhase::Reviewing);
        Self {
            card,
            state: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn into_inner(self) -> Card {
        self.card
    }

    #[must_use]
    pub fn phase(&self) -> CardPhase {
        self.card.phase
    }

    #[must_use]
    pub fn lapse(self) -> CardState<Relearning> {
        let mut card = self.card;
        card.phase = CardPhase::Relearning;
        CardState {
            card,
            state: std::marker::PhantomData,
        }
    }

    pub fn apply_review_outcome(
        mut self,
        outcome: &ReviewOutcome,
        reviewed_at: DateTime<Utc>,
    ) -> CardState<Reviewing> {
        // Use the central phase logic: for a reviewing card, any non-Again
        // grade keeps it in Reviewing. We treat this helper as a successful
        // review, so we map it to `Good`.
        self.card
            .apply_review_with_phase(ReviewGrade::Good, outcome, reviewed_at);
        self
    }
}

#[allow(dead_code)]
impl CardState<Relearning> {
    #[must_use]
    pub fn card(&self) -> &Card {
        &self.card
    }

    #[must_use]
    pub fn card_mut(&mut self) -> &mut Card {
        &mut self.card
    }
    #[must_use]
    pub fn from_persisted(card: Card) -> Self {
        debug_assert_eq!(card.phase(), CardPhase::Relearning);
        Self {
            card,
            state: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn into_inner(self) -> Card {
        self.card
    }

    #[must_use]
    pub fn phase(&self) -> CardPhase {
        self.card.phase
    }

    #[must_use]
    pub fn regraduate(self) -> CardState<Reviewing> {
        let mut card = self.card;
        card.phase = CardPhase::Reviewing;
        CardState {
            card,
            state: std::marker::PhantomData,
        }
    }
}

impl CardPhase {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CardPhase::New => "new",
            CardPhase::Learning => "learning",
            CardPhase::Reviewing => "reviewing",
            CardPhase::Relearning => "relearning",
        }
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::content::ContentDraft;
    use crate::time::fixed_now;

    #[test]
    fn valid_card_creation() {
        let prompt = ContentDraft::text_only("What is 2+2?")
            .validate(fixed_now(), None, None)
            .unwrap();

        let answer = ContentDraft::text_only("4")
            .validate(fixed_now(), None, None)
            .unwrap();

        let now = fixed_now();
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
        let now = fixed_now();
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
        let result = ContentDraft::text_only("").validate(fixed_now(), None, None);
        assert!(result.is_err());

        let result = ContentDraft::text_only("   ").validate(fixed_now(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn typestate_transitions_preserve_phase() {
        let prompt = ContentDraft::text_only("Q")
            .validate(fixed_now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("A")
            .validate(fixed_now(), None, None)
            .unwrap();
        let now = fixed_now();
        let card = Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap();

        let new_state = CardState::<New>::new(card);
        assert_eq!(new_state.phase(), CardPhase::New);

        let learning = new_state.start_learning();
        assert_eq!(learning.phase(), CardPhase::Learning);

        let reviewing = learning.graduate();
        assert_eq!(reviewing.phase(), CardPhase::Reviewing);

        let relearning = reviewing.lapse();
        assert_eq!(relearning.phase(), CardPhase::Relearning);

        let back = relearning.regraduate();
        assert_eq!(back.phase(), CardPhase::Reviewing);
    }

    #[test]
    fn applying_outcome_updates_card_across_states() {
        let prompt = ContentDraft::text_only("Q")
            .validate(fixed_now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("A")
            .validate(fixed_now(), None, None)
            .unwrap();
        let now = fixed_now();
        let outcome = ReviewOutcome::new(now + chrono::Duration::days(1), 1.0, 2.0, 0.0, 1.0);

        let mut card = Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap();

        card.apply_review_with_phase(ReviewGrade::Good, &outcome, now);
        assert_eq!(card.review_count(), 1);
        assert_eq!(card.last_review_at(), Some(now));
        assert_eq!(card.phase(), CardPhase::Learning);

        card.apply_review_with_phase(ReviewGrade::Good, &outcome, now);
        assert_eq!(card.review_count(), 2);
        assert_eq!(card.phase(), CardPhase::Reviewing);

        card.apply_review_with_phase(ReviewGrade::Again, &outcome, now);
        assert_eq!(card.review_count(), 3);
        assert_eq!(card.phase(), CardPhase::Relearning);

        card.apply_review_with_phase(ReviewGrade::Hard, &outcome, now);
        assert_eq!(card.review_count(), 4);
        assert_eq!(card.phase(), CardPhase::Reviewing);
    }

    #[test]
    fn card_phase_as_str() {
        assert_eq!(CardPhase::New.as_str(), "new");
        assert_eq!(CardPhase::Learning.as_str(), "learning");
        assert_eq!(CardPhase::Reviewing.as_str(), "reviewing");
        assert_eq!(CardPhase::Relearning.as_str(), "relearning");
    }

    #[test]
    fn apply_review_with_phase_changes_phase_by_grade() {
        let prompt = ContentDraft::text_only("Q")
            .validate(fixed_now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("A")
            .validate(fixed_now(), None, None)
            .unwrap();
        let now = fixed_now();
        let outcome = ReviewOutcome::new(now + chrono::Duration::days(1), 1.0, 2.0, 0.0, 1.0);
        let mut card = Card::new(CardId::new(1), DeckId::new(1), prompt, answer, now, now).unwrap();

        assert_eq!(card.phase(), CardPhase::New);
        card.apply_review_with_phase(ReviewGrade::Good, &outcome, now);
        assert_eq!(card.phase(), CardPhase::Learning);

        card.apply_review_with_phase(ReviewGrade::Good, &outcome, now);
        assert_eq!(card.phase(), CardPhase::Reviewing);

        card.apply_review_with_phase(ReviewGrade::Again, &outcome, now);
        assert_eq!(card.phase(), CardPhase::Relearning);

        card.apply_review_with_phase(ReviewGrade::Hard, &outcome, now);
        assert_eq!(card.phase(), CardPhase::Reviewing);
    }
}

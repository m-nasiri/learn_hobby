use chrono::{DateTime, Utc};
use thiserror::Error;
use std::fmt;

use learn_core::{
    model::{Card, CardId, Deck, DeckId, ReviewGrade},
    scheduler::Scheduler,
};

use crate::review_service::{ReviewResult, ReviewService, ReviewServiceError};

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SessionError {
    #[error("no cards available for session")]
    Empty,
    #[error("session already completed")]
    Completed,
    #[error(transparent)]
    Review(#[from] ReviewServiceError),
}

//
// ─── REVIEW RESULT WITH CARD ───────────────────────────────────────────────────
//

/// Captures the outcome of reviewing a card within a session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionReview {
    pub card_id: CardId,
    pub result: ReviewResult,
}

//
// ─── SESSION ───────────────────────────────────────────────────────────────────
//

/// In-memory micro-session for a deck.
///
/// Selects up to `micro_session_size` cards from the provided list and steps through
/// them sequentially, applying grades via `ReviewService`.
pub struct SessionService {
    deck_id: DeckId,
    cards: Vec<Card>,
    current: usize,
    results: Vec<SessionReview>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    review_service: ReviewService,
}

impl SessionService {
    /// Create a new session for the given deck, selecting up to `micro_session_size` cards.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are provided.
    /// Propagates scheduler/review errors via `SessionError::Review`.
    pub fn new(
        deck: &Deck,
        mut cards: Vec<Card>,
        review_service: ReviewService,
    ) -> Result<Self, SessionError> {
        let limit = deck.settings().micro_session_size() as usize;
        cards.truncate(limit);

        if cards.is_empty() {
            return Err(SessionError::Empty);
        }

        Ok(Self {
            deck_id: deck.id(),
            cards,
            current: 0,
            results: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            review_service,
        })
    }

    /// Convenience constructor with a default scheduler-backed review service.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are provided.
    /// Propagates scheduler/review errors via `SessionError::Review`.
    pub fn with_scheduler(
        deck: &Deck,
        cards: Vec<Card>,
        scheduler: Scheduler,
    ) -> Result<Self, SessionError> {
        Self::new(deck, cards, ReviewService::with_scheduler(scheduler))
    }

    #[must_use]
    pub fn deck_id(&self) -> DeckId {
        self.deck_id
    }

    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    #[must_use]
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    #[must_use]
    pub fn results(&self) -> &[SessionReview] {
        &self.results
    }

    /// Total number of cards in this session.
    #[must_use]
    pub fn total_cards(&self) -> usize {
        self.cards.len()
    }

    /// Number of cards that have already been answered.
    #[must_use]
    pub fn answered_count(&self) -> usize {
        self.results.len()
    }

    /// Number of remaining cards that have not been answered yet.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.cards.len().saturating_sub(self.current)
    }

    #[must_use]
    pub fn current_card(&self) -> Option<&Card> {
        if self.current < self.cards.len() {
            Some(&self.cards[self.current])
        } else {
            None
        }
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.completed_at.is_some()
    }

    /// Apply a grade to the current card and advance the session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Completed` if the session is already finished.
    /// Propagates scheduler/review errors via `SessionError::Review`.
    pub fn answer_current(
        &mut self,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
    ) -> Result<&SessionReview, SessionError> {
        if self.is_complete() {
            return Err(SessionError::Completed);
        }

        let Some(card) = self.cards.get_mut(self.current) else {
            return Err(SessionError::Completed);
        };

        let result = self.review_service.review_card(card, grade, reviewed_at)?;

        self.results.push(SessionReview {
            card_id: card.id(),
            result,
        });

        self.current += 1;
        if self.current >= self.cards.len() {
            self.completed_at = Some(reviewed_at);
        }

        self.results
            .last()
            .ok_or(SessionError::Completed)
    }
}

impl fmt::Debug for SessionService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionService")
            .field("deck_id", &self.deck_id)
            .field("cards_len", &self.cards.len())
            .field("current", &self.current)
            .field("results_len", &self.results.len())
            .field("started_at", &self.started_at)
            .field("completed_at", &self.completed_at)
            .finish_non_exhaustive()
    }
}

impl Default for SessionService {
    fn default() -> Self {
        Self {
            deck_id: DeckId::new(0),
            cards: Vec::new(),
            current: 0,
            results: Vec::new(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            review_service: ReviewService::new(),
        }
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::model::{DeckId, content::ContentDraft};

    fn build_card(id: u64) -> Card {
        let prompt = ContentDraft::text_only("Q")
            .validate(Utc::now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("A")
            .validate(Utc::now(), None, None)
            .unwrap();
        let now = Utc::now();
        Card::new(
            learn_core::model::CardId::new(id),
            DeckId::new(1),
            prompt,
            answer,
            now,
            now,
        )
        .unwrap()
    }

    fn build_deck() -> Deck {
        Deck::new(
            DeckId::new(1),
            "Test",
            None,
            learn_core::model::DeckSettings::default_for_adhd(),
            Utc::now(),
        )
        .unwrap()
    }

    #[test]
    fn session_honors_micro_session_size() {
        let deck = build_deck();
        let cards = vec![build_card(1), build_card(2), build_card(3)];
        let session = SessionService::new(&deck, cards, ReviewService::new()).unwrap();

        let expected = deck.settings().micro_session_size().min(3) as usize;
        assert_eq!(session.cards.len(), expected);
    }

    #[test]
    fn empty_session_returns_error() {
        let deck = build_deck();
        let err = SessionService::new(&deck, Vec::new(), ReviewService::new()).unwrap_err();
        assert!(matches!(err, SessionError::Empty));
    }

    #[test]
    fn session_advances_and_completes() {
        let deck = build_deck();
        let mut session = SessionService::new(
            &deck,
            vec![build_card(1), build_card(2)],
            ReviewService::new(),
        )
        .unwrap();

        assert!(!session.is_complete());
        let first_card_id = session.current_card().unwrap().id();
        let t1 = Utc::now();
        let res1 = session.answer_current(ReviewGrade::Good, t1).unwrap();
        assert_eq!(res1.card_id, first_card_id);
        assert_eq!(session.results.len(), 1);
        assert!(!session.is_complete());

        let second_card_id = session.current_card().unwrap().id();
        let t2 = t1 + chrono::Duration::minutes(1);
        let res2 = session.answer_current(ReviewGrade::Hard, t2).unwrap();
        assert_eq!(res2.card_id, second_card_id);
        assert!(session.is_complete());
        assert_eq!(session.completed_at(), Some(t2));
    }
}

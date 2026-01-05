use chrono::{DateTime, Utc};
use std::fmt;
use learn_core::model::{Card, CardId, Deck, DeckId, DeckSettings, ReviewGrade, SessionSummary};

use crate::error::SessionError;
use crate::review_service::{ReviewResult, ReviewService};
use super::progress::SessionProgress;

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
    deck_settings: DeckSettings,
    cards: Vec<Card>,
    current: usize,
    results: Vec<SessionReview>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    summary_id: Option<i64>,
}

impl SessionService {
    /// Create a new session for the given deck, selecting up to `micro_session_size` cards.
    ///
    /// `started_at` should come from the services layer clock to keep time deterministic.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are provided.
    pub fn new(deck: &Deck, cards: Vec<Card>, started_at: DateTime<Utc>) -> Result<Self, SessionError> {
        Self::new_with_limit(deck, cards, started_at, Some(deck.settings().micro_session_size()))
    }

    pub(crate) fn new_all(
        deck: &Deck,
        cards: Vec<Card>,
        started_at: DateTime<Utc>,
    ) -> Result<Self, SessionError> {
        Self::new_with_limit(deck, cards, started_at, None)
    }

    fn new_with_limit(
        deck: &Deck,
        mut cards: Vec<Card>,
        started_at: DateTime<Utc>,
        limit: Option<u32>,
    ) -> Result<Self, SessionError> {
        if let Some(limit) = limit {
            let limit = usize::try_from(limit).unwrap_or(usize::MAX);
            cards.truncate(limit);
        }

        if cards.is_empty() {
            return Err(SessionError::Empty);
        }

        Ok(Self {
            deck_id: deck.id(),
            deck_settings: deck.settings().clone(),
            cards,
            current: 0,
            results: Vec::new(),
            started_at,
            completed_at: None,
            summary_id: None,
        })
    }

    #[must_use]
    pub fn deck_id(&self) -> DeckId {
        self.deck_id
    }

    #[must_use]
    pub fn deck_settings(&self) -> &DeckSettings {
        &self.deck_settings
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
    pub fn summary_id(&self) -> Option<i64> {
        self.summary_id
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

    /// Returns a summary of the current session progress.
    #[must_use]
    pub fn progress(&self) -> SessionProgress {
        SessionProgress {
            total: self.total_cards(),
            answered: self.answered_count(),
            remaining: self.remaining(),
            is_complete: self.is_complete(),
        }
    }

    #[must_use]
    pub fn current_card(&self) -> Option<&Card> {
        if self.current < self.cards.len() {
            Some(&self.cards[self.current])
        } else {
            None
        }
    }

    pub(crate) fn current_card_mut(&mut self) -> Option<&mut Card> {
        if self.current < self.cards.len() {
            Some(&mut self.cards[self.current])
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
    /// `reviewed_at` should come from the services layer clock.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Completed` if the session is already finished.
    /// Propagates scheduler/review errors via `SessionError::Review`.
    pub fn answer_current(
        &mut self,
        review_service: &ReviewService,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
    ) -> Result<&SessionReview, SessionError> {
        let deck_settings = self.deck_settings.clone();
        let (card_id, result) = {
            let Some(card) = self.current_card_mut() else {
                return Err(SessionError::Completed);
            };
            let result = review_service.review_card_with_settings(
                card,
                grade,
                reviewed_at,
                &deck_settings,
            )?;
            (card.id(), result)
        };

        self.record_review_result(card_id, result, reviewed_at)
    }

    pub(crate) fn record_review_result(
        &mut self,
        card_id: CardId,
        result: ReviewResult,
        reviewed_at: DateTime<Utc>,
    ) -> Result<&SessionReview, SessionError> {
        if self.is_complete() {
            return Err(SessionError::Completed);
        }

        self.results.push(SessionReview { card_id, result });

        self.current += 1;
        if self.current >= self.cards.len() {
            self.completed_at = Some(reviewed_at);
        }

        self.results.last().ok_or(SessionError::Completed)
    }

    pub(crate) fn build_summary(
        &self,
        completed_at: DateTime<Utc>,
    ) -> Result<SessionSummary, SessionError> {
        let logs: Vec<_> = self
            .results
            .iter()
            .map(|review| review.result.applied.log.clone())
            .collect();
        Ok(SessionSummary::from_logs(
            self.deck_id,
            self.started_at,
            completed_at,
            &logs,
        )?)
    }

    pub(crate) fn set_summary_id(&mut self, id: i64) {
        self.summary_id = Some(id);
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
            .field("summary_id", &self.summary_id)
            .finish_non_exhaustive()
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::Clock;
    use learn_core::model::{CardPhase, DeckId, content::ContentDraft};
    use learn_core::time::fixed_now;

    fn build_card(id: u64) -> Card {
        let prompt = ContentDraft::text_only("Q")
            .validate(fixed_now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("A")
            .validate(fixed_now(), None, None)
            .unwrap();
        let now = fixed_now();
        Card::new(CardId::new(id), DeckId::new(1), prompt, answer, now, now).unwrap()
    }

    fn build_deck() -> Deck {
        Deck::new(
            DeckId::new(1),
            "Test",
            None,
            learn_core::model::DeckSettings::default_for_adhd(),
            fixed_now(),
        )
        .unwrap()
    }

    #[test]
    fn session_honors_micro_session_size() {
        let deck = build_deck();
        let cards = vec![build_card(1), build_card(2), build_card(3)];
        let session = SessionService::new(&deck, cards, fixed_now()).unwrap();

        let expected = deck.settings().micro_session_size().min(3) as usize;
        assert_eq!(session.cards.len(), expected);
    }

    #[test]
    fn empty_session_returns_error() {
        let deck = build_deck();
        let err = SessionService::new(&deck, Vec::new(), fixed_now()).unwrap_err();
        assert!(matches!(err, SessionError::Empty));
    }

    #[test]
    fn session_advances_and_completes() {
        let deck = build_deck();
        let mut session =
            SessionService::new(&deck, vec![build_card(1), build_card(2)], fixed_now()).unwrap();
        let review_service = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(fixed_now()));

        assert!(!session.is_complete());
        let first_card_id = session.current_card().unwrap().id();
        let res1 = session
            .answer_current(&review_service, ReviewGrade::Good, fixed_now())
            .unwrap();
        assert_eq!(res1.card_id, first_card_id);
        assert_eq!(session.results.len(), 1);
        assert!(!session.is_complete());

        let second_card_id = session.current_card().unwrap().id();
        let res2 = session
            .answer_current(&review_service, ReviewGrade::Hard, fixed_now())
            .unwrap();
        assert_eq!(res2.card_id, second_card_id);
        assert!(session.is_complete());
        assert_eq!(session.completed_at(), Some(fixed_now()));
    }

    #[test]
    fn integration_session_runs_with_review_logs_and_phase_updates() {
        let deck = build_deck();
        let mut session =
            SessionService::new(&deck, vec![build_card(1), build_card(2)], fixed_now()).unwrap();
        let review_service = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(fixed_now()));

        session
            .answer_current(&review_service, ReviewGrade::Good, fixed_now())
            .unwrap();
        let first = session.results.last().unwrap();
        assert_eq!(first.card_id, session.results[0].card_id);
        assert_eq!(first.result.applied.log.grade, ReviewGrade::Good);
        assert_eq!(session.cards[0].review_count(), 1);
        assert_eq!(session.cards[0].phase(), CardPhase::Learning);

        session
            .answer_current(&review_service, ReviewGrade::Hard, fixed_now())
            .unwrap();
        let second = session.results.last().unwrap();
        assert_eq!(second.card_id, session.results[1].card_id);
        assert_eq!(second.result.applied.log.grade, ReviewGrade::Hard);
        assert_eq!(session.cards[1].review_count(), 1);
        assert_eq!(session.cards[1].phase(), CardPhase::Learning);

        assert!(session.is_complete());
        assert_eq!(session.results.len(), 2);
    }
}

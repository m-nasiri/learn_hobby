use std::sync::Arc;

use learn_core::model::{DeckId, ReviewGrade, TagName};
use storage::repository::{
    CardRepository, DeckRepository, ReviewPersistence, SessionSummaryRepository,
};

use crate::review_service::ReviewService;
use crate::Clock;
use super::queries::SessionQueries;
use crate::error::SessionError;
use super::service::{SessionReview, SessionService};

/// Result of answering a single card in a session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionAnswerResult {
    pub review: SessionReview,
    pub is_complete: bool,
    pub summary_id: Option<i64>,
}

/// Orchestrates session start and persisted answering.
#[derive(Clone)]
pub struct SessionLoopService {
    clock: Clock,
    decks: Arc<dyn DeckRepository>,
    cards: Arc<dyn CardRepository>,
    reviews: Arc<dyn ReviewPersistence>,
    summaries: Arc<dyn SessionSummaryRepository>,
    shuffle_new: bool,
}

impl SessionLoopService {
    #[must_use]
    pub fn new(
        clock: Clock,
        decks: Arc<dyn DeckRepository>,
        cards: Arc<dyn CardRepository>,
        reviews: Arc<dyn ReviewPersistence>,
        summaries: Arc<dyn SessionSummaryRepository>,
    ) -> Self {
        Self {
            clock,
            decks,
            cards,
            reviews,
            summaries,
            shuffle_new: false,
        }
    }

    #[must_use]
    pub fn with_shuffle_new(mut self, shuffle_new: bool) -> Self {
        self.shuffle_new = shuffle_new;
        self
    }

    /// Start a new session for the given deck.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` for storage or session start failures.
    pub async fn start_session(&self, deck_id: DeckId) -> Result<SessionService, SessionError> {
        let now = self.clock.now();
        let (_deck, session) = SessionQueries::start_from_storage(
            deck_id,
            self.decks.as_ref(),
            self.cards.as_ref(),
            now,
            self.shuffle_new,
        )
        .await?;
        Ok(session)
    }

    /// Start a new session including all cards in the deck.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` for storage or session start failures.
    pub async fn start_session_all_cards(
        &self,
        deck_id: DeckId,
    ) -> Result<SessionService, SessionError> {
        let now = self.clock.now();
        let (_deck, session) = SessionQueries::start_from_storage_all_cards(
            deck_id,
            self.decks.as_ref(),
            self.cards.as_ref(),
            now,
        )
        .await?;
        Ok(session)
    }

    /// Start a new session from cards currently in relearning (mistakes).
    ///
    /// # Errors
    ///
    /// Returns `SessionError` for storage or session start failures.
    pub async fn start_session_mistakes(
        &self,
        deck_id: DeckId,
    ) -> Result<SessionService, SessionError> {
        let now = self.clock.now();
        let (_deck, session) = SessionQueries::start_from_storage_mistakes(
            deck_id,
            self.decks.as_ref(),
            self.cards.as_ref(),
            now,
        )
        .await?;
        Ok(session)
    }

    /// Start a new session for the given deck filtered by tags.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` for storage or session start failures.
    pub async fn start_session_with_tags(
        &self,
        deck_id: DeckId,
        tag_names: &[TagName],
    ) -> Result<SessionService, SessionError> {
        let now = self.clock.now();
        let (_deck, session) = SessionQueries::start_from_storage_with_tags(
            deck_id,
            self.decks.as_ref(),
            self.cards.as_ref(),
            now,
            self.shuffle_new,
            tag_names,
        )
        .await?;
        Ok(session)
    }

    /// Answer the current card and persist review + summary when completed.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` for review or persistence failures.
    pub async fn answer_current(
        &self,
        session: &mut SessionService,
        grade: ReviewGrade,
    ) -> Result<SessionAnswerResult, SessionError> {
        let review_service = ReviewService::new()?.with_clock(self.clock);
        let reviewed_at = self.clock.now();
        let deck_settings = session.deck_settings().clone();
        let Some(card) = session.current_card_mut() else {
            return Err(SessionError::Completed);
        };

        let card_id = card.id();
        let (result, _log_id) = review_service
            .review_card_persisted_with_settings(
                card,
                grade,
                reviewed_at,
                &deck_settings,
                self.reviews.as_ref(),
            )
            .await?;
        let review = session
            .record_review_result(card_id, result, reviewed_at)?
            .clone();

        if session.is_complete() && session.summary_id().is_none() {
            let completed_at = session.completed_at().ok_or(SessionError::Completed)?;
            let summary = session.build_summary(completed_at)?;
            let summary_id = self.summaries.append_summary(&summary).await?;
            session.set_summary_id(summary_id);
        }

        Ok(SessionAnswerResult {
            review,
            is_complete: session.is_complete(),
            summary_id: session.summary_id(),
        })
    }

    /// Retry summary persistence after a completed session.
    ///
    /// This is useful when the final summary append failed (e.g. transient storage error).
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Completed` if the session is not complete or missing timestamps.
    /// Returns `SessionError::Storage` if persistence fails.
    pub async fn finalize_summary(&self, session: &mut SessionService) -> Result<i64, SessionError> {
        if let Some(id) = session.summary_id() {
            return Ok(id);
        }

        if !session.is_complete() {
            return Err(SessionError::Completed);
        }

        let completed_at = session.completed_at().ok_or(SessionError::Completed)?;
        let summary = session.build_summary(completed_at)?;
        let id = self.summaries.append_summary(&summary).await?;
        session.set_summary_id(id);
        Ok(id)
    }
}

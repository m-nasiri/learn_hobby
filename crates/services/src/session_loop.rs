use std::sync::Arc;

use learn_core::model::{DeckId, ReviewGrade};
use storage::repository::{
    CardRepository, DeckRepository, ReviewPersistence, SessionSummaryRepository,
};

use crate::session_service::{SessionError, SessionReview, SessionService};
use crate::{Clock, ReviewService};

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
    decks: Arc<dyn DeckRepository + Send + Sync>,
    cards: Arc<dyn CardRepository + Send + Sync>,
    reviews: Arc<dyn ReviewPersistence + Send + Sync>,
    summaries: Arc<dyn SessionSummaryRepository + Send + Sync>,
    shuffle_new: bool,
}

impl SessionLoopService {
    #[must_use]
    pub fn new(
        clock: Clock,
        decks: Arc<dyn DeckRepository + Send + Sync>,
        cards: Arc<dyn CardRepository + Send + Sync>,
        reviews: Arc<dyn ReviewPersistence + Send + Sync>,
        summaries: Arc<dyn SessionSummaryRepository + Send + Sync>,
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
        let review_service = ReviewService::new()?.with_clock(self.clock);
        let (_deck, session) = SessionService::start_session(
            deck_id,
            self.decks.as_ref(),
            self.cards.as_ref(),
            self.shuffle_new,
            review_service,
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
        let review = session
            .answer_current_persisted(grade, self.reviews.as_ref(), self.summaries.as_ref())
            .await?
            .clone();

        Ok(SessionAnswerResult {
            review,
            is_complete: session.is_complete(),
            summary_id: session.summary_id(),
        })
    }
}

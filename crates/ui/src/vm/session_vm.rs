use learn_core::model::{DeckId, ReviewGrade};
use services::{SessionLoopService, SessionService};

use crate::views::ViewError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionIntent {
    Reveal,
    Grade(ReviewGrade),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionPhase {
    Prompt,
    Answer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionOutcome {
    Continue,
    Completed { summary_id: Option<i64> },
}

pub struct SessionVm {
    session: SessionService,
    phase: SessionPhase,
}

impl SessionVm {
    #[must_use]
    pub fn new(session: SessionService) -> Self {
        Self {
            session,
            phase: SessionPhase::Prompt,
        }
    }

    #[must_use]
    pub fn phase(&self) -> SessionPhase {
        self.phase
    }

    pub fn reveal(&mut self) {
        self.phase = SessionPhase::Answer;
    }

    #[must_use]
    pub fn prompt_text(&self) -> Option<&str> {
        self.session.current_card().map(|card| card.prompt().text())
    }

    #[must_use]
    pub fn answer_text(&self) -> Option<&str> {
        self.session.current_card().map(|card| card.answer().text())
    }

    #[must_use]
    pub fn has_card(&self) -> bool {
        self.session.current_card().is_some()
    }

    /// # Errors
    ///
    /// Returns `ViewError::Unknown` for service failures.
    pub async fn answer_current(
        &mut self,
        session_loop: &SessionLoopService,
        grade: ReviewGrade,
    ) -> Result<SessionOutcome, ViewError> {
        let result = session_loop
            .answer_current(&mut self.session, grade)
            .await
            .map_err(|_| ViewError::Unknown)?;

        if result.is_complete {
            return Ok(SessionOutcome::Completed {
                summary_id: result.summary_id,
            });
        }

        self.phase = SessionPhase::Prompt;
        Ok(SessionOutcome::Continue)
    }
}

/// # Errors
///
/// Returns `ViewError::EmptySession` when no cards are available.
/// Returns `ViewError::Unknown` for other failures.
pub async fn start_session(
    session_loop: &SessionLoopService,
    deck_id: DeckId,
) -> Result<SessionVm, ViewError> {
    let session = match session_loop.start_session(deck_id).await {
        Ok(session) => session,
        Err(services::SessionError::Empty) => return Err(ViewError::EmptySession),
        Err(_) => return Err(ViewError::Unknown),
    };

    Ok(SessionVm::new(session))
}

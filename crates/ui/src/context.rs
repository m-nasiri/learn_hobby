use std::sync::Arc;

use learn_core::model::DeckId;
use services::SessionSummaryService;

pub trait UiApp: Send + Sync {
    fn current_deck_id(&self) -> DeckId;

    fn session_summaries(&self) -> Arc<SessionSummaryService>;
}

#[derive(Clone)]
pub struct AppContext {
    app: Arc<dyn UiApp>,
}

impl AppContext {
    #[must_use]
    pub fn new(app: Arc<dyn UiApp>) -> Self {
        Self { app }
    }

    #[must_use]
    pub fn app(&self) -> Arc<dyn UiApp> {
        Arc::clone(&self.app)
    }

    #[must_use]
    pub fn current_deck_id(&self) -> DeckId {
        self.app.as_ref().current_deck_id()
    }

    #[must_use]
    pub fn session_summaries(&self) -> Arc<SessionSummaryService> {
        self.app.as_ref().session_summaries()
    }
}

// This context is provided by the application composition root (e.g. `crates/app`).

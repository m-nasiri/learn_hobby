use std::sync::Arc;

use learn_core::model::DeckId;
use services::{SessionLoopService, SessionSummaryService};

pub trait UiApp: Send + Sync {
    fn current_deck_id(&self) -> DeckId;
    fn open_editor_on_launch(&self) -> bool;

    fn session_summaries(&self) -> Arc<SessionSummaryService>;
    fn session_loop(&self) -> Arc<SessionLoopService>;
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
    pub fn open_editor_on_launch(&self) -> bool {
        self.app.as_ref().open_editor_on_launch()
    }

    #[must_use]
    pub fn session_summaries(&self) -> Arc<SessionSummaryService> {
        self.app.as_ref().session_summaries()
    }

    #[must_use]
    pub fn session_loop(&self) -> Arc<SessionLoopService> {
        self.app.as_ref().session_loop()
    }
}

// This context is provided by the application composition root (e.g. `crates/app`).

/// Build an `AppContext` from a UI-facing app implementation.
#[must_use]
pub fn build_app_context(app: Arc<dyn UiApp>) -> AppContext {
    AppContext::new(app)
}

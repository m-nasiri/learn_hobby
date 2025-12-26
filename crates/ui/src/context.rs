use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

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
    current_deck_id: DeckId,
    open_editor_on_launch_configured: bool,
    open_editor_on_launch_once: Arc<AtomicBool>,

    session_summaries: Arc<SessionSummaryService>,
    session_loop: Arc<SessionLoopService>,
}

impl AppContext {
    #[must_use]
    pub fn new(app: &Arc<dyn UiApp>) -> Self {
        let current_deck_id = app.current_deck_id();
        let open_editor_on_launch_configured = app.open_editor_on_launch();

        let session_summaries = app.session_summaries();
        let session_loop = app.session_loop();

        Self {
            current_deck_id,
            open_editor_on_launch_configured,
            open_editor_on_launch_once: Arc::new(AtomicBool::new(open_editor_on_launch_configured)),
            session_summaries,
            session_loop,
        }
    }

    #[must_use]
    pub fn current_deck_id(&self) -> DeckId {
        self.current_deck_id
    }

    #[must_use]
    pub fn take_open_editor_on_launch(&self) -> bool {
        self.open_editor_on_launch_once
            .swap(false, Ordering::AcqRel)
    }

    /// The configured value (not the one-shot value). Useful for diagnostics/UI.
    #[must_use]
    pub fn open_editor_on_launch_configured(&self) -> bool {
        self.open_editor_on_launch_configured
    }

    #[must_use]
    pub fn session_summaries(&self) -> Arc<SessionSummaryService> {
        Arc::clone(&self.session_summaries)
    }

    #[must_use]
    pub fn session_loop(&self) -> Arc<SessionLoopService> {
        Arc::clone(&self.session_loop)
    }
}

// This context is provided by the application composition root (e.g. `crates/app`).

/// Build an `AppContext` from a UI-facing app implementation.
#[must_use]
pub fn build_app_context(app: &Arc<dyn UiApp>) -> AppContext {
    AppContext::new(app)
}

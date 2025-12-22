use std::sync::Arc;

use dioxus::prelude::*;

/// What the UI is allowed to do.
/// Keep this small; expand only when a screen needs it.
pub trait UiApp: Send + Sync {
    fn app_name(&self) -> &'static str;
}

/// `AppContext` exposed via Dioxus context.
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
    pub fn app(&self) -> &dyn UiApp {
        self.app.as_ref()
    }
}

/// Provide the `AppContext` once at the app root.
pub fn provide_app_context() {
    // For Step 1: a stub implementation.
    // Next steps: construct real services here (or in a builder) and wrap them.
    let app: Arc<dyn UiApp> = Arc::new(StubUiApp);

    use_context_provider(|| AppContext::new(app));
}

// ─── Stub implementation (replace later) ───────────────────────────────────────

struct StubUiApp;

impl UiApp for StubUiApp {
    fn app_name(&self) -> &'static str {
        "Learn"
    }
}

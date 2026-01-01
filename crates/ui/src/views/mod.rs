pub(crate) mod editor;
mod history;
mod home;
mod practice;
mod session;
mod settings;
mod summary;
mod state;

pub use editor::EditorView;
pub use history::HistoryView;
pub use home::HomeView;
pub use practice::PracticeView;
pub use session::SessionView;
pub use settings::SettingsView;
pub use summary::SummaryView;
pub use state::{view_state_from_resource, ViewError, ViewState};

#[cfg(test)]
mod test_harness;
#[cfg(test)]
mod view_smoke;

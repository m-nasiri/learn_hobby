//! Shared error types for the services crate.

use thiserror::Error;

use learn_core::model::{AppSettingsError, CardError, DeckError, SessionSummaryError};
use learn_core::scheduler::SchedulerError;
use storage::repository::StorageError;
use storage::sqlite::SqliteInitError;

/// Errors emitted by AI usage/budget policy.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AiUsageError {
    #[error("daily request cap reached ({cap})")]
    DailyCapReached { cap: u32 },
    #[error("cooldown active ({remaining_secs}s remaining)")]
    CooldownActive { remaining_secs: u32 },
    #[error("pricing missing for provider {provider} model {model}")]
    MissingPriceEntry { provider: String, model: String },
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Errors emitted by `WritingToolsService`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WritingToolsError {
    #[error("writing tools are not configured")]
    Disabled,
    #[error("writing tools returned an empty response")]
    EmptyResponse,
    #[error("writing tools response did not include usage data")]
    MissingUsage,
    #[error("writing tools request failed with status {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error(transparent)]
    Usage(#[from] AiUsageError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

/// Errors emitted by `ReviewService`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReviewServiceError {
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Errors emitted by `CardService`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CardServiceError {
    #[error(transparent)]
    Card(#[from] CardError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Errors emitted by `DeckService`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DeckServiceError {
    #[error(transparent)]
    Deck(#[from] DeckError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Errors emitted by session services.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SessionError {
    #[error("no cards available for session")]
    Empty,
    #[error("session already completed")]
    Completed,
    #[error("not enough grades to complete session")]
    InsufficientGrades,
    #[error(transparent)]
    Summary(#[from] SessionSummaryError),
    #[error(transparent)]
    Review(#[from] ReviewServiceError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Errors emitted while bootstrapping app services.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppServicesError {
    #[error(transparent)]
    Sqlite(#[from] SqliteInitError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Deck(#[from] DeckError),
}

/// Errors emitted by `AppSettingsService`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppSettingsServiceError {
    #[error(transparent)]
    Settings(#[from] AppSettingsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

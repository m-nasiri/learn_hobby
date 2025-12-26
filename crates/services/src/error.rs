//! Shared error types for the services crate.

use thiserror::Error;

use learn_core::model::{CardError, DeckError, SessionSummaryError};
use learn_core::scheduler::SchedulerError;
use storage::repository::StorageError;
use storage::sqlite::SqliteInitError;

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

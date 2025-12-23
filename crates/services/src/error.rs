//! Shared error types for the services crate.

use thiserror::Error;

use learn_core::model::SessionSummaryError;
use learn_core::scheduler::SchedulerError;
use storage::repository::StorageError;

/// Errors emitted by `ReviewService`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReviewServiceError {
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
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

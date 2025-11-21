use thiserror::Error;

use crate::model::content::MediaValidationError;
use crate::model::content::TextError;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    MediaValidation(#[from] MediaValidationError),
    #[error(transparent)]
    TextValidation(#[from] TextError),
    // #[error(transparent)]
    // Storage(#[from] StorageError),
    // later:
    // Scheduler(#[from] SchedulerError),
}

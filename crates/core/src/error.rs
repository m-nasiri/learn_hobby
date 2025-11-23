use thiserror::Error;

use crate::model::ContentValidationError;
use crate::model::MediaValidationError;
use crate::model::TextError;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    MediaValidation(#[from] MediaValidationError),

    #[error(transparent)]
    TextValidation(#[from] TextError),

    #[error(transparent)]
    ContentValidation(#[from] ContentValidationError),
}

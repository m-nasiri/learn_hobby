use thiserror::Error;

use crate::model::CardError;
use crate::model::ContentValidationError;
use crate::model::DeckError;
use crate::model::MediaValidationError;
use crate::model::ReviewError;
use crate::model::TextError;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    MediaValidation(#[from] MediaValidationError),

    #[error(transparent)]
    TextValidation(#[from] TextError),

    #[error(transparent)]
    ContentValidation(#[from] ContentValidationError),

    #[error(transparent)]
    ReviewValidation(#[from] ReviewError),

    #[error(transparent)]
    DeckValidation(#[from] DeckError),

    #[error(transparent)]
    CardValidation(#[from] CardError),
}

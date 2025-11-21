use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TextError {
    #[error("Text not be empty.")]
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontText(String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackText(String);

impl FrontText {
    pub fn parse(s: impl Into<String>) -> Result<Self, TextError> {
        let s = s.into();
        if s.trim().is_empty() {
            return Err(TextError::Empty);
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl BackText {
    pub fn parse(s: impl Into<String>) -> Result<Self, TextError> {
        let s = s.into();
        if s.trim().is_empty() {
            return Err(TextError::Empty);
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TextError {
    #[error("Text not be empty.")]
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Text<T>(String, std::marker::PhantomData<T>);

pub struct Front;
pub struct Back;

pub type FrontText = Text<Front>;
pub type BackText = Text<Back>;

impl<T> Text<T> {
    pub fn parse(s: impl Into<String>) -> Result<Self, TextError> {
        let s = s.into();
        if s.trim().is_empty() {
            return Err(TextError::Empty);
        }
        Ok(Self(s, std::marker::PhantomData))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

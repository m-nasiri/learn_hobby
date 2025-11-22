use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TextError {
    #[error("Text not be empty.")]
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Text<T>(String, std::marker::PhantomData<T>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Front;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

//
// ─── UNIT TESTS ──────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_parse() {
        let txt = FrontText::parse("Text is not empty").unwrap();
        assert_eq!(txt.as_str(), "Text is not empty");
    }

    #[test]
    fn test_text_with_leading_trailing_whitespace() {
        let txt = FrontText::parse(" Text is not empty ").unwrap();
        assert_eq!(txt.as_str(), " Text is not empty ");
    }

    #[test]
    fn test_empty_text() {
        let txt = FrontText::parse("  ");
        assert_eq!(txt, Err(TextError::Empty));
    }
}

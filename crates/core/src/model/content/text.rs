use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when validating text content.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextError {
    #[error("Text cannot be empty.")]
    Empty,
}

/// Type-safe wrapper for text with phantom type marker.
///
/// `Text<T>` uses the typestate pattern to distinguish between different kinds of text
/// at compile time. This prevents accidentally mixing front and back text.
///
/// # Type Safety
///
/// ```
/// use learn_core::model::content::text::{FrontText, BackText};
///
/// let front = FrontText::parse("Question").unwrap();
/// let back = BackText::parse("Answer").unwrap();
///
/// // The following would not compile:
/// // let mixed: FrontText = back; // Error: type mismatch
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Text<T>(String, std::marker::PhantomData<T>);

/// Marker type for front-facing text (questions/prompts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Front;

/// Marker type for back-facing text (answers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Back;

/// Type alias for front-facing text (questions/prompts).
pub type FrontText = Text<Front>;

/// Type alias for back-facing text (answers).
pub type BackText = Text<Back>;

impl<T> Text<T> {
    /// Parses and validates text from a string.
    ///
    /// # Errors
    ///
    /// Returns `TextError::Empty` if the input is empty or whitespace-only.
    ///
    /// # Examples
    ///
    /// ```
    /// use learn_core::model::content::text::FrontText;
    ///
    /// let text = FrontText::parse("Hello, world!").unwrap();
    /// assert_eq!(text.as_str(), "Hello, world!");
    ///
    /// let empty = FrontText::parse("   ");
    /// assert!(empty.is_err());
    /// ```
    pub fn parse(s: impl Into<String>) -> Result<Self, TextError> {
        let s = s.into();
        if s.trim().is_empty() {
            return Err(TextError::Empty);
        }
        Ok(Self(s, std::marker::PhantomData))
    }

    /// Returns the text as a string slice.
    #[must_use]
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

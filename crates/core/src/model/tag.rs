use thiserror::Error;

use crate::model::DeckId;
use crate::model::ids::TagId;

/// Validated tag name (trimmed, non-empty).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagName(String);

impl TagName {
    /// Create a validated tag name.
    ///
    /// # Errors
    ///
    /// Returns `TagError::EmptyName` if the name is empty after trimming.
    pub fn new(value: impl Into<String>) -> Result<Self, TagError> {
        let raw = value.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(TagError::EmptyName);
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TagName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A tag scoped to a deck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    id: TagId,
    deck_id: DeckId,
    name: TagName,
}

impl Tag {
    #[must_use]
    pub fn new(id: TagId, deck_id: DeckId, name: TagName) -> Self {
        Self { id, deck_id, name }
    }

    #[must_use]
    pub fn id(&self) -> TagId {
        self.id
    }

    #[must_use]
    pub fn deck_id(&self) -> DeckId {
        self.deck_id
    }

    #[must_use]
    pub fn name(&self) -> &TagName {
        &self.name
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TagError {
    #[error("tag name cannot be empty")]
    EmptyName,
}

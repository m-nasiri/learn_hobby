use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Unique identifier for a Card
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CardId(u64);

impl CardId {
    /// Creates a new `CardId`
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the underlying u64 value
    #[must_use]
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Unique identifier for a Deck
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeckId(u64);

impl DeckId {
    /// Creates a new `DeckId`
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the underlying u64 value
    #[must_use]
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Unique identifier for a Media item
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MediaId(u64);

impl MediaId {
    /// Creates a new `MediaId`
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the underlying u64 value
    #[must_use]
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Unique identifier for a Tag
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TagId(u64);

impl TagId {
    /// Creates a new `TagId`
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the underlying u64 value
    #[must_use]
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Debug for CardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CardId({})", self.0)
    }
}

impl fmt::Debug for DeckId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeckId({})", self.0)
    }
}

impl fmt::Debug for MediaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MediaId({})", self.0)
    }
}

impl fmt::Debug for TagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TagId({})", self.0)
    }
}

// ─── Display Implementations ───────────────────────────────────────────────────

impl fmt::Display for CardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for DeckId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for MediaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for TagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ─── FromStr Implementations ───────────────────────────────────────────────────

/// Error type for parsing ID from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIdError {
    kind: String,
}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to parse {} from string", self.kind)
    }
}

impl std::error::Error for ParseIdError {}

impl FromStr for CardId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>()
            .map(CardId::new)
            .map_err(|_| ParseIdError {
                kind: "CardId".to_string(),
            })
    }
}

impl FromStr for DeckId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>()
            .map(DeckId::new)
            .map_err(|_| ParseIdError {
                kind: "DeckId".to_string(),
            })
    }
}

impl FromStr for MediaId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>()
            .map(MediaId::new)
            .map_err(|_| ParseIdError {
                kind: "MediaId".to_string(),
            })
    }
}

impl FromStr for TagId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>()
            .map(TagId::new)
            .map_err(|_| ParseIdError {
                kind: "TagId".to_string(),
            })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_id_display() {
        let id = CardId::new(42);
        assert_eq!(id.to_string(), "42");
    }

    #[test]
    fn test_card_id_from_str() {
        let id: CardId = "123".parse().unwrap();
        assert_eq!(id, CardId::new(123));
    }

    #[test]
    fn test_card_id_from_str_invalid() {
        let result = "not-a-number".parse::<CardId>();
        assert!(result.is_err());
    }

    #[test]
    fn test_deck_id_display() {
        let id = DeckId::new(99);
        assert_eq!(id.to_string(), "99");
    }

    #[test]
    fn test_deck_id_from_str() {
        let id: DeckId = "456".parse().unwrap();
        assert_eq!(id, DeckId::new(456));
    }

    #[test]
    fn test_media_id_display() {
        let id = MediaId::new(1000);
        assert_eq!(id.to_string(), "1000");
    }

    #[test]
    fn test_media_id_from_str() {
        let id: MediaId = "789".parse().unwrap();
        assert_eq!(id, MediaId::new(789));
    }

    #[test]
    fn test_tag_id_display() {
        let id = TagId::new(77);
        assert_eq!(id.to_string(), "77");
    }

    #[test]
    fn test_tag_id_from_str() {
        let id: TagId = "55".parse().unwrap();
        assert_eq!(id, TagId::new(55));
    }

    #[test]
    fn test_id_roundtrip() {
        let original = CardId::new(42);
        let serialized = original.to_string();
        let deserialized: CardId = serialized.parse().unwrap();
        assert_eq!(original, deserialized);
    }
}

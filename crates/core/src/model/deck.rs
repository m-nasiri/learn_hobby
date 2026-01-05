use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::ids::DeckId;

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeckError {
    #[error("deck name cannot be empty")]
    EmptyName,

    #[error("micro session size must be > 0")]
    InvalidMicroSessionSize,

    #[error("new cards per day must be > 0")]
    InvalidNewCardsPerDay,

    #[error("review limit per day must be > 0")]
    InvalidReviewLimitPerDay,
}

//
// ─── SETTINGS ──────────────────────────────────────────────────────────────────
//

/// Configuration settings for a deck.
///
/// Controls daily limits and session sizes for spaced repetition learning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckSettings {
    new_cards_per_day: u32,
    review_limit_per_day: u32,
    micro_session_size: u32,
    protect_overload: bool,
}

impl DeckSettings {
    /// Creates ADHD-friendly default settings.
    ///
    /// Returns settings optimized for users with ADHD:
    /// - 5 new cards per day (manageable goal)
    /// - 30 reviews per day limit (prevents overwhelm)
    /// - 5 cards per micro-session (quick wins)
    /// - protect overload enabled (keeps daily load calm)
    #[must_use]
    pub fn default_for_adhd() -> Self {
        Self {
            new_cards_per_day: 5,
            review_limit_per_day: 30,
            micro_session_size: 5,
            protect_overload: true,
        }
    }

    /// Creates custom deck settings.
    ///
    /// # Errors
    ///
    /// Returns error if any parameter is zero.
    pub fn new(
        new_cards_per_day: u32,
        review_limit_per_day: u32,
        micro_session_size: u32,
        protect_overload: bool,
    ) -> Result<Self, DeckError> {
        if micro_session_size == 0 {
            return Err(DeckError::InvalidMicroSessionSize);
        }
        if new_cards_per_day == 0 {
            return Err(DeckError::InvalidNewCardsPerDay);
        }
        if review_limit_per_day == 0 {
            return Err(DeckError::InvalidReviewLimitPerDay);
        }

        Ok(Self {
            new_cards_per_day,
            review_limit_per_day,
            micro_session_size,
            protect_overload,
        })
    }

    // Accessors
    #[must_use]
    pub fn new_cards_per_day(&self) -> u32 {
        self.new_cards_per_day
    }

    #[must_use]
    pub fn review_limit_per_day(&self) -> u32 {
        self.review_limit_per_day
    }

    #[must_use]
    pub fn micro_session_size(&self) -> u32 {
        self.micro_session_size
    }

    /// When true, enforce review limits to avoid overload.
    #[must_use]
    pub fn protect_overload(&self) -> bool {
        self.protect_overload
    }
}

//
// ─── DECK ──────────────────────────────────────────────────────────────────────
//

/// A collection of flashcards with associated settings.
///
/// Decks organize cards by topic and control learning parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deck {
    id: DeckId,
    name: String,
    description: Option<String>,
    settings: DeckSettings,
    created_at: DateTime<Utc>,
}

impl Deck {
    /// Creates a new Deck.
    ///
    /// # Errors
    ///
    /// Returns `DeckError::EmptyName` if name is empty or whitespace-only.
    pub fn new(
        id: DeckId,
        name: impl Into<String>,
        description: Option<String>,
        settings: DeckSettings,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DeckError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DeckError::EmptyName);
        }

        let description = description
            .map(|d| d.trim().to_owned())
            .filter(|d| !d.is_empty());

        Ok(Self {
            id,
            name: name.trim().to_owned(),
            description,
            settings,
            created_at,
        })
    }

    // Accessors
    #[must_use]
    pub fn id(&self) -> DeckId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[must_use]
    pub fn settings(&self) -> &DeckSettings {
        &self.settings
    }

    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::fixed_now;

    #[test]
    fn deck_new_rejects_empty_name() {
        let settings = DeckSettings::default_for_adhd();
        let err = Deck::new(DeckId::new(1), "   ", None, settings, fixed_now()).unwrap_err();
        assert_eq!(err, DeckError::EmptyName);
    }

    #[test]
    fn settings_new_rejects_zero_micro_session() {
        let err = DeckSettings::new(5, 30, 0, true).unwrap_err();
        assert_eq!(err, DeckError::InvalidMicroSessionSize);
    }

    #[test]
    fn settings_default_for_adhd() {
        let settings = DeckSettings::default_for_adhd();
        assert_eq!(settings.new_cards_per_day(), 5);
        assert_eq!(settings.review_limit_per_day(), 30);
        assert_eq!(settings.micro_session_size(), 5);
        assert!(settings.protect_overload());
    }

    #[test]
    fn deck_new_happy_path() {
        let settings = DeckSettings::default_for_adhd();
        let deck = Deck::new(
            DeckId::new(10),
            "German B1",
            Some("verbs + phrases".into()),
            settings,
            fixed_now(),
        )
        .unwrap();

        assert_eq!(deck.id(), DeckId::new(10));
        assert_eq!(deck.name(), "German B1");
        assert_eq!(deck.description(), Some("verbs + phrases"));
        assert_eq!(deck.settings().micro_session_size(), 5);
    }

    #[test]
    fn deck_trims_name_and_description() {
        let settings = DeckSettings::default_for_adhd();
        let deck = Deck::new(
            DeckId::new(1),
            "  Spanish  ",
            Some("  grammar  ".into()),
            settings,
            fixed_now(),
        )
        .unwrap();

        assert_eq!(deck.name(), "Spanish");
        assert_eq!(deck.description(), Some("grammar"));
    }

    #[test]
    fn deck_filters_empty_description() {
        let settings = DeckSettings::default_for_adhd();
        let deck = Deck::new(
            DeckId::new(1),
            "French",
            Some("   ".into()),
            settings,
            fixed_now(),
        )
        .unwrap();

        assert_eq!(deck.description(), None);
    }
}

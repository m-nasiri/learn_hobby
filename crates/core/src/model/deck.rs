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

    #[error("lapse minimum interval must be > 0")]
    InvalidLapseMinInterval,
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
    preserve_stability_on_lapse: bool,
    lapse_min_interval_secs: u32,
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
            preserve_stability_on_lapse: true,
            lapse_min_interval_secs: 86_400,
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
        preserve_stability_on_lapse: bool,
        lapse_min_interval_secs: u32,
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
        if lapse_min_interval_secs == 0 {
            return Err(DeckError::InvalidLapseMinInterval);
        }

        Ok(Self {
            new_cards_per_day,
            review_limit_per_day,
            micro_session_size,
            protect_overload,
            preserve_stability_on_lapse,
            lapse_min_interval_secs,
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

    #[must_use]
    pub fn preserve_stability_on_lapse(&self) -> bool {
        self.preserve_stability_on_lapse
    }

    #[must_use]
    pub fn lapse_min_interval_secs(&self) -> u32 {
        self.lapse_min_interval_secs
    }

    #[must_use]
    pub fn lapse_min_interval(&self) -> chrono::Duration {
        chrono::Duration::seconds(i64::from(self.lapse_min_interval_secs))
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
        let err = DeckSettings::new(5, 30, 0, true, true, 86_400).unwrap_err();
        assert_eq!(err, DeckError::InvalidMicroSessionSize);
    }

    #[test]
    fn settings_default_for_adhd() {
        let settings = DeckSettings::default_for_adhd();
        assert_eq!(settings.new_cards_per_day(), 5);
        assert_eq!(settings.review_limit_per_day(), 30);
        assert_eq!(settings.micro_session_size(), 5);
        assert!(settings.protect_overload());
        assert!(settings.preserve_stability_on_lapse());
        assert_eq!(settings.lapse_min_interval_secs(), 86_400);
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

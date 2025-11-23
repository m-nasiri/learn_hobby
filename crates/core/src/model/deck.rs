use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::ids::DeckId;

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckSettings {
    pub new_cards_per_day: u32,
    pub review_limit_per_day: u32,
    pub micro_session_size: u32,
}

impl DeckSettings {
    pub fn default_for_adhd() -> Self {
        Self {
            new_cards_per_day: 5,
            review_limit_per_day: 30,
            micro_session_size: 5,
        }
    }

    pub fn new(
        new_cards_per_day: u32,
        review_limit_per_day: u32,
        micro_session_size: u32,
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
        })
    }
}

//
// ─── DECK ──────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deck {
    pub id: DeckId,
    pub name: String,
    pub description: Option<String>,
    pub settings: DeckSettings,
    pub created_at: DateTime<Utc>,
}

impl Deck {
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
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn deck_new_rejects_empty_name() {
        let settings = DeckSettings::default_for_adhd();
        let err = Deck::new(DeckId(1), "   ", None, settings, Utc::now()).unwrap_err();
        assert_eq!(err, DeckError::EmptyName);
    }

    #[test]
    fn settings_new_rejects_zero_micro_session() {
        let err = DeckSettings::new(5, 30, 0).unwrap_err();
        assert_eq!(err, DeckError::InvalidMicroSessionSize);
    }

    #[test]
    fn deck_new_happy_path() {
        let settings = DeckSettings::default_for_adhd();
        let deck = Deck::new(
            DeckId(10),
            "German B1",
            Some("verbs + phrases".into()),
            settings,
            Utc::now(),
        )
        .unwrap();

        assert_eq!(deck.id, DeckId(10));
        assert_eq!(deck.name, "German B1");
        assert_eq!(deck.settings.micro_session_size, 5);
    }
}

use chrono::{DateTime, Utc};
use chrono::Weekday;
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

    #[error("FSRS target retention must be in (0, 1]")]
    InvalidFsrsTargetRetention,

    #[error("FSRS optimize-after must be > 0")]
    InvalidFsrsOptimizeAfter,

    #[error("soft reminder seconds must be between 5 and 600")]
    InvalidSoftReminderSeconds,

    #[error("auto reveal seconds must be between 5 and 600")]
    InvalidAutoRevealSeconds,

    #[error("minimum interval must be at least 1 day")]
    InvalidMinIntervalDays,

    #[error("maximum interval must be at least 1 day")]
    InvalidMaxIntervalDays,

    #[error("minimum interval must be <= maximum interval")]
    InvalidIntervalBounds,

    #[error("easy day load factor must be in (0, 1]")]
    InvalidEasyDayLoadFactor,

    #[error("easy days must include at least one day when enabled")]
    InvalidEasyDaysMask,
}

//
// ─── SETTINGS ──────────────────────────────────────────────────────────────────
//

/// Configuration settings for a deck.
///
/// Controls daily limits and session sizes for spaced repetition learning.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct DeckSettings {
    new_cards_per_day: u32,
    review_limit_per_day: u32,
    micro_session_size: u32,
    protect_overload: bool,
    preserve_stability_on_lapse: bool,
    lapse_min_interval_secs: u32,
    show_timer: bool,
    soft_time_reminder: bool,
    auto_advance_cards: bool,
    soft_time_reminder_secs: u32,
    auto_reveal_secs: u32,
    min_interval_days: u32,
    max_interval_days: u32,
    easy_days_enabled: bool,
    easy_day_load_factor: f32,
    easy_days_mask: u8,
    fsrs_target_retention: f32,
    fsrs_optimize_enabled: bool,
    fsrs_optimize_after: u32,
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
            show_timer: false,
            soft_time_reminder: false,
            auto_advance_cards: false,
            soft_time_reminder_secs: 25,
            auto_reveal_secs: 20,
            min_interval_days: 1,
            max_interval_days: 365,
            easy_days_enabled: true,
            easy_day_load_factor: 0.5,
            easy_days_mask: easy_days_mask(&[Weekday::Sat, Weekday::Sun]),
            fsrs_target_retention: 0.85,
            fsrs_optimize_enabled: true,
            fsrs_optimize_after: 100,
        }
    }

    /// Creates custom deck settings.
    ///
    /// # Errors
    ///
    /// Returns error if any parameter is zero.
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn new(
        new_cards_per_day: u32,
        review_limit_per_day: u32,
        micro_session_size: u32,
        protect_overload: bool,
        preserve_stability_on_lapse: bool,
        lapse_min_interval_secs: u32,
        show_timer: bool,
        soft_time_reminder: bool,
        auto_advance_cards: bool,
        soft_time_reminder_secs: u32,
        auto_reveal_secs: u32,
        min_interval_days: u32,
        max_interval_days: u32,
        easy_days_enabled: bool,
        easy_day_load_factor: f32,
        easy_days_mask: u8,
        fsrs_target_retention: f32,
        fsrs_optimize_enabled: bool,
        fsrs_optimize_after: u32,
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
        if !fsrs_target_retention.is_finite()
            || fsrs_target_retention <= 0.0
            || fsrs_target_retention > 1.0
        {
            return Err(DeckError::InvalidFsrsTargetRetention);
        }
        if fsrs_optimize_after == 0 {
            return Err(DeckError::InvalidFsrsOptimizeAfter);
        }
        if !(5..=600).contains(&soft_time_reminder_secs) {
            return Err(DeckError::InvalidSoftReminderSeconds);
        }
        if !(5..=600).contains(&auto_reveal_secs) {
            return Err(DeckError::InvalidAutoRevealSeconds);
        }
        if min_interval_days == 0 {
            return Err(DeckError::InvalidMinIntervalDays);
        }
        if max_interval_days == 0 {
            return Err(DeckError::InvalidMaxIntervalDays);
        }
        if min_interval_days > max_interval_days {
            return Err(DeckError::InvalidIntervalBounds);
        }
        if !easy_day_load_factor.is_finite()
            || easy_day_load_factor <= 0.0
            || easy_day_load_factor > 1.0
        {
            return Err(DeckError::InvalidEasyDayLoadFactor);
        }
        if easy_days_enabled && easy_days_mask == 0 {
            return Err(DeckError::InvalidEasyDaysMask);
        }

        Ok(Self {
            new_cards_per_day,
            review_limit_per_day,
            micro_session_size,
            protect_overload,
            preserve_stability_on_lapse,
            lapse_min_interval_secs,
            show_timer,
            soft_time_reminder,
            auto_advance_cards,
            soft_time_reminder_secs,
            auto_reveal_secs,
            min_interval_days,
            max_interval_days,
            easy_days_enabled,
            easy_day_load_factor,
            easy_days_mask,
            fsrs_target_retention,
            fsrs_optimize_enabled,
            fsrs_optimize_after,
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
    pub fn show_timer(&self) -> bool {
        self.show_timer
    }

    #[must_use]
    pub fn soft_time_reminder(&self) -> bool {
        self.soft_time_reminder
    }

    #[must_use]
    pub fn auto_advance_cards(&self) -> bool {
        self.auto_advance_cards
    }

    #[must_use]
    pub fn soft_time_reminder_secs(&self) -> u32 {
        self.soft_time_reminder_secs
    }

    #[must_use]
    pub fn auto_reveal_secs(&self) -> u32 {
        self.auto_reveal_secs
    }

    #[must_use]
    pub fn min_interval_days(&self) -> u32 {
        self.min_interval_days
    }

    #[must_use]
    pub fn max_interval_days(&self) -> u32 {
        self.max_interval_days
    }

    #[must_use]
    pub fn easy_days_enabled(&self) -> bool {
        self.easy_days_enabled
    }

    #[must_use]
    pub fn easy_day_load_factor(&self) -> f32 {
        self.easy_day_load_factor
    }

    #[must_use]
    pub fn easy_days_mask(&self) -> u8 {
        self.easy_days_mask
    }

    #[must_use]
    pub fn is_easy_day(&self, weekday: Weekday) -> bool {
        if !self.easy_days_enabled {
            return false;
        }
        self.easy_days_mask & weekday_bit(weekday) != 0
    }

    #[must_use]
    pub fn fsrs_target_retention(&self) -> f32 {
        self.fsrs_target_retention
    }

    #[must_use]
    pub fn fsrs_optimize_enabled(&self) -> bool {
        self.fsrs_optimize_enabled
    }

    #[must_use]
    pub fn fsrs_optimize_after(&self) -> u32 {
        self.fsrs_optimize_after
    }

    #[must_use]
    pub fn lapse_min_interval(&self) -> chrono::Duration {
        chrono::Duration::seconds(i64::from(self.lapse_min_interval_secs))
    }
}

fn weekday_bit(weekday: Weekday) -> u8 {
    match weekday {
        Weekday::Mon => 1 << 0,
        Weekday::Tue => 1 << 1,
        Weekday::Wed => 1 << 2,
        Weekday::Thu => 1 << 3,
        Weekday::Fri => 1 << 4,
        Weekday::Sat => 1 << 5,
        Weekday::Sun => 1 << 6,
    }
}

fn easy_days_mask(days: &[Weekday]) -> u8 {
    days.iter().fold(0, |mask, day| mask | weekday_bit(*day))
}

//
// ─── DECK ──────────────────────────────────────────────────────────────────────
//

/// A collection of flashcards with associated settings.
///
/// Decks organize cards by topic and control learning parameters.
#[derive(Debug, Clone, PartialEq)]
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
        let err = DeckSettings::new(
            5, 30, 0, true, true, 86_400, false, false, false, 25, 20, 1, 365, false, 0.5, 0,
            0.85, true, 100,
        )
        .unwrap_err();
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
        assert!(!settings.show_timer());
        assert!(!settings.soft_time_reminder());
        assert!(!settings.auto_advance_cards());
        assert_eq!(settings.soft_time_reminder_secs(), 25);
        assert_eq!(settings.auto_reveal_secs(), 20);
        assert_eq!(settings.min_interval_days(), 1);
        assert_eq!(settings.max_interval_days(), 365);
        assert!(settings.easy_days_enabled());
        assert!((settings.easy_day_load_factor() - 0.5).abs() < f32::EPSILON);
        assert!(settings.is_easy_day(Weekday::Sat));
        assert!(settings.is_easy_day(Weekday::Sun));
        assert!(!settings.is_easy_day(Weekday::Mon));
        assert!((settings.fsrs_target_retention() - 0.85).abs() < f32::EPSILON);
        assert!(settings.fsrs_optimize_enabled());
        assert_eq!(settings.fsrs_optimize_after(), 100);
    }

    #[test]
    fn settings_rejects_invalid_retention() {
        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 25, 20, 1, 365, false, 0.5, 0, 0.0,
            true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidFsrsTargetRetention);

        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 25, 20, 1, 365, false, 0.5, 0, 1.1,
            true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidFsrsTargetRetention);
    }

    #[test]
    fn settings_rejects_invalid_timer_bounds() {
        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 2, 20, 1, 365, false, 0.5, 0, 0.85,
            true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidSoftReminderSeconds);

        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 25, 700, 1, 365, false, 0.5, 0,
            0.85, true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidAutoRevealSeconds);
    }

    #[test]
    fn settings_rejects_invalid_interval_bounds() {
        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 25, 20, 0, 365, false, 0.5, 0, 0.85,
            true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidMinIntervalDays);

        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 25, 20, 1, 0, false, 0.5, 0, 0.85,
            true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidMaxIntervalDays);

        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 25, 20, 10, 5, false, 0.5, 0, 0.85,
            true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidIntervalBounds);
    }

    #[test]
    fn settings_rejects_invalid_easy_days() {
        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 25, 20, 1, 365, true, 0.0, 1, 0.85,
            true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidEasyDayLoadFactor);

        let err = DeckSettings::new(
            5, 30, 5, true, true, 86_400, false, false, false, 25, 20, 1, 365, true, 0.5, 0, 0.85,
            true, 100,
        )
        .unwrap_err();
        assert_eq!(err, DeckError::InvalidEasyDaysMask);
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

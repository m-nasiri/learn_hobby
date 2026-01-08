use learn_core::model::{Deck, DeckId, DeckSettings};

use crate::views::ViewError;

use super::helpers::{
    format_lapse_interval, format_retention, normalize_description, parse_lapse_interval_secs,
    parse_positive_u32, parse_retention, parse_timer_secs,
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct DeckSettingsData {
    pub(super) deck: Deck,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct DeckSettingsSnapshot {
    pub(super) deck_id: DeckId,
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) new_cards_per_day: u32,
    pub(super) review_limit_per_day: u32,
    pub(super) micro_session_size: u32,
    pub(super) protect_overload: bool,
    pub(super) preserve_stability_on_lapse: bool,
    pub(super) lapse_min_interval_secs: u32,
    pub(super) show_timer: bool,
    pub(super) soft_time_reminder: bool,
    pub(super) auto_advance_cards: bool,
    pub(super) soft_time_reminder_secs: u32,
    pub(super) auto_reveal_secs: u32,
    pub(super) min_interval_days: u32,
    pub(super) max_interval_days: u32,
    pub(super) easy_days_enabled: bool,
    pub(super) easy_day_load_factor: f32,
    pub(super) easy_days_mask: u8,
    pub(super) fsrs_target_retention: f32,
    pub(super) fsrs_optimize_enabled: bool,
    pub(super) fsrs_optimize_after: u32,
}

impl DeckSettingsSnapshot {
    pub(super) fn from_deck(deck: &Deck) -> Self {
        let settings = deck.settings();
        Self {
            deck_id: deck.id(),
            name: deck.name().to_string(),
            description: deck.description().map(str::to_owned),
            new_cards_per_day: settings.new_cards_per_day(),
            review_limit_per_day: settings.review_limit_per_day(),
            micro_session_size: settings.micro_session_size(),
            protect_overload: settings.protect_overload(),
            preserve_stability_on_lapse: settings.preserve_stability_on_lapse(),
            lapse_min_interval_secs: settings.lapse_min_interval_secs(),
            show_timer: settings.show_timer(),
            soft_time_reminder: settings.soft_time_reminder(),
            auto_advance_cards: settings.auto_advance_cards(),
            soft_time_reminder_secs: settings.soft_time_reminder_secs(),
            auto_reveal_secs: settings.auto_reveal_secs(),
            min_interval_days: settings.min_interval_days(),
            max_interval_days: settings.max_interval_days(),
            easy_days_enabled: settings.easy_days_enabled(),
            easy_day_load_factor: settings.easy_day_load_factor(),
            easy_days_mask: settings.easy_days_mask(),
            fsrs_target_retention: settings.fsrs_target_retention(),
            fsrs_optimize_enabled: settings.fsrs_optimize_enabled(),
            fsrs_optimize_after: settings.fsrs_optimize_after(),
        }
    }

    pub(super) fn from_validated(deck_id: DeckId, validated: &ValidatedSettings) -> Self {
        let settings = &validated.settings;
        Self {
            deck_id,
            name: validated.name.clone(),
            description: validated.description.clone(),
            new_cards_per_day: settings.new_cards_per_day(),
            review_limit_per_day: settings.review_limit_per_day(),
            micro_session_size: settings.micro_session_size(),
            protect_overload: settings.protect_overload(),
            preserve_stability_on_lapse: settings.preserve_stability_on_lapse(),
            lapse_min_interval_secs: settings.lapse_min_interval_secs(),
            show_timer: settings.show_timer(),
            soft_time_reminder: settings.soft_time_reminder(),
            auto_advance_cards: settings.auto_advance_cards(),
            soft_time_reminder_secs: settings.soft_time_reminder_secs(),
            auto_reveal_secs: settings.auto_reveal_secs(),
            min_interval_days: settings.min_interval_days(),
            max_interval_days: settings.max_interval_days(),
            easy_days_enabled: settings.easy_days_enabled(),
            easy_day_load_factor: settings.easy_day_load_factor(),
            easy_days_mask: settings.easy_days_mask(),
            fsrs_target_retention: settings.fsrs_target_retention(),
            fsrs_optimize_enabled: settings.fsrs_optimize_enabled(),
            fsrs_optimize_after: settings.fsrs_optimize_after(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct DeckSettingsForm {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) new_cards_per_day: String,
    pub(super) review_limit_per_day: String,
    pub(super) micro_session_size: String,
    pub(super) protect_overload: bool,
    pub(super) preserve_stability_on_lapse: bool,
    pub(super) lapse_min_interval: String,
    pub(super) show_timer: bool,
    pub(super) soft_time_reminder: bool,
    pub(super) auto_advance_cards: bool,
    pub(super) soft_time_reminder_secs: String,
    pub(super) auto_reveal_secs: String,
    pub(super) easy_days_enabled: bool,
    pub(super) easy_day_load_factor: String,
    pub(super) easy_days_mask: u8,
    pub(super) fsrs_target_retention: String,
    pub(super) fsrs_optimize_enabled: bool,
    pub(super) fsrs_optimize_after: String,
    pub(super) max_interval_days: String,
    pub(super) min_interval_days: String,
    pub(super) fsrs_parameters: String,
}

impl DeckSettingsForm {
    pub(super) fn from_snapshot(snapshot: &DeckSettingsSnapshot) -> Self {
        Self {
            name: snapshot.name.clone(),
            description: snapshot.description.clone().unwrap_or_default(),
            new_cards_per_day: snapshot.new_cards_per_day.to_string(),
            review_limit_per_day: snapshot.review_limit_per_day.to_string(),
            micro_session_size: snapshot.micro_session_size.to_string(),
            protect_overload: snapshot.protect_overload,
            preserve_stability_on_lapse: snapshot.preserve_stability_on_lapse,
            lapse_min_interval: format_lapse_interval(snapshot.lapse_min_interval_secs),
            show_timer: snapshot.show_timer,
            soft_time_reminder: snapshot.soft_time_reminder,
            auto_advance_cards: snapshot.auto_advance_cards,
            soft_time_reminder_secs: snapshot.soft_time_reminder_secs.to_string(),
            auto_reveal_secs: snapshot.auto_reveal_secs.to_string(),
            easy_days_enabled: snapshot.easy_days_enabled,
            easy_day_load_factor: format_retention(snapshot.easy_day_load_factor),
            easy_days_mask: snapshot.easy_days_mask,
            fsrs_target_retention: format_retention(snapshot.fsrs_target_retention),
            fsrs_optimize_enabled: snapshot.fsrs_optimize_enabled,
            fsrs_optimize_after: snapshot.fsrs_optimize_after.to_string(),
            max_interval_days: snapshot.max_interval_days.to_string(),
            min_interval_days: snapshot.min_interval_days.to_string(),
            fsrs_parameters: "0.2120, 1.2931, 2.3065, 8.2956, 6.4133, 0.8334, 3.0194, 0.0010, 1.8722, 0.1666, 0.7960, 1.4835, 0.0614, 0.2629, 1.6483, 0.6014, 1.8729, 0.5425, 0.0912, 0.0658, 0.1542".to_string(),
        }
    }

    pub(super) fn to_snapshot(&self, deck_id: DeckId) -> Option<DeckSettingsSnapshot> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }

        let new_cards_per_day = parse_positive_u32(&self.new_cards_per_day)?;
        let review_limit_per_day = parse_positive_u32(&self.review_limit_per_day)?;
        let micro_session_size = parse_positive_u32(&self.micro_session_size)?;

        let lapse_min_interval_secs = parse_lapse_interval_secs(&self.lapse_min_interval)?;
        let fsrs_target_retention = parse_retention(&self.fsrs_target_retention)?;
        let fsrs_optimize_after = parse_positive_u32(&self.fsrs_optimize_after)?;
        let soft_time_reminder_secs = parse_timer_secs(&self.soft_time_reminder_secs)?;
        let auto_reveal_secs = parse_timer_secs(&self.auto_reveal_secs)?;
        let min_interval_days = parse_positive_u32(&self.min_interval_days)?;
        let max_interval_days = parse_positive_u32(&self.max_interval_days)?;
        let easy_day_load_factor = parse_retention(&self.easy_day_load_factor)?;
        if self.easy_days_enabled && self.easy_days_mask == 0 {
            return None;
        }
        if min_interval_days > max_interval_days {
            return None;
        }

        Some(DeckSettingsSnapshot {
            deck_id,
            name: name.to_string(),
            description: normalize_description(&self.description),
            new_cards_per_day,
            review_limit_per_day,
            micro_session_size,
            protect_overload: self.protect_overload,
            preserve_stability_on_lapse: self.preserve_stability_on_lapse,
            lapse_min_interval_secs,
            show_timer: self.show_timer,
            soft_time_reminder: self.soft_time_reminder,
            auto_advance_cards: self.auto_advance_cards,
            soft_time_reminder_secs,
            auto_reveal_secs,
            min_interval_days,
            max_interval_days,
            easy_days_enabled: self.easy_days_enabled,
            easy_day_load_factor,
            easy_days_mask: self.easy_days_mask,
            fsrs_target_retention,
            fsrs_optimize_enabled: self.fsrs_optimize_enabled,
            fsrs_optimize_after,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct DeckSettingsErrors {
    pub(super) name: Option<&'static str>,
    pub(super) new_cards_per_day: Option<&'static str>,
    pub(super) review_limit_per_day: Option<&'static str>,
    pub(super) micro_session_size: Option<&'static str>,
    pub(super) lapse_min_interval: Option<&'static str>,
    pub(super) soft_time_reminder_secs: Option<&'static str>,
    pub(super) auto_reveal_secs: Option<&'static str>,
    pub(super) min_interval_days: Option<&'static str>,
    pub(super) max_interval_days: Option<&'static str>,
    pub(super) easy_day_load_factor: Option<&'static str>,
    pub(super) easy_days_mask: Option<&'static str>,
    pub(super) fsrs_target_retention: Option<&'static str>,
    pub(super) fsrs_optimize_after: Option<&'static str>,
}

impl DeckSettingsErrors {
    pub(super) fn has_any(&self) -> bool {
        self.name.is_some()
            || self.new_cards_per_day.is_some()
            || self.review_limit_per_day.is_some()
            || self.micro_session_size.is_some()
            || self.lapse_min_interval.is_some()
            || self.soft_time_reminder_secs.is_some()
            || self.auto_reveal_secs.is_some()
            || self.min_interval_days.is_some()
            || self.max_interval_days.is_some()
            || self.easy_day_load_factor.is_some()
            || self.easy_days_mask.is_some()
            || self.fsrs_target_retention.is_some()
            || self.fsrs_optimize_after.is_some()
    }
}

pub(super) struct ValidatedSettings {
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) settings: DeckSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SettingsSection {
    DailyLimits,
    Lapses,
    Fsrs,
    Audio,
    Timers,
    EasyDays,
    Advanced,
}

impl SettingsSection {
    pub(super) fn anchor_id(self) -> &'static str {
        match self {
            SettingsSection::DailyLimits => "settings-daily-limits",
            SettingsSection::Lapses => "settings-lapses",
            SettingsSection::Fsrs => "settings-fsrs",
            SettingsSection::Audio => "settings-audio",
            SettingsSection::Timers => "settings-timers",
            SettingsSection::EasyDays => "settings-easy-days",
            SettingsSection::Advanced => "settings-advanced",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SaveState {
    Idle,
    Saving,
    Saved,
    Error(ViewError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResetState {
    Idle,
    Resetting,
    Error(ViewError),
}

#[allow(clippy::too_many_lines)]
pub(super) fn validate_form(
    form: &DeckSettingsForm,
) -> Result<ValidatedSettings, Box<DeckSettingsErrors>> {
    let mut errors = DeckSettingsErrors::default();
    let parsed = parse_settings_form(form, &mut errors);

    if errors.has_any() {
        return Err(Box::new(errors));
    }

    let settings = DeckSettings::new(
        parsed.new_cards_per_day,
        parsed.review_limit_per_day,
        parsed.micro_session_size,
        form.protect_overload,
        form.preserve_stability_on_lapse,
        parsed.lapse_min_interval_secs,
        form.show_timer,
        form.soft_time_reminder,
        form.auto_advance_cards,
        parsed.soft_time_reminder_secs,
        parsed.auto_reveal_secs,
        parsed.min_interval_days,
        parsed.max_interval_days,
        form.easy_days_enabled,
        parsed.easy_day_load_factor,
        parsed.easy_days_mask,
        parsed.fsrs_target_retention,
        form.fsrs_optimize_enabled,
        parsed.fsrs_optimize_after,
    )
    .map_err(|err| map_deck_settings_error(&err))?;

    Ok(ValidatedSettings {
        name: parsed.name,
        description: normalize_description(&form.description),
        settings,
    })
}

struct ParsedSettings {
    name: String,
    new_cards_per_day: u32,
    review_limit_per_day: u32,
    micro_session_size: u32,
    lapse_min_interval_secs: u32,
    soft_time_reminder_secs: u32,
    auto_reveal_secs: u32,
    min_interval_days: u32,
    max_interval_days: u32,
    easy_day_load_factor: f32,
    easy_days_mask: u8,
    fsrs_target_retention: f32,
    fsrs_optimize_after: u32,
}

fn parse_settings_form(
    form: &DeckSettingsForm,
    errors: &mut DeckSettingsErrors,
) -> ParsedSettings {
    let name = parse_required_name(form, errors);

    let new_cards_per_day = parse_u32_field(
        &form.new_cards_per_day,
        &mut errors.new_cards_per_day,
        "Enter a positive number.",
    );
    let review_limit_per_day = parse_u32_field(
        &form.review_limit_per_day,
        &mut errors.review_limit_per_day,
        "Enter a positive number.",
    );
    let micro_session_size = parse_u32_field(
        &form.micro_session_size,
        &mut errors.micro_session_size,
        "Enter a positive number.",
    );
    let lapse_min_interval_secs = parse_duration_field(
        &form.lapse_min_interval,
        &mut errors.lapse_min_interval,
        "Use a duration like 10m or 1d.",
    );
    let soft_time_reminder_secs = parse_timer_field(
        &form.soft_time_reminder_secs,
        &mut errors.soft_time_reminder_secs,
        "Enter 5-600 seconds.",
    );
    let auto_reveal_secs = parse_timer_field(
        &form.auto_reveal_secs,
        &mut errors.auto_reveal_secs,
        "Enter 5-600 seconds.",
    );
    let min_interval_days = parse_u32_field(
        &form.min_interval_days,
        &mut errors.min_interval_days,
        "Enter a positive number.",
    );
    let max_interval_days = parse_u32_field(
        &form.max_interval_days,
        &mut errors.max_interval_days,
        "Enter a positive number.",
    );
    if min_interval_days > 0 && max_interval_days > 0 && min_interval_days > max_interval_days {
        errors.min_interval_days = Some("Must be <= maximum interval.");
        errors.max_interval_days = Some("Must be >= minimum interval.");
    }
    let easy_day_load_factor = parse_retention_field(
        &form.easy_day_load_factor,
        &mut errors.easy_day_load_factor,
        "Enter a value between 0 and 1.",
    );
    let easy_days_mask = form.easy_days_mask;
    if form.easy_days_enabled && easy_days_mask == 0 {
        errors.easy_days_mask = Some("Pick at least one day.");
    }
    let fsrs_target_retention = parse_retention_field(
        &form.fsrs_target_retention,
        &mut errors.fsrs_target_retention,
        "Enter a value between 0 and 1.",
    );
    let fsrs_optimize_after = parse_u32_field(
        &form.fsrs_optimize_after,
        &mut errors.fsrs_optimize_after,
        "Enter a positive number.",
    );

    ParsedSettings {
        name,
        new_cards_per_day,
        review_limit_per_day,
        micro_session_size,
        lapse_min_interval_secs,
        soft_time_reminder_secs,
        auto_reveal_secs,
        min_interval_days,
        max_interval_days,
        easy_day_load_factor,
        easy_days_mask,
        fsrs_target_retention,
        fsrs_optimize_after,
    }
}

fn parse_required_name(form: &DeckSettingsForm, errors: &mut DeckSettingsErrors) -> String {
    let name = form.name.trim();
    if name.is_empty() {
        errors.name = Some("Deck name is required.");
    }
    name.to_string()
}

fn parse_u32_field(
    value: &str,
    error_slot: &mut Option<&'static str>,
    message: &'static str,
) -> u32 {
    parse_positive_u32(value).unwrap_or_else(|| {
        *error_slot = Some(message);
        0
    })
}

fn parse_duration_field(
    value: &str,
    error_slot: &mut Option<&'static str>,
    message: &'static str,
) -> u32 {
    parse_lapse_interval_secs(value).unwrap_or_else(|| {
        *error_slot = Some(message);
        0
    })
}

fn parse_timer_field(
    value: &str,
    error_slot: &mut Option<&'static str>,
    message: &'static str,
) -> u32 {
    parse_timer_secs(value).unwrap_or_else(|| {
        *error_slot = Some(message);
        0
    })
}

fn parse_retention_field(
    value: &str,
    error_slot: &mut Option<&'static str>,
    message: &'static str,
) -> f32 {
    parse_retention(value).unwrap_or_else(|| {
        *error_slot = Some(message);
        0.0
    })
}

fn map_deck_settings_error(
    error: &learn_core::model::DeckError,
) -> Box<DeckSettingsErrors> {
    let mut errors = DeckSettingsErrors::default();
    match *error {
        learn_core::model::DeckError::InvalidMicroSessionSize => {
            errors.micro_session_size = Some("Enter a positive number.");
        }
        learn_core::model::DeckError::InvalidNewCardsPerDay => {
            errors.new_cards_per_day = Some("Enter a positive number.");
        }
        learn_core::model::DeckError::InvalidReviewLimitPerDay => {
            errors.review_limit_per_day = Some("Enter a positive number.");
        }
        learn_core::model::DeckError::InvalidLapseMinInterval => {
            errors.lapse_min_interval = Some("Use a duration like 10m or 1d.");
        }
        learn_core::model::DeckError::InvalidSoftReminderSeconds => {
            errors.soft_time_reminder_secs = Some("Enter 5-600 seconds.");
        }
        learn_core::model::DeckError::InvalidAutoRevealSeconds => {
            errors.auto_reveal_secs = Some("Enter 5-600 seconds.");
        }
        learn_core::model::DeckError::InvalidMinIntervalDays => {
            errors.min_interval_days = Some("Enter at least 1 day.");
        }
        learn_core::model::DeckError::InvalidMaxIntervalDays => {
            errors.max_interval_days = Some("Enter at least 1 day.");
        }
        learn_core::model::DeckError::InvalidIntervalBounds => {
            errors.min_interval_days = Some("Must be <= maximum interval.");
            errors.max_interval_days = Some("Must be >= minimum interval.");
        }
        learn_core::model::DeckError::InvalidEasyDayLoadFactor => {
            errors.easy_day_load_factor = Some("Enter a value between 0 and 1.");
        }
        learn_core::model::DeckError::InvalidEasyDaysMask => {
            errors.easy_days_mask = Some("Pick at least one day.");
        }
        learn_core::model::DeckError::InvalidFsrsTargetRetention => {
            errors.fsrs_target_retention = Some("Enter a value between 0 and 1.");
        }
        learn_core::model::DeckError::InvalidFsrsOptimizeAfter => {
            errors.fsrs_optimize_after = Some("Enter a positive number.");
        }
        learn_core::model::DeckError::EmptyName => {
            errors.name = Some("Deck name is required.");
        }
        _ => {
            errors.name = Some("Invalid deck settings.");
        }
    }
    Box::new(errors)
}

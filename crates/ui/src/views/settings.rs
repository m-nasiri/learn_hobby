use dioxus::document::eval;
use dioxus::prelude::*;
use dioxus_router::use_navigator;
use learn_core::model::{Deck, DeckId, DeckSettings};

use crate::context::AppContext;
use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Debug, PartialEq)]
struct DeckSettingsData {
    deck: Deck,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
struct DeckSettingsSnapshot {
    deck_id: DeckId,
    name: String,
    description: Option<String>,
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
    easy_days_enabled: bool,
    easy_day_load_factor: f32,
    easy_days_mask: u8,
    fsrs_target_retention: f32,
    fsrs_optimize_enabled: bool,
    fsrs_optimize_after: u32,
}

impl DeckSettingsSnapshot {
    fn from_deck(deck: &Deck) -> Self {
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
            easy_days_enabled: settings.easy_days_enabled(),
            easy_day_load_factor: settings.easy_day_load_factor(),
            easy_days_mask: settings.easy_days_mask(),
            fsrs_target_retention: settings.fsrs_target_retention(),
            fsrs_optimize_enabled: settings.fsrs_optimize_enabled(),
            fsrs_optimize_after: settings.fsrs_optimize_after(),
        }
    }

    fn from_validated(deck_id: DeckId, validated: &ValidatedSettings) -> Self {
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
struct DeckSettingsForm {
    name: String,
    description: String,
    new_cards_per_day: String,
    review_limit_per_day: String,
    micro_session_size: String,
    protect_overload: bool,
    preserve_stability_on_lapse: bool,
    lapse_min_interval: String,
    show_timer: bool,
    soft_time_reminder: bool,
    auto_advance_cards: bool,
    soft_time_reminder_secs: String,
    auto_reveal_secs: String,
    easy_days_enabled: bool,
    easy_day_load_factor: String,
    easy_days_mask: u8,
    fsrs_target_retention: String,
    fsrs_optimize_enabled: bool,
    fsrs_optimize_after: String,
    max_interval_days: String,
    min_interval_days: String,
    fsrs_parameters: String,
}

impl DeckSettingsForm {
    fn from_snapshot(snapshot: &DeckSettingsSnapshot) -> Self {
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
            max_interval_days: "365".to_string(),
            min_interval_days: "1".to_string(),
            fsrs_parameters: "0.2120, 1.2931, 2.3065, 8.2956, 6.4133, 0.8334, 3.0194, 0.0010, 1.8722, 0.1666, 0.7960, 1.4835, 0.0614, 0.2629, 1.6483, 0.6014, 1.8729, 0.5425, 0.0912, 0.0658, 0.1542".to_string(),
        }
    }

    fn to_snapshot(&self, deck_id: DeckId) -> Option<DeckSettingsSnapshot> {
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
        let easy_day_load_factor = parse_retention(&self.easy_day_load_factor)?;
        if self.easy_days_enabled && self.easy_days_mask == 0 {
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
struct DeckSettingsErrors {
    name: Option<&'static str>,
    new_cards_per_day: Option<&'static str>,
    review_limit_per_day: Option<&'static str>,
    micro_session_size: Option<&'static str>,
    lapse_min_interval: Option<&'static str>,
    soft_time_reminder_secs: Option<&'static str>,
    auto_reveal_secs: Option<&'static str>,
    easy_day_load_factor: Option<&'static str>,
    easy_days_mask: Option<&'static str>,
    fsrs_target_retention: Option<&'static str>,
    fsrs_optimize_after: Option<&'static str>,
}

impl DeckSettingsErrors {
    fn has_any(&self) -> bool {
        self.name.is_some()
            || self.new_cards_per_day.is_some()
            || self.review_limit_per_day.is_some()
            || self.micro_session_size.is_some()
            || self.lapse_min_interval.is_some()
            || self.soft_time_reminder_secs.is_some()
            || self.auto_reveal_secs.is_some()
            || self.easy_day_load_factor.is_some()
            || self.easy_days_mask.is_some()
            || self.fsrs_target_retention.is_some()
            || self.fsrs_optimize_after.is_some()
    }
}

struct ValidatedSettings {
    name: String,
    description: Option<String>,
    settings: DeckSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsSection {
    DailyLimits,
    Lapses,
    Fsrs,
    Audio,
    Timers,
    EasyDays,
    Advanced,
}

impl SettingsSection {
    fn anchor_id(self) -> &'static str {
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
enum SaveState {
    Idle,
    Saving,
    Saved,
    Error(ViewError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResetState {
    Idle,
    Resetting,
    Error(ViewError),
}

#[component]
pub fn SettingsView(deck_id: Option<u64>) -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let deck_id = deck_id.map_or_else(|| ctx.current_deck_id(), DeckId::new);
    let deck_service = ctx.deck_service();
    let deck_service_for_resource = deck_service.clone();
    let card_service = ctx.card_service();

    let mut form = use_signal(DeckSettingsForm::default);
    let mut errors = use_signal(DeckSettingsErrors::default);
    let mut save_state = use_signal(|| SaveState::Idle);
    let mut initial_snapshot = use_signal(|| None::<DeckSettingsSnapshot>);
    let mut show_reset_modal = use_signal(|| false);
    let mut reset_state = use_signal(|| ResetState::Idle);
    let active_section = use_signal(|| SettingsSection::DailyLimits);
    let mut search = use_signal(String::new);
    let expanded_section = use_signal(|| Some(SettingsSection::DailyLimits));
    let mut autoplay_audio = use_signal(|| true);
    let mut replay_audio_after_answer = use_signal(|| false);
    let mut audio_delay_ms = use_signal(|| "300".to_string());

    let resource = use_resource(move || {
        let deck_service = deck_service_for_resource.clone();
        async move {
            let deck = deck_service
                .get_deck(deck_id)
                .await
                .map_err(|_| ViewError::Unknown)?
                .ok_or(ViewError::Unknown)?;
            Ok::<_, ViewError>(DeckSettingsData { deck })
        }
    });
    let state = view_state_from_resource(&resource);

    use_effect(move || {
        let deck = resource
            .value()
            .read()
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .map(|data| data.deck.clone());
        if let Some(deck) = deck {
            let should_reset = initial_snapshot()
                .as_ref()
                .is_none_or(|snapshot| snapshot.deck_id != deck.id());
            if should_reset {
                let snapshot = DeckSettingsSnapshot::from_deck(&deck);
                initial_snapshot.set(Some(snapshot.clone()));
                form.set(DeckSettingsForm::from_snapshot(&snapshot));
                errors.set(DeckSettingsErrors::default());
                save_state.set(SaveState::Idle);
            }
        }
    });

    let current_snapshot = initial_snapshot();
    let form_value = form();
    let form_snapshot = current_snapshot
        .as_ref()
        .and_then(|snapshot| form_value.to_snapshot(snapshot.deck_id));
    let has_valid_form = form_snapshot.is_some();
    let is_dirty = current_snapshot
        .as_ref()
        .is_some_and(|snapshot| form_snapshot.as_ref() != Some(snapshot));

    let status_label = match save_state() {
        SaveState::Saving => Some("Saving..."),
        SaveState::Saved => Some("Saved"),
        SaveState::Error(_) => Some("Couldn't save"),
        SaveState::Idle if is_dirty => Some("Unsaved changes"),
        SaveState::Idle => None,
    };

    let on_save = {
        let deck_service = deck_service.clone();
        let form = form;
        let mut errors = errors;
        let save_state = save_state;
        let initial_snapshot = initial_snapshot;
        use_callback(move |()| {
            let form_value = form();
            match validate_form(&form_value) {
                Ok(validated) => {
                    errors.set(DeckSettingsErrors::default());
                    let deck_service = deck_service.clone();
                        let mut save_state = save_state;
                        let mut initial_snapshot = initial_snapshot;
                        let mut form = form;
                    spawn(async move {
                        save_state.set(SaveState::Saving);
                        match deck_service
                            .update_deck(
                                deck_id,
                                validated.name.clone(),
                                validated.description.clone(),
                                validated.settings.clone(),
                            )
                            .await
                        {
                            Ok(()) => {
                                let snapshot =
                                    DeckSettingsSnapshot::from_validated(deck_id, &validated);
                                initial_snapshot.set(Some(snapshot.clone()));
                                form.set(DeckSettingsForm::from_snapshot(&snapshot));
                                save_state.set(SaveState::Saved);
                            }
                            Err(_) => {
                                save_state.set(SaveState::Error(ViewError::Unknown));
                            }
                        }
                    });
                }
                Err(next_errors) => {
                    errors.set(*next_errors);
                }
            }
        })
    };

    let on_restore_defaults = {
        let mut form = form;
        let mut errors = errors;
        let mut save_state = save_state;
        use_callback(move |()| {
            let defaults = DeckSettings::default_for_adhd();
            let mut next = form();
            next.new_cards_per_day = defaults.new_cards_per_day().to_string();
        next.review_limit_per_day = defaults.review_limit_per_day().to_string();
        next.micro_session_size = defaults.micro_session_size().to_string();
        next.protect_overload = defaults.protect_overload();
        next.preserve_stability_on_lapse = defaults.preserve_stability_on_lapse();
        next.lapse_min_interval = format_lapse_interval(defaults.lapse_min_interval_secs());
        next.show_timer = defaults.show_timer();
        next.soft_time_reminder = defaults.soft_time_reminder();
        next.auto_advance_cards = defaults.auto_advance_cards();
        next.soft_time_reminder_secs = defaults.soft_time_reminder_secs().to_string();
        next.auto_reveal_secs = defaults.auto_reveal_secs().to_string();
        next.easy_days_enabled = defaults.easy_days_enabled();
        next.easy_day_load_factor = format_retention(defaults.easy_day_load_factor());
        next.easy_days_mask = defaults.easy_days_mask();
        next.fsrs_target_retention = format_retention(defaults.fsrs_target_retention());
        next.fsrs_optimize_enabled = defaults.fsrs_optimize_enabled();
        next.fsrs_optimize_after = defaults.fsrs_optimize_after().to_string();
        form.set(next);
        errors.set(DeckSettingsErrors::default());
        save_state.set(SaveState::Idle);
    })
    };

    let deck_title = form_value.name.trim().to_string();
    let deck_title = if deck_title.is_empty() {
        current_snapshot
            .as_ref()
            .map_or_else(|| "Deck".to_string(), |snapshot| snapshot.name.clone())
    } else {
        deck_title
    };

    let easy_day_options = [
        ("Mon", 1_u8 << 0, "Monday"),
        ("Tue", 1_u8 << 1, "Tuesday"),
        ("Wed", 1_u8 << 2, "Wednesday"),
        ("Thu", 1_u8 << 3, "Thursday"),
        ("Fri", 1_u8 << 4, "Friday"),
        ("Sat", 1_u8 << 5, "Saturday"),
        ("Sun", 1_u8 << 6, "Sunday"),
    ];

    let on_nav_select = {
        let mut active_section = active_section;
        let mut expanded_section = expanded_section;
        use_callback(move |section: SettingsSection| {
            active_section.set(section);
            expanded_section.set(Some(section));
            let _ = eval(&format!(
                "document.getElementById('{}')?.scrollIntoView({{behavior: 'smooth', block: 'start'}});",
                section.anchor_id()
            ));
        })
    };

    rsx! {
        div { class: "page settings-page",
            div { class: "settings-shell",
                aside { class: "settings-nav",
                    button {
                        class: "settings-nav-header",
                        r#type: "button",
                        onclick: move |_| navigator.go_back(),
                        span { class: "settings-nav-back",
                            svg {
                                view_box: "0 0 24 24",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.8",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                path { d: "M15 6l-6 6 6 6" }
                            }
                        }
                        span { "Deck Settings" }
                    }
                    nav { class: "settings-nav-list",
                        SettingsNavItem {
                            label: "Daily Limits",
                            section: SettingsSection::DailyLimits,
                            active: active_section(),
                            on_select: on_nav_select,
                        }
                        SettingsNavItem {
                            label: "Lapses",
                            section: SettingsSection::Lapses,
                            active: active_section(),
                            on_select: on_nav_select,
                        }
                        SettingsNavItem {
                            label: "FSRS",
                            section: SettingsSection::Fsrs,
                            active: active_section(),
                            on_select: on_nav_select,
                        }
                        SettingsNavItem {
                            label: "Audio",
                            section: SettingsSection::Audio,
                            active: active_section(),
                            on_select: on_nav_select,
                        }
                        SettingsNavItem {
                            label: "Timers",
                            section: SettingsSection::Timers,
                            active: active_section(),
                            on_select: on_nav_select,
                        }
                        SettingsNavItem {
                            label: "Easy Days",
                            section: SettingsSection::EasyDays,
                            active: active_section(),
                            on_select: on_nav_select,
                        }
                        SettingsNavItem {
                            label: "Advanced",
                            section: SettingsSection::Advanced,
                            active: active_section(),
                            on_select: on_nav_select,
                        }
                    }
                    div { class: "settings-nav-footer",
                        button {
                            class: "settings-reset",
                            r#type: "button",
                            onclick: move |_| {
                                reset_state.set(ResetState::Idle);
                                show_reset_modal.set(true);
                            },
                            span { class: "settings-reset-icon",
                                svg {
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "1.8",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    path { d: "M3 12a9 9 0 1 0 3-6.7" }
                                    path { d: "M3 4v5h5" }
                                }
                            }
                            span { "Reset Deck" }
                        }
                    }
                }

                section { class: "settings-content",
                    header { class: "settings-topbar",
                        div { class: "settings-title-group",
                            h2 { class: "settings-title", "Deck Settings — {deck_title}" }
                            if let Some(label) = status_label {
                                p { class: "settings-status", "{label}" }
                            }
                        }
                        div { class: "settings-search",
                            span { class: "settings-search-icon", aria_hidden: "true",
                                svg {
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "1.8",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    circle { cx: "11", cy: "11", r: "7" }
                                    path { d: "M20 20l-3.5-3.5" }
                                }
                            }
                            input {
                                class: "settings-search-input",
                                r#type: "search",
                                placeholder: "Search",
                                value: "{search()}",
                                oninput: move |evt| search.set(evt.value()),
                            }
                        }
                    }

                    match state {
                        ViewState::Idle => rsx! {
                            p { "Idle" }
                        },
                        ViewState::Loading => rsx! {
                            p { "Loading..." }
                        },
                        ViewState::Ready(_) => {
                            let errors_value = errors();
                            rsx! {
                                section { class: "settings-accordion",
                                    SettingsAccordionSection {
                                        label: "Daily Limits",
                                        section: SettingsSection::DailyLimits,
                                        expanded: expanded_section() == Some(SettingsSection::DailyLimits),
                                        on_toggle: expanded_section,
                                        help_title: None,
                                        div { class: "settings-card",
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "new-cards", "New cards per day" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Maximum number of new cards introduced today. Keeping this low improves focus and reduces anxiety.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field",
                                                    input {
                                                        id: "new-cards",
                                                        class: if errors_value.new_cards_per_day.is_some() {
                                                            "editor-input settings-input editor-input--error"
                                                        } else {
                                                            "editor-input settings-input"
                                                        },
                                                        r#type: "number",
                                                        min: "1",
                                                        inputmode: "numeric",
                                                        value: "{form_value.new_cards_per_day}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.new_cards_per_day = evt.value();
                                                            form.set(next);
                                                            let mut next_errors = errors();
                                                            next_errors.new_cards_per_day = None;
                                                            errors.set(next_errors);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                    if let Some(message) = errors_value.new_cards_per_day {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "review-limit", "Maximum reviews per day" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Upper limit of review cards shown per day. Extra reviews are postponed to the next day.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field",
                                                    input {
                                                        id: "review-limit",
                                                        class: if errors_value.review_limit_per_day.is_some() {
                                                            "editor-input settings-input editor-input--error"
                                                        } else {
                                                            "editor-input settings-input"
                                                        },
                                                        r#type: "number",
                                                        min: "1",
                                                        inputmode: "numeric",
                                                        value: "{form_value.review_limit_per_day}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.review_limit_per_day = evt.value();
                                                            form.set(next);
                                                            let mut next_errors = errors();
                                                            next_errors.review_limit_per_day = None;
                                                            errors.set(next_errors);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                    if let Some(message) = errors_value.review_limit_per_day {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Protect from overload" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "When enabled, the system delays additional reviews instead of overwhelming you.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{form_value.protect_overload}",
                                                        onclick: move |_| {
                                                            let mut next = form();
                                                            next.protect_overload = !next.protect_overload;
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    SettingsAccordionSection {
                                        label: "Lapses",
                                        section: SettingsSection::Lapses,
                                        expanded: expanded_section() == Some(SettingsSection::Lapses),
                                        on_toggle: expanded_section,
                                        help_title: Some("Lapse settings for failed review cards."),
                                        div { class: "settings-card",
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Preserve stability on lapse" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Forgetting a card does not reset all previous progress.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{form_value.preserve_stability_on_lapse}",
                                                        onclick: move |_| {
                                                            let mut next = form();
                                                            next.preserve_stability_on_lapse =
                                                                !next.preserve_stability_on_lapse;
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "lapse-min-interval", "Minimum interval after lapse" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Small delay after pressing “Again” to avoid immediate pressure.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    input {
                                                        id: "lapse-min-interval",
                                                        class: if errors_value.lapse_min_interval.is_some() {
                                                            "editor-input settings-input editor-input--error"
                                                        } else {
                                                            "editor-input settings-input"
                                                        },
                                                        r#type: "text",
                                                        value: "{form_value.lapse_min_interval}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.lapse_min_interval = evt.value();
                                                            form.set(next);
                                                            let mut next_errors = errors();
                                                            next_errors.lapse_min_interval = None;
                                                            errors.set(next_errors);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                    p { class: "settings-field-hint", "Use 10m, 2h, 1d." }
                                                    if let Some(message) = errors_value.lapse_min_interval {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    SettingsAccordionSection {
                                        label: "FSRS",
                                        section: SettingsSection::Fsrs,
                                        expanded: expanded_section() == Some(SettingsSection::Fsrs),
                                        on_toggle: expanded_section,
                                        help_title: Some("Controls memory retention behavior."),
                                        div { class: "settings-card",
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "fsrs-retention", "Target retention" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Desired probability of remembering a card. 0.85 balances speed and long-term retention.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    input {
                                                        id: "fsrs-retention",
                                                        class: if errors_value.fsrs_target_retention.is_some() {
                                                            "editor-input settings-input editor-input--error"
                                                        } else {
                                                            "editor-input settings-input"
                                                        },
                                                        r#type: "text",
                                                        value: "{form_value.fsrs_target_retention}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.fsrs_target_retention = evt.value();
                                                            form.set(next);
                                                            let mut next_errors = errors();
                                                            next_errors.fsrs_target_retention = None;
                                                            errors.set(next_errors);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                    if let Some(message) = errors_value.fsrs_target_retention {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Enable FSRS optimization" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Allows FSRS to adapt scheduling based on your review history.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{form_value.fsrs_optimize_enabled}",
                                                        onclick: move |_| {
                                                            let mut next = form();
                                                            next.fsrs_optimize_enabled =
                                                                !next.fsrs_optimize_enabled;
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "fsrs-optimize-after", "Start optimization after" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Minimum number of reviews required before FSRS begins self-optimizing.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    input {
                                                        id: "fsrs-optimize-after",
                                                        class: if errors_value.fsrs_optimize_after.is_some() {
                                                            "editor-input settings-input editor-input--error"
                                                        } else {
                                                            "editor-input settings-input"
                                                        },
                                                        r#type: "number",
                                                        min: "0",
                                                        inputmode: "numeric",
                                                        value: "{form_value.fsrs_optimize_after}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.fsrs_optimize_after = evt.value();
                                                            form.set(next);
                                                            let mut next_errors = errors();
                                                            next_errors.fsrs_optimize_after = None;
                                                            errors.set(next_errors);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                    if let Some(message) = errors_value.fsrs_optimize_after {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    SettingsAccordionSection {
                                        label: "Audio",
                                        section: SettingsSection::Audio,
                                        expanded: expanded_section() == Some(SettingsSection::Audio),
                                        on_toggle: expanded_section,
                                        help_title: Some("Language learning audio support."),
                                        div { class: "settings-card",
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Auto-play audio" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Automatically plays audio when the card appears.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{autoplay_audio()}",
                                                        onclick: move |_| autoplay_audio.set(!autoplay_audio()),
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Replay audio after answer" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Plays audio again after revealing the answer.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{replay_audio_after_answer()}",
                                                        onclick: move |_| {
                                                            replay_audio_after_answer
                                                                .set(!replay_audio_after_answer());
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "audio-delay", "Audio delay" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Short delay before playback to improve focus.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    div { class: "settings-inline-input",
                                                        input {
                                                            id: "audio-delay",
                                                            class: "editor-input settings-input",
                                                            r#type: "number",
                                                            min: "0",
                                                            inputmode: "numeric",
                                                            value: "{audio_delay_ms()}",
                                                            oninput: move |evt| audio_delay_ms.set(evt.value()),
                                                        }
                                                        span { class: "settings-inline-suffix", "ms" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    SettingsAccordionSection {
                                        label: "Timers",
                                        section: SettingsSection::Timers,
                                        expanded: expanded_section() == Some(SettingsSection::Timers),
                                        on_toggle: expanded_section,
                                        help_title: Some("Minimize stress during reviews."),
                                        div { class: "settings-card",
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Show timer" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Timers can increase anxiety. Disabled by default.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{form_value.show_timer}",
                                                        onclick: move |_| {
                                                            let mut next = form();
                                                            next.show_timer = !next.show_timer;
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Soft time reminder" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Gentle reminder instead of hard time limits.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{form_value.soft_time_reminder}",
                                                        onclick: move |_| {
                                                            let mut next = form();
                                                            next.soft_time_reminder =
                                                                !next.soft_time_reminder;
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "soft-reminder-secs", "Soft reminder after" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "How many seconds before the gentle reminder appears.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    div { class: "settings-inline-input",
                                                        input {
                                                            id: "soft-reminder-secs",
                                                            class: if errors_value.soft_time_reminder_secs.is_some() {
                                                                "editor-input settings-input editor-input--error"
                                                            } else {
                                                                "editor-input settings-input"
                                                            },
                                                            r#type: "number",
                                                            min: "5",
                                                            max: "600",
                                                            inputmode: "numeric",
                                                            value: "{form_value.soft_time_reminder_secs}",
                                                            disabled: "{!form_value.soft_time_reminder}",
                                                            oninput: move |evt| {
                                                                let mut next = form();
                                                                next.soft_time_reminder_secs = evt.value();
                                                                form.set(next);
                                                                let mut next_errors = errors();
                                                                next_errors.soft_time_reminder_secs = None;
                                                                errors.set(next_errors);
                                                                save_state.set(SaveState::Idle);
                                                            },
                                                        }
                                                        span { class: "settings-inline-suffix", "sec" }
                                                    }
                                                    if let Some(message) = errors_value.soft_time_reminder_secs {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Auto-advance cards" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Keeps the user fully in control of pacing.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{form_value.auto_advance_cards}",
                                                        onclick: move |_| {
                                                            let mut next = form();
                                                            next.auto_advance_cards =
                                                                !next.auto_advance_cards;
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "auto-reveal-secs", "Auto reveal after" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Seconds before the answer is revealed automatically.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    div { class: "settings-inline-input",
                                                        input {
                                                            id: "auto-reveal-secs",
                                                            class: if errors_value.auto_reveal_secs.is_some() {
                                                                "editor-input settings-input editor-input--error"
                                                            } else {
                                                                "editor-input settings-input"
                                                            },
                                                            r#type: "number",
                                                            min: "5",
                                                            max: "600",
                                                            inputmode: "numeric",
                                                            value: "{form_value.auto_reveal_secs}",
                                                            disabled: "{!form_value.auto_advance_cards}",
                                                            oninput: move |evt| {
                                                                let mut next = form();
                                                                next.auto_reveal_secs = evt.value();
                                                                form.set(next);
                                                                let mut next_errors = errors();
                                                                next_errors.auto_reveal_secs = None;
                                                                errors.set(next_errors);
                                                                save_state.set(SaveState::Idle);
                                                            },
                                                        }
                                                        span { class: "settings-inline-suffix", "sec" }
                                                    }
                                                    if let Some(message) = errors_value.auto_reveal_secs {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    SettingsAccordionSection {
                                        label: "Easy Days",
                                        section: SettingsSection::EasyDays,
                                        expanded: expanded_section() == Some(SettingsSection::EasyDays),
                                        on_toggle: expanded_section,
                                        help_title: Some("Support low-energy or busy days."),
                                        div { class: "settings-card",
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Enable easy days" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Reduces daily workload on selected days.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{form_value.easy_days_enabled}",
                                                        onclick: move |_| {
                                                            let mut next = form();
                                                            next.easy_days_enabled = !next.easy_days_enabled;
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "easy-day-factor", "Easy day load factor" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Percentage of normal review volume on easy days.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    input {
                                                        id: "easy-day-factor",
                                                        class: if errors_value.easy_day_load_factor.is_some() {
                                                            "editor-input settings-input editor-input--error"
                                                        } else {
                                                            "editor-input settings-input"
                                                        },
                                                        r#type: "text",
                                                        value: "{form_value.easy_day_load_factor}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.easy_day_load_factor = evt.value();
                                                            form.set(next);
                                                            let mut next_errors = errors();
                                                            next_errors.easy_day_load_factor = None;
                                                            errors.set(next_errors);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                    if let Some(message) = errors_value.easy_day_load_factor {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "easy-days", "Easy days" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Days with intentionally reduced cognitive load.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    div { class: "settings-day-picker",
                                                        for (label, bit, title) in easy_day_options {
                                                            button {
                                                                class: if form_value.easy_days_mask & bit != 0 {
                                                                    "settings-pill settings-pill--active"
                                                                } else {
                                                                    "settings-pill"
                                                                },
                                                                r#type: "button",
                                                                title: "{title}",
                                                                disabled: "{!form_value.easy_days_enabled}",
                                                                onclick: move |_| {
                                                                    let mut next = form();
                                                                    if next.easy_days_mask & bit != 0 {
                                                                        next.easy_days_mask &= !bit;
                                                                    } else {
                                                                        next.easy_days_mask |= bit;
                                                                    }
                                                                    form.set(next);
                                                                    let mut next_errors = errors();
                                                                    next_errors.easy_days_mask = None;
                                                                    errors.set(next_errors);
                                                                    save_state.set(SaveState::Idle);
                                                                },
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                    if let Some(message) = errors_value.easy_days_mask {
                                                        p { class: "editor-error", "{message}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    SettingsAccordionSection {
                                        label: "Advanced",
                                        section: SettingsSection::Advanced,
                                        expanded: expanded_section() == Some(SettingsSection::Advanced),
                                        on_toggle: expanded_section,
                                        help_title: Some("Power-user settings."),
                                        div { class: "settings-card",
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "max-interval", "Maximum interval" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Upper bound for review intervals.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    div { class: "settings-inline-input",
                                                        input {
                                                            id: "max-interval",
                                                            class: "editor-input settings-input",
                                                            r#type: "number",
                                                            min: "1",
                                                            inputmode: "numeric",
                                                            value: "{form_value.max_interval_days}",
                                                            oninput: move |evt| {
                                                                let mut next = form();
                                                                next.max_interval_days = evt.value();
                                                                form.set(next);
                                                                save_state.set(SaveState::Idle);
                                                            },
                                                        }
                                                        span { class: "settings-inline-suffix", "days" }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "min-interval", "Minimum interval" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Prevents overly frequent reviews.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    div { class: "settings-inline-input",
                                                        input {
                                                            id: "min-interval",
                                                            class: "editor-input settings-input",
                                                            r#type: "number",
                                                            min: "1",
                                                            inputmode: "numeric",
                                                            value: "{form_value.min_interval_days}",
                                                            oninput: move |evt| {
                                                                let mut next = form();
                                                                next.min_interval_days = evt.value();
                                                                form.set(next);
                                                                save_state.set(SaveState::Idle);
                                                            },
                                                        }
                                                        span { class: "settings-inline-suffix", "days" }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { r#for: "fsrs-params-readonly", "FSRS parameters" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Displays current learned parameters.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--wide",
                                                    textarea {
                                                        id: "fsrs-params-readonly",
                                                        class: "editor-input settings-input settings-fsrs-textarea",
                                                        rows: "3",
                                                        value: "{form_value.fsrs_parameters}",
                                                        disabled: true,
                                                    }
                                                }
                                            }
                                        }
                                        button {
                                            class: "btn settings-danger",
                                            r#type: "button",
                                            onclick: move |_| {
                                                reset_state.set(ResetState::Idle);
                                                show_reset_modal.set(true);
                                            },
                                            title: "Resets all scheduling data for this deck.",
                                            "Reset FSRS data"
                                        }
                                        p { class: "settings-inline-note", "Not wired yet." }
                                    }
                                }

                                footer { class: "settings-footer",
                                    if let Some(label) = status_label {
                                        span { class: "settings-footer-status", "{label}" }
                                    }
                                    div { class: "settings-footer-actions",
                                        button {
                                            class: "btn btn-secondary",
                                            r#type: "button",
                                            onclick: move |_| on_restore_defaults.call(()),
                                            "Restore Defaults"
                                        }
                                        button {
                                            class: "btn btn-primary",
                                            r#type: "button",
                                            disabled: !has_valid_form || !is_dirty || save_state() == SaveState::Saving,
                                            onclick: move |_| on_save.call(()),
                                            "Save"
                                        }
                                    }
                                }
                            }
                        },
                        ViewState::Error(err) => rsx! {
                            p { "{err.message()}" }
                            button {
                                class: "btn btn-secondary",
                                r#type: "button",
                                onclick: move |_| {
                                    let mut resource = resource;
                                    resource.restart();
                                },
                                "Retry"
                            }
                        },
                    }
                }
            }
            if show_reset_modal() {
                div {
                    class: "editor-modal-overlay",
                    onclick: move |_| {
                        show_reset_modal.set(false);
                        reset_state.set(ResetState::Idle);
                    },
                    div {
                        class: "editor-modal",
                        onclick: move |evt| evt.stop_propagation(),
                        h3 { class: "editor-modal-title", "Reset deck learning?" }
                        p { class: "editor-modal-body",
                            "This resets scheduling for every card in this deck."
                        }
                        if let ResetState::Error(err) = reset_state() {
                            p { class: "editor-modal-error", "{err.message()}" }
                        }
                        div { class: "editor-modal-actions",
                            button {
                                class: "btn editor-modal-cancel",
                                r#type: "button",
                                onclick: move |_| {
                                    show_reset_modal.set(false);
                                    reset_state.set(ResetState::Idle);
                                },
                                "Cancel"
                            }
                            button {
                                class: "btn editor-modal-confirm",
                                r#type: "button",
                                disabled: reset_state() == ResetState::Resetting,
                                onclick: move |_| {
                                    let mut reset_state = reset_state;
                                    let mut show_reset_modal = show_reset_modal;
                                    let card_service = card_service.clone();
                                    spawn(async move {
                                        reset_state.set(ResetState::Resetting);
                                        match card_service.reset_deck_learning(deck_id).await {
                                            Ok(_) => {
                                                reset_state.set(ResetState::Idle);
                                                show_reset_modal.set(false);
                                            }
                                            Err(_) => {
                                                reset_state.set(ResetState::Error(ViewError::Unknown));
                                            }
                                        }
                                    });
                                },
                                "Reset"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SettingsNavItem(
    label: &'static str,
    section: SettingsSection,
    active: SettingsSection,
    on_select: Callback<SettingsSection>,
) -> Element {
    let is_active = active == section;
    rsx! {
        button {
            class: if is_active {
                "settings-nav-item settings-nav-item--active"
            } else {
                "settings-nav-item"
            },
            r#type: "button",
            onclick: move |_| on_select.call(section),
            span { class: "settings-nav-icon",
                SettingsNavIcon { section }
            }
            span { class: "settings-nav-label", "{label}" }
        }
    }
}

#[component]
fn SettingsNavIcon(section: SettingsSection) -> Element {
    let path = match section {
        SettingsSection::DailyLimits => "M4 6h16M4 12h10M4 18h12",
        SettingsSection::Lapses => "M4 8l8 8 8-8",
        SettingsSection::Fsrs => "M4 12h16M12 4v16",
        SettingsSection::Audio => "M5 9h4l5-4v14l-5-4H5z",
        SettingsSection::Timers => "M12 6v6l4 2",
        SettingsSection::EasyDays => "M12 4v16M4 12h16",
        SettingsSection::Advanced => "M12 2l3 6 6 1-4 4 1 6-6-3-6 3 1-6-4-4 6-1z",
    };
    let view_box = "0 0 24 24";
    rsx! {
        svg {
            view_box: view_box,
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.7",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: path }
        }
    }
}

#[component]
fn SettingsAccordionSection(
    label: &'static str,
    section: SettingsSection,
    expanded: bool,
    on_toggle: Signal<Option<SettingsSection>>,
    help_title: Option<&'static str>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "settings-accordion-section", id: "{section.anchor_id()}",
            button {
                class: if expanded {
                    "settings-accordion-header settings-accordion-header--open"
                } else {
                    "settings-accordion-header"
                },
                r#type: "button",
                onclick: move |_| {
                    if expanded {
                        on_toggle.set(None);
                    } else {
                        on_toggle.set(Some(section));
                    }
                },
                span { "{label}" }
                span { class: "settings-accordion-trailing",
                    if let Some(help) = help_title {
                        span { class: "settings-accordion-help", title: "{help}", "?" }
                    }
                    span { class: "settings-accordion-caret" }
                }
            }
            if expanded {
                div { class: "settings-accordion-body",
                    {children}
                }
            }
        }
    }
}

fn validate_form(form: &DeckSettingsForm) -> Result<ValidatedSettings, Box<DeckSettingsErrors>> {
    let mut errors = DeckSettingsErrors::default();

    let name = form.name.trim();
    if name.is_empty() {
        errors.name = Some("Deck name is required.");
    }

    let new_cards_per_day = parse_positive_u32(&form.new_cards_per_day).unwrap_or_else(|| {
        errors.new_cards_per_day = Some("Enter a positive number.");
        0
    });
    let review_limit_per_day = parse_positive_u32(&form.review_limit_per_day).unwrap_or_else(|| {
        errors.review_limit_per_day = Some("Enter a positive number.");
        0
    });
    let micro_session_size = parse_positive_u32(&form.micro_session_size).unwrap_or_else(|| {
        errors.micro_session_size = Some("Enter a positive number.");
        0
    });
    let lapse_min_interval_secs = parse_lapse_interval_secs(&form.lapse_min_interval)
        .unwrap_or_else(|| {
            errors.lapse_min_interval = Some("Use a duration like 10m or 1d.");
            0
        });
    let soft_time_reminder_secs = parse_timer_secs(&form.soft_time_reminder_secs).unwrap_or_else(|| {
        errors.soft_time_reminder_secs = Some("Enter 5-600 seconds.");
        0
    });
    let auto_reveal_secs = parse_timer_secs(&form.auto_reveal_secs).unwrap_or_else(|| {
        errors.auto_reveal_secs = Some("Enter 5-600 seconds.");
        0
    });
    let easy_day_load_factor = parse_retention(&form.easy_day_load_factor).unwrap_or_else(|| {
        errors.easy_day_load_factor = Some("Enter a value between 0 and 1.");
        0.0
    });
    let easy_days_mask = form.easy_days_mask;
    if form.easy_days_enabled && easy_days_mask == 0 {
        errors.easy_days_mask = Some("Pick at least one day.");
    }
    let fsrs_target_retention = parse_retention(&form.fsrs_target_retention).unwrap_or_else(|| {
        errors.fsrs_target_retention = Some("Enter a value between 0 and 1.");
        0.0
    });
    let fsrs_optimize_after = parse_positive_u32(&form.fsrs_optimize_after).unwrap_or_else(|| {
        errors.fsrs_optimize_after = Some("Enter a positive number.");
        0
    });

    if errors.has_any() {
        return Err(Box::new(errors));
    }

    let settings = DeckSettings::new(
        new_cards_per_day,
        review_limit_per_day,
        micro_session_size,
        form.protect_overload,
        form.preserve_stability_on_lapse,
        lapse_min_interval_secs,
        form.show_timer,
        form.soft_time_reminder,
        form.auto_advance_cards,
        soft_time_reminder_secs,
        auto_reveal_secs,
        form.easy_days_enabled,
        easy_day_load_factor,
        easy_days_mask,
        fsrs_target_retention,
        form.fsrs_optimize_enabled,
        fsrs_optimize_after,
    )
    .map_err(|err| {
        let mut errors = DeckSettingsErrors::default();
        match err {
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
    })?;

    Ok(ValidatedSettings {
        name: name.to_string(),
        description: normalize_description(&form.description),
        settings,
    })
}

fn parse_positive_u32(value: &str) -> Option<u32> {
    let value = value.trim();
    let parsed = value.parse::<u32>().ok()?;
    if parsed == 0 {
        return None;
    }
    Some(parsed)
}

fn parse_lapse_interval_secs(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.to_ascii_lowercase();
    let mut chars = normalized.chars();
    let last = chars.next_back();
    let (number_part, unit) = match last {
        Some(unit) if unit.is_ascii_alphabetic() => (&normalized[..normalized.len() - 1], unit),
        _ => (normalized.as_str(), 'd'),
    };
    let amount = number_part.trim().parse::<u32>().ok()?;
    if amount == 0 {
        return None;
    }
    match unit {
        's' => Some(amount),
        'm' => amount.checked_mul(60),
        'h' => amount.checked_mul(3600),
        'd' => amount.checked_mul(86_400),
        _ => None,
    }
}

fn format_lapse_interval(secs: u32) -> String {
    if secs.is_multiple_of(86_400) {
        format!("{}d", secs / 86_400)
    } else if secs.is_multiple_of(3600) {
        format!("{}h", secs / 3600)
    } else if secs.is_multiple_of(60) {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

fn parse_timer_secs(value: &str) -> Option<u32> {
    let value = value.trim();
    let parsed = value.parse::<u32>().ok()?;
    if !(5..=600).contains(&parsed) {
        return None;
    }
    Some(parsed)
}

fn parse_retention(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    let parsed = trimmed.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 || parsed > 1.0 {
        return None;
    }
    Some(parsed)
}

fn format_retention(value: f32) -> String {
    format!("{value:.2}")
}

fn normalize_description(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

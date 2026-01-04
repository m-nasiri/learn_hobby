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
struct DeckSettingsSnapshot {
    deck_id: DeckId,
    name: String,
    description: Option<String>,
    new_cards_per_day: u32,
    review_limit_per_day: u32,
    micro_session_size: u32,
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
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct DeckSettingsForm {
    name: String,
    description: String,
    new_cards_per_day: String,
    review_limit_per_day: String,
    micro_session_size: String,
    fsrs_target_retention: String,
    fsrs_optimize_after: String,
    lapse_min_interval: String,
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
            fsrs_target_retention: "0.85".to_string(),
            fsrs_optimize_after: "100".to_string(),
            lapse_min_interval: "1d".to_string(),
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

        Some(DeckSettingsSnapshot {
            deck_id,
            name: name.to_string(),
            description: normalize_description(&self.description),
            new_cards_per_day,
            review_limit_per_day,
            micro_session_size,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct DeckSettingsErrors {
    name: Option<&'static str>,
    new_cards_per_day: Option<&'static str>,
    review_limit_per_day: Option<&'static str>,
    micro_session_size: Option<&'static str>,
}

impl DeckSettingsErrors {
    fn has_any(&self) -> bool {
        self.name.is_some()
            || self.new_cards_per_day.is_some()
            || self.review_limit_per_day.is_some()
            || self.micro_session_size.is_some()
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
    Burying,
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
            SettingsSection::Burying => "settings-burying",
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
    let deck_id = deck_id.map(DeckId::new).unwrap_or_else(|| ctx.current_deck_id());
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
    let mut fsrs_optimization_enabled = use_signal(|| true);
    let mut protect_overload = use_signal(|| true);
    let mut preserve_stability_on_lapse = use_signal(|| true);
    let mut bury_related_cards = use_signal(|| true);
    let mut bury_siblings_until_next_day = use_signal(|| true);
    let mut autoplay_audio = use_signal(|| true);
    let mut replay_audio_after_answer = use_signal(|| false);
    let mut audio_delay_ms = use_signal(|| "300".to_string());
    let mut show_timer = use_signal(|| false);
    let mut soft_time_reminder = use_signal(|| false);
    let mut auto_advance_cards = use_signal(|| false);
    let mut easy_days_enabled = use_signal(|| true);
    let mut easy_day_load_factor = use_signal(|| "0.5".to_string());
    let mut easy_days_selected = use_signal(|| "Saturday, Sunday".to_string());

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
                .map_or(true, |snapshot| snapshot.deck_id != deck.id());
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
        .map_or(false, |snapshot| form_snapshot.as_ref() != Some(snapshot));

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
        use_callback(move |_| {
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
                            Ok(_) => {
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
                    errors.set(next_errors);
                }
            }
        })
    };

    let on_restore_defaults = {
        let mut form = form;
        let mut errors = errors;
        let mut save_state = save_state;
        use_callback(move |_| {
            let defaults = DeckSettings::default_for_adhd();
            let mut next = form();
            next.new_cards_per_day = defaults.new_cards_per_day().to_string();
            next.review_limit_per_day = defaults.review_limit_per_day().to_string();
            next.micro_session_size = defaults.micro_session_size().to_string();
            form.set(next);
            errors.set(DeckSettingsErrors::default());
            save_state.set(SaveState::Idle);
        })
    };

    let deck_title = form_value.name.trim().to_string();
    let deck_title = if deck_title.is_empty() {
        current_snapshot
            .as_ref()
            .map(|snapshot| snapshot.name.clone())
            .unwrap_or_else(|| "Deck".to_string())
    } else {
        deck_title
    };

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
                            label: "Burying",
                            section: SettingsSection::Burying,
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
                                                        aria_checked: "{protect_overload()}",
                                                        onclick: move |_| protect_overload.set(!protect_overload()),
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
                                                        aria_checked: "{preserve_stability_on_lapse()}",
                                                        onclick: move |_| {
                                                            preserve_stability_on_lapse
                                                                .set(!preserve_stability_on_lapse());
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
                                                        class: "editor-input settings-input",
                                                        r#type: "text",
                                                        value: "{form_value.lapse_min_interval}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.lapse_min_interval = evt.value();
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                        p { class: "settings-inline-note", "Not wired yet." }
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
                                                        class: "editor-input settings-input",
                                                        r#type: "text",
                                                        value: "{form_value.fsrs_target_retention}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.fsrs_target_retention = evt.value();
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
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
                                                        aria_checked: "{fsrs_optimization_enabled()}",
                                                        onclick: move |_| {
                                                            fsrs_optimization_enabled.set(!fsrs_optimization_enabled());
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
                                                        class: "editor-input settings-input",
                                                        r#type: "number",
                                                        min: "0",
                                                        inputmode: "numeric",
                                                        value: "{form_value.fsrs_optimize_after}",
                                                        oninput: move |evt| {
                                                            let mut next = form();
                                                            next.fsrs_optimize_after = evt.value();
                                                            form.set(next);
                                                            save_state.set(SaveState::Idle);
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                        p { class: "settings-inline-note", "Not wired yet." }
                                    }
                                    SettingsAccordionSection {
                                        label: "Burying",
                                        section: SettingsSection::Burying,
                                        expanded: expanded_section() == Some(SettingsSection::Burying),
                                        on_toggle: expanded_section,
                                        help_title: Some("Reduce interference between similar cards."),
                                        div { class: "settings-card",
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Bury related cards" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Prevents similar cards from appearing on the same day.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{bury_related_cards()}",
                                                        onclick: move |_| {
                                                            bury_related_cards.set(!bury_related_cards());
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-row__label",
                                                    label { "Bury siblings until next day" }
                                                    span {
                                                        class: "settings-row__help",
                                                        title: "Improves clarity and reduces confusion.",
                                                        "?"
                                                    }
                                                }
                                                div { class: "settings-row__field settings-row__field--toggle",
                                                    button {
                                                        class: "settings-toggle",
                                                        r#type: "button",
                                                        role: "switch",
                                                        aria_checked: "{bury_siblings_until_next_day()}",
                                                        onclick: move |_| {
                                                            bury_siblings_until_next_day
                                                                .set(!bury_siblings_until_next_day());
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                        p { class: "settings-inline-note", "Not wired yet." }
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
                                        p { class: "settings-inline-note", "Not wired yet." }
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
                                                        aria_checked: "{show_timer()}",
                                                        onclick: move |_| show_timer.set(!show_timer()),
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
                                                        aria_checked: "{soft_time_reminder()}",
                                                        onclick: move |_| soft_time_reminder.set(!soft_time_reminder()),
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
                                                        aria_checked: "{auto_advance_cards()}",
                                                        onclick: move |_| auto_advance_cards.set(!auto_advance_cards()),
                                                    }
                                                }
                                            }
                                        }
                                        p { class: "settings-inline-note", "Not wired yet." }
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
                                                        aria_checked: "{easy_days_enabled()}",
                                                        onclick: move |_| easy_days_enabled.set(!easy_days_enabled()),
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
                                                        class: "editor-input settings-input",
                                                        r#type: "text",
                                                        value: "{easy_day_load_factor()}",
                                                        oninput: move |evt| easy_day_load_factor.set(evt.value()),
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
                                                    input {
                                                        id: "easy-days",
                                                        class: "editor-input settings-input",
                                                        r#type: "text",
                                                        value: "{easy_days_selected()}",
                                                        oninput: move |evt| easy_days_selected.set(evt.value()),
                                                    }
                                                }
                                            }
                                        }
                                        p { class: "settings-inline-note", "Not wired yet." }
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
        SettingsSection::Burying => "M4 7h16M7 7v10",
        SettingsSection::Audio => "M5 9h4l5-4v14l-5-4H5z",
        SettingsSection::Timers => "M12 6v6l4 2",
        SettingsSection::EasyDays => "M12 4v16M4 12h16",
        SettingsSection::Advanced => "M12 2l3 6 6 1-4 4 1 6-6-3-6 3 1-6-4-4 6-1z",
    };
    let view_box = if matches!(section, SettingsSection::Advanced) {
        "0 0 24 24"
    } else {
        "0 0 24 24"
    };
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

fn validate_form(form: &DeckSettingsForm) -> Result<ValidatedSettings, DeckSettingsErrors> {
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

    if errors.has_any() {
        return Err(errors);
    }

    let settings = DeckSettings::new(new_cards_per_day, review_limit_per_day, micro_session_size)
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
                learn_core::model::DeckError::EmptyName => {
                    errors.name = Some("Deck name is required.");
                }
                _ => {
                    errors.name = Some("Invalid deck settings.");
                }
            }
            errors
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

fn normalize_description(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

use dioxus::document::eval;
use dioxus::prelude::*;
use dioxus_router::use_navigator;
use learn_core::model::{DeckId, DeckSettings};

use crate::context::AppContext;
use crate::views::{ViewError, ViewState, view_state_from_resource};

use super::components::{SettingsAccordionSection, SettingsNavItem};
use super::helpers::{format_lapse_interval, format_retention};
use super::state::{
    DeckSettingsData, DeckSettingsErrors, DeckSettingsForm, DeckSettingsSnapshot, ResetState,
    SaveState, SettingsSection, validate_form,
};

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
        next.min_interval_days = defaults.min_interval_days().to_string();
        next.max_interval_days = defaults.max_interval_days().to_string();
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
                                                            class: if errors_value.max_interval_days.is_some() {
                                                                "editor-input settings-input editor-input--error"
                                                            } else {
                                                                "editor-input settings-input"
                                                            },
                                                            r#type: "number",
                                                            min: "1",
                                                            inputmode: "numeric",
                                                            value: "{form_value.max_interval_days}",
                                                            oninput: move |evt| {
                                                                let mut next = form();
                                                                next.max_interval_days = evt.value();
                                                                form.set(next);
                                                                let mut next_errors = errors();
                                                                next_errors.max_interval_days = None;
                                                                errors.set(next_errors);
                                                                save_state.set(SaveState::Idle);
                                                            },
                                                        }
                                                        span { class: "settings-inline-suffix", "days" }
                                                    }
                                                    if let Some(message) = errors_value.max_interval_days {
                                                        p { class: "editor-error", "{message}" }
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
                                                            class: if errors_value.min_interval_days.is_some() {
                                                                "editor-input settings-input editor-input--error"
                                                            } else {
                                                                "editor-input settings-input"
                                                            },
                                                            r#type: "number",
                                                            min: "1",
                                                            inputmode: "numeric",
                                                            value: "{form_value.min_interval_days}",
                                                            oninput: move |evt| {
                                                                let mut next = form();
                                                                next.min_interval_days = evt.value();
                                                                form.set(next);
                                                                let mut next_errors = errors();
                                                                next_errors.min_interval_days = None;
                                                                errors.set(next_errors);
                                                                save_state.set(SaveState::Idle);
                                                            },
                                                        }
                                                        span { class: "settings-inline-suffix", "days" }
                                                    }
                                                    if let Some(message) = errors_value.min_interval_days {
                                                        p { class: "editor-error", "{message}" }
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

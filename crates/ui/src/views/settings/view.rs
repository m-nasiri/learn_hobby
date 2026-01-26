use dioxus::document::eval;
use dioxus::prelude::*;
use dioxus_router::use_navigator;
use learn_core::model::{DeckId, DeckSettings};

use crate::context::AppContext;
use crate::views::{ViewError, ViewState, view_state_from_resource};

use super::components::SettingsNavItem;
use super::helpers::{format_lapse_interval, format_retention};
use super::sections::{
    advanced_section, audio_section, daily_limits_section, easy_days_section, fsrs_section,
    lapses_section, timers_section,
};
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
    let autoplay_audio = use_signal(|| true);
    let replay_audio_after_answer = use_signal(|| false);
    let audio_delay_ms = use_signal(|| "300".to_string());

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
        next.min_interval = format_lapse_interval(defaults.min_interval_secs());
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
                            rsx! {
                                section { class: "settings-accordion",
                                    {daily_limits_section(form, errors, save_state, expanded_section)}
                                    {lapses_section(form, errors, save_state, expanded_section)}
                                    {fsrs_section(form, errors, save_state, expanded_section)}
                                    {audio_section(autoplay_audio, replay_audio_after_answer, audio_delay_ms, expanded_section)}
                                    {timers_section(form, errors, save_state, expanded_section)}
                                    {easy_days_section(form, errors, save_state, expanded_section)}
                                    {advanced_section(form, errors, save_state, reset_state, show_reset_modal, expanded_section)}
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

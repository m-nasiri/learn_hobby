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
}

impl DeckSettingsForm {
    fn from_snapshot(snapshot: &DeckSettingsSnapshot) -> Self {
        Self {
            name: snapshot.name.clone(),
            description: snapshot.description.clone().unwrap_or_default(),
            new_cards_per_day: snapshot.new_cards_per_day.to_string(),
            review_limit_per_day: snapshot.review_limit_per_day.to_string(),
            micro_session_size: snapshot.micro_session_size.to_string(),
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

    let on_discard = {
        let mut form = form;
        let mut errors = errors;
        let mut save_state = save_state;
        let current_snapshot = current_snapshot.clone();
        use_callback(move |_| {
            if let Some(snapshot) = current_snapshot.clone() {
                form.set(DeckSettingsForm::from_snapshot(&snapshot));
                errors.set(DeckSettingsErrors::default());
                save_state.set(SaveState::Idle);
            }
        })
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

    let deck_title = form_value.name.trim().to_string();
    let deck_title = if deck_title.is_empty() {
        current_snapshot
            .as_ref()
            .map(|snapshot| snapshot.name.clone())
            .unwrap_or_else(|| "Deck".to_string())
    } else {
        deck_title
    };

    rsx! {
        div { class: "page settings-page",
            header { class: "settings-header view-header",
                div { class: "settings-header__row",
                    div { class: "settings-header__title",
                        button {
                            class: "btn settings-icon-btn",
                            r#type: "button",
                            aria_label: "Back",
                            onclick: move |_| navigator.go_back(),
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
                        div { class: "settings-header__titles",
                            h2 { class: "view-title", "Deck Settings — {deck_title}" }
                            p { class: "view-subtitle",
                                "Adjust learning limits and deck metadata."
                            }
                        }
                    }
                    div { class: "settings-header__actions",
                        if let Some(label) = status_label {
                            span { class: "settings-status", "{label}" }
                        }
                        if is_dirty {
                            button {
                                class: "btn btn-secondary",
                                r#type: "button",
                                onclick: move |_| on_discard.call(()),
                                "Discard"
                            }
                            button {
                                class: "btn btn-primary",
                                r#type: "button",
                                disabled: !has_valid_form || save_state() == SaveState::Saving,
                                onclick: move |_| on_save.call(()),
                                "Save"
                            }
                        }
                        span { class: "settings-more",
                            aria_hidden: "true",
                            svg {
                                view_box: "0 0 24 24",
                                fill: "currentColor",
                                circle { cx: "5", cy: "12", r: "1.5" }
                                circle { cx: "12", cy: "12", r: "1.5" }
                                circle { cx: "19", cy: "12", r: "1.5" }
                            }
                        }
                    }
                }
            }
            div { class: "view-divider" }

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
                        section { class: "settings-section",
                            h3 { class: "settings-section-title", "General Settings" }
                            div { class: "settings-card",
                                div { class: "settings-row",
                                    div { class: "settings-row__label",
                                        label { r#for: "deck-name", "Deck Name" }
                                    }
                                    div { class: "settings-row__field settings-row__field--wide",
                                        input {
                                            id: "deck-name",
                                            class: if errors_value.name.is_some() {
                                                "editor-input settings-input editor-input--error"
                                            } else {
                                                "editor-input settings-input"
                                            },
                                            r#type: "text",
                                            value: "{form_value.name}",
                                            placeholder: "Deck name",
                                            oninput: move |evt| {
                                                let mut next = form();
                                                next.name = evt.value();
                                                form.set(next);
                                                let mut next_errors = errors();
                                                next_errors.name = None;
                                                errors.set(next_errors);
                                                save_state.set(SaveState::Idle);
                                            },
                                        }
                                        if let Some(message) = errors_value.name {
                                            p { class: "editor-error", "{message}" }
                                        }
                                    }
                                }
                                div { class: "settings-row settings-row--stack",
                                    div { class: "settings-row__label",
                                        label { r#for: "deck-description", "Description" }
                                        span { class: "settings-row__hint", "Optional" }
                                    }
                                    div { class: "settings-row__field settings-row__field--wide",
                                        textarea {
                                            id: "deck-description",
                                            class: "editor-input settings-input settings-textarea",
                                            rows: "2",
                                            placeholder: "Describe this deck...",
                                            value: "{form_value.description}",
                                            oninput: move |evt| {
                                                let mut next = form();
                                                next.description = evt.value();
                                                form.set(next);
                                                save_state.set(SaveState::Idle);
                                            },
                                        }
                                    }
                                }
                            }
                        }

                        section { class: "settings-section",
                            h3 { class: "settings-section-title", "Learning Settings" }
                            div { class: "settings-card",
                                div { class: "settings-row",
                                    div { class: "settings-row__label",
                                        label { r#for: "new-cards", "New Cards Per Day" }
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
                                        label { r#for: "review-limit", "Review Limit Per Day" }
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
                                        label { r#for: "micro-session", "Cards Per Micro-Session" }
                                    }
                                    div { class: "settings-row__field",
                                        input {
                                            id: "micro-session",
                                            class: if errors_value.micro_session_size.is_some() {
                                                "editor-input settings-input editor-input--error"
                                            } else {
                                                "editor-input settings-input"
                                            },
                                            r#type: "number",
                                            min: "1",
                                            inputmode: "numeric",
                                            value: "{form_value.micro_session_size}",
                                            oninput: move |evt| {
                                                let mut next = form();
                                                next.micro_session_size = evt.value();
                                                form.set(next);
                                                let mut next_errors = errors();
                                                next_errors.micro_session_size = None;
                                                errors.set(next_errors);
                                                save_state.set(SaveState::Idle);
                                            },
                                        }
                                        if let Some(message) = errors_value.micro_session_size {
                                            p { class: "editor-error", "{message}" }
                                        }
                                    }
                                }
                            }
                        }

                        section { class: "settings-section",
                            h3 { class: "settings-section-title", "Destructive Action" }
                            p { class: "settings-section-hint",
                                "Use with care. This resets scheduling for every card in this deck."
                            }
                            button {
                                class: "btn settings-danger",
                                r#type: "button",
                                onclick: move |_| {
                                    reset_state.set(ResetState::Idle);
                                    show_reset_modal.set(true);
                                },
                                "Reset Learning Progress"
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

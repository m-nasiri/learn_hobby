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
enum SettingsSection {
    DailyLimits,
    NewCards,
    Reviews,
    Lapses,
    DisplayOrder,
    Fsrs,
    Burying,
    Audio,
    Timers,
    AutoAdvance,
    EasyDays,
    Advanced,
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
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "New Cards",
                            section: SettingsSection::NewCards,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Reviews",
                            section: SettingsSection::Reviews,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Lapses",
                            section: SettingsSection::Lapses,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Display Order",
                            section: SettingsSection::DisplayOrder,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "FSRS",
                            section: SettingsSection::Fsrs,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Burying",
                            section: SettingsSection::Burying,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Audio",
                            section: SettingsSection::Audio,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Timers",
                            section: SettingsSection::Timers,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Auto Advance",
                            section: SettingsSection::AutoAdvance,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Easy Days",
                            section: SettingsSection::EasyDays,
                            active: active_section(),
                            on_select: active_section,
                        }
                        SettingsNavItem {
                            label: "Advanced",
                            section: SettingsSection::Advanced,
                            active: active_section(),
                            on_select: active_section,
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
                                section { class: "settings-section",
                                    h3 { class: "settings-section-title", "Daily Limits" }
                                    div { class: "settings-card",
                                        div { class: "settings-row",
                                            div { class: "settings-row__label",
                                                label { r#for: "new-cards", "Maximum new cards/day" }
                                                span { class: "settings-row__help", "?" }
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
                                                label { r#for: "review-limit", "Maximum reviews/day" }
                                                span { class: "settings-row__help", "?" }
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
                                                label { r#for: "max-total", "Maximum new + review cards/day" }
                                                span { class: "settings-row__help", "?" }
                                            }
                                            div { class: "settings-row__field",
                                                input {
                                                    id: "max-total",
                                                    class: "editor-input settings-input",
                                                    r#type: "number",
                                                    value: "0",
                                                    disabled: true,
                                                }
                                                p { class: "settings-inline-note", "Not available yet." }
                                            }
                                        }
                                    }
                                }

                                section { class: "settings-accordion",
                                    SettingsAccordionItem { label: "New Cards" }
                                    SettingsAccordionItem { label: "Reviews" }
                                    SettingsAccordionItem { label: "Lapses" }
                                    SettingsAccordionItem { label: "Display Order" }
                                    SettingsAccordionItem { label: "FSRS" }
                                    SettingsAccordionItem { label: "Burying" }
                                    SettingsAccordionItem { label: "Audio" }
                                    SettingsAccordionItem { label: "Timers" }
                                    SettingsAccordionItem { label: "Auto Advance" }
                                    SettingsAccordionItem { label: "Easy Days" }
                                    SettingsAccordionItem { label: "Advanced" }
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
    on_select: Signal<SettingsSection>,
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
            onclick: move |_| on_select.set(section),
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
        SettingsSection::NewCards => "M5 12h14M12 5v14",
        SettingsSection::Reviews => "M4 6h16v12H4z",
        SettingsSection::Lapses => "M4 8l8 8 8-8",
        SettingsSection::DisplayOrder => "M6 6h12M6 12h8M6 18h10",
        SettingsSection::Fsrs => "M4 12h16M12 4v16",
        SettingsSection::Burying => "M4 7h16M7 7v10",
        SettingsSection::Audio => "M5 9h4l5-4v14l-5-4H5z",
        SettingsSection::Timers => "M12 6v6l4 2",
        SettingsSection::AutoAdvance => "M6 6l6 6-6 6",
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
fn SettingsAccordionItem(label: &'static str) -> Element {
    rsx! {
        button {
            class: "settings-accordion-item",
            r#type: "button",
            disabled: true,
            span { "{label}" }
            span { class: "settings-accordion-caret" }
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

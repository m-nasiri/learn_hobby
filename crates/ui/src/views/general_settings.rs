use dioxus::prelude::*;

use learn_core::model::{AppSettings, AppSettingsDraft};

use crate::context::AppContext;
use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeChoice {
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    fn label(self) -> &'static str {
        match self {
            ThemeChoice::System => "System",
            ThemeChoice::Light => "Light",
            ThemeChoice::Dark => "Dark",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LanguageOption {
    English,
    German,
    Spanish,
    French,
}

impl LanguageOption {
    fn label(self) -> &'static str {
        match self {
            LanguageOption::English => "English",
            LanguageOption::German => "German",
            LanguageOption::Spanish => "Spanish",
            LanguageOption::French => "French",
        }
    }

    fn from_value(value: &str) -> Self {
        match value {
            "German" => LanguageOption::German,
            "Spanish" => LanguageOption::Spanish,
            "French" => LanguageOption::French,
            _ => LanguageOption::English,
        }
    }
}

#[derive(Clone, Copy)]
struct ModelPreset {
    value: &'static str,
    note: &'static str,
}

const MODEL_PRESETS: [ModelPreset; 3] = [
    ModelPreset {
        value: "gpt-4.1-mini",
        note: "Default: balanced quality and cost.",
    },
    ModelPreset {
        value: "gpt-4.1",
        note: "Higher quality, higher cost.",
    },
    ModelPreset {
        value: "gpt-4o-mini",
        note: "Fast and low cost for quick edits.",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccentColor {
    Blue,
    Green,
    Orange,
    Purple,
}

impl AccentColor {
    fn label(self) -> &'static str {
        match self {
            AccentColor::Blue => "Blue",
            AccentColor::Green => "Green",
            AccentColor::Orange => "Orange",
            AccentColor::Purple => "Purple",
        }
    }

    fn from_value(value: &str) -> Self {
        match value {
            "Green" => AccentColor::Green,
            "Orange" => AccentColor::Orange,
            "Purple" => AccentColor::Purple,
            _ => AccentColor::Blue,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct GeneralSettingsForm {
    language: LanguageOption,
    theme: ThemeChoice,
    appearance_enabled: bool,
    accent_color: AccentColor,
    protect_overload: bool,
    target_retention: String,
    analytics_enabled: bool,
    email: String,
    ai_api_key: String,
    ai_model: String,
    ai_fallback_model: String,
    ai_system_prompt: String,
    ai_daily_request_cap: String,
    ai_cooldown_secs: String,
}

impl Default for GeneralSettingsForm {
    fn default() -> Self {
        Self {
            language: LanguageOption::English,
            theme: ThemeChoice::System,
            appearance_enabled: true,
            accent_color: AccentColor::Blue,
            protect_overload: true,
            target_retention: "0.85".to_string(),
            analytics_enabled: false,
            email: "john.smil@gmail.com".to_string(),
            ai_api_key: String::new(),
            ai_model: String::new(),
            ai_fallback_model: String::new(),
            ai_system_prompt: String::new(),
            ai_daily_request_cap: "100".to_string(),
            ai_cooldown_secs: "5".to_string(),
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

fn apply_app_settings(form: &mut GeneralSettingsForm, settings: &AppSettings) {
    form.ai_api_key = settings.api_key().unwrap_or_default().to_string();
    form.ai_model = settings.api_model().unwrap_or_default().to_string();
    form.ai_fallback_model = settings.api_fallback_model().unwrap_or_default().to_string();
    form.ai_system_prompt = settings.ai_system_prompt().unwrap_or_default().to_string();
    form.ai_daily_request_cap = settings.ai_daily_request_cap().to_string();
    form.ai_cooldown_secs = settings.ai_cooldown_secs().to_string();
}

fn to_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_optional_u32(value: &str) -> Result<Option<u32>, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed.parse::<u32>().map(Some).map_err(|_| ())
}


#[component]
pub fn GeneralSettingsView() -> Element {
    let ctx = use_context::<AppContext>();
    let app_settings = ctx.app_settings();
    let app_settings_for_resource = app_settings.clone();
    let mut form = use_signal(GeneralSettingsForm::default);
    let mut initial = use_signal(GeneralSettingsForm::default);
    let mut save_state = use_signal(|| SaveState::Idle);
    let mut settings_loaded = use_signal(|| false);
    let mut show_model_menu = use_signal(|| false);

    let settings_resource = use_resource(move || {
        let app_settings = app_settings_for_resource.clone();
        async move {
            let settings = app_settings.load().await.map_err(|_| ViewError::Unknown)?;
            Ok::<_, ViewError>(settings)
        }
    });

    let settings_state = view_state_from_resource(&settings_resource);
    if let ViewState::Ready(settings) = settings_state
        && !settings_loaded()
    {
        let mut next = form();
        apply_app_settings(&mut next, &settings);
        form.set(next.clone());
        initial.set(next);
        settings_loaded.set(true);
    }

    let form_value = form();
    let initial_value = initial();
    let is_dirty = form_value != initial_value;

    let status_label = match save_state() {
        SaveState::Saving => Some("Saving..."),
        SaveState::Error(_) => Some("Save failed"),
        SaveState::Saved if !is_dirty => Some("Saved"),
        _ if is_dirty => Some("Unsaved changes"),
        _ => None,
    };

    rsx! {
        div { class: "page settings-page",
            section { class: "settings-content",
                if show_model_menu() {
                    div {
                        class: "settings-combo-overlay",
                        onclick: move |_| show_model_menu.set(false),
                    }
                }
                header { class: "settings-topbar",
                    div { class: "settings-title-group",
                        h2 { class: "settings-title", "General" }
                        if let Some(label) = status_label {
                            p { class: "settings-status", "{label}" }
                        }
                    }
                }

                section { class: "settings-section",
                    div { class: "settings-card",
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        circle { cx: "12", cy: "12", r: "9" }
                                        path { d: "M3 12h18" }
                                        path { d: "M12 3a15 15 0 0 1 0 18" }
                                        path { d: "M12 3a15 15 0 0 0 0 18" }
                                    }
                                }
                                span { "Language" }
                            }
                            div { class: "settings-row__field",
                                div { class: "settings-select-wrap",
                                    select {
                                        class: "settings-select",
                                        value: "{form_value.language.label()}",
                                        onchange: move |evt| {
                                            let mut next = form();
                                            next.language = LanguageOption::from_value(&evt.value());
                                            form.set(next);
                                            save_state.set(SaveState::Idle);
                                        },
                                        option { value: "English", "English" }
                                        option { value: "German", "German" }
                                        option { value: "Spanish", "Spanish" }
                                        option { value: "French", "French" }
                                    }
                                    span { class: "settings-select-caret" }
                                }
                            }
                        }

                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        circle { cx: "12", cy: "12", r: "4" }
                                        path { d: "M12 2v2" }
                                        path { d: "M12 20v2" }
                                        path { d: "M4.9 4.9l1.4 1.4" }
                                        path { d: "M17.7 17.7l1.4 1.4" }
                                        path { d: "M2 12h2" }
                                        path { d: "M20 12h2" }
                                        path { d: "M4.9 19.1l1.4-1.4" }
                                        path { d: "M17.7 6.3l1.4-1.4" }
                                    }
                                }
                                span { "Theme" }
                            }
                            div { class: "settings-row__field settings-row__field--wide",
                                div { class: "settings-segment",
                                    for choice in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark] {
                                        button {
                                            class: if form_value.theme == choice { "settings-segment__button settings-segment__button--active" } else { "settings-segment__button" },
                                            r#type: "button",
                                            onclick: move |_| {
                                                let mut next = form();
                                                next.theme = choice;
                                                form.set(next);
                                                save_state.set(SaveState::Idle);
                                            },
                                            "{choice.label()}"
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        circle { cx: "12", cy: "12", r: "9" }
                                        path { d: "M12 7v5l3 2" }
                                    }
                                }
                                span { "Appearance" }
                            }
                            div { class: "settings-row__field settings-row__field--toggle",
                                button {
                                    class: "settings-toggle",
                                    r#type: "button",
                                    aria_checked: "{form_value.appearance_enabled}",
                                    onclick: move |_| {
                                        let mut next = form();
                                        next.appearance_enabled = !next.appearance_enabled;
                                        form.set(next);
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }
                        }

                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        circle { cx: "12", cy: "12", r: "9" }
                                        path { d: "M12 3v18" }
                                        path { d: "M3 12h18" }
                                    }
                                }
                                span { "Accent color" }
                            }
                            div { class: "settings-row__field",
                                div { class: "settings-select-wrap",
                                    select {
                                        class: "settings-select",
                                        value: "{form_value.accent_color.label()}",
                                        onchange: move |evt| {
                                            let mut next = form();
                                            next.accent_color = AccentColor::from_value(&evt.value());
                                            form.set(next);
                                            save_state.set(SaveState::Idle);
                                        },
                                        option { value: "Blue", "Blue" }
                                        option { value: "Green", "Green" }
                                        option { value: "Orange", "Orange" }
                                        option { value: "Purple", "Purple" }
                                    }
                                    span { class: "settings-select-caret" }
                                }
                            }
                        }

                        div { class: "settings-row",
                            div { class: "settings-row__label settings-row__label--stacked",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        path { d: "M12 3l7 4v5c0 5-3.5 7.5-7 9-3.5-1.5-7-4-7-9V7l7-4z" }
                                    }
                                }
                                div { class: "settings-row__text",
                                    span { "Protect from overload" }
                                    span { class: "settings-row__sub",
                                        "Delays extra reviews instead of overwhelming you."
                                    }
                                }
                            }
                            div { class: "settings-row__field settings-row__field--toggle",
                                button {
                                    class: "settings-toggle",
                                    r#type: "button",
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

                section { class: "settings-section",
                    h3 { class: "settings-section-title", "Writing Tools" }
                    div { class: "settings-card",
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        path { d: "M4 12a8 8 0 1 1 16 0" }
                                        path { d: "M4 12a8 8 0 0 0 6 7.7" }
                                        circle { cx: "12", cy: "12", r: "2.8" }
                                    }
                                }
                                span { "API key" }
                            }
                            div { class: "settings-row__field settings-row__field--wide",
                                input {
                                    class: "editor-input settings-input",
                                    r#type: "password",
                                    value: "{form_value.ai_api_key}",
                                    placeholder: "sk-…",
                                    oninput: move |evt| {
                                        let mut next = form();
                                        next.ai_api_key = evt.value();
                                        form.set(next);
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        rect {
                                            x: "4",
                                            y: "4",
                                            width: "16",
                                            height: "16",
                                            rx: "3",
                                        }
                                        path { d: "M8 9h8" }
                                        path { d: "M8 13h6" }
                                    }
                                }
                                span { class: "settings-row__label-text",
                                    span { "Preferred model" }
                                    span {
                                        class: "settings-help",
                                        title: "Used by default for writing tools.",
                                        "?"
                                    }
                                }
                            }
                            div { class: "settings-row__field",
                                div { class: "settings-combo",
                                    input {
                                        class: "editor-input settings-input settings-combo__input",
                                        r#type: "text",
                                        value: "{form_value.ai_model}",
                                        placeholder: "gpt-4.1-mini",
                                        oninput: move |evt| {
                                            let mut next = form();
                                            next.ai_model = evt.value();
                                            form.set(next);
                                            save_state.set(SaveState::Idle);
                                        },
                                        onfocus: move |_| {
                                            show_model_menu.set(true);
                                        },
                                    }
                                    button {
                                        class: "settings-combo__toggle",
                                        r#type: "button",
                                        aria_expanded: "{show_model_menu()}",
                                        onclick: move |_| {
                                            show_model_menu.set(!show_model_menu());
                                        },
                                        span { class: "settings-combo__caret" }
                                    }
                                    if show_model_menu() {
                                        div { class: "settings-combo__menu",
                                            for preset in MODEL_PRESETS {
                                                {
                                                    let value = preset.value;
                                                    let note = preset.note;
                                                    rsx! {
                                                        button {
                                                            class: "settings-combo__item",
                                                            r#type: "button",
                                                            title: "{note}",
                                                            onclick: move |_| {
                                                                let mut next = form();
                                                                next.ai_model = value.to_string();
                                                                form.set(next);
                                                                save_state.set(SaveState::Idle);
                                                                show_model_menu.set(false);
                                                            },
                                                            span { class: "settings-combo__item-label",
                                                                "{value}"
                                                            }
                                                            span { class: "settings-combo__item-note",
                                                                "{note}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        path { d: "M4 7h16" }
                                        path { d: "M4 12h10" }
                                        path { d: "M4 17h6" }
                                    }
                                }
                                span { class: "settings-row__label-text",
                                    span { "Daily request cap" }
                                    span {
                                        class: "settings-help",
                                        title: "Maximum writing tool requests per day.",
                                        "?"
                                    }
                                }
                            }
                            div { class: "settings-row__field",
                                input {
                                    class: "editor-input settings-input settings-input--short",
                                    r#type: "text",
                                    value: "{form_value.ai_daily_request_cap}",
                                    placeholder: "100",
                                    oninput: move |evt| {
                                        let mut next = form();
                                        next.ai_daily_request_cap = evt.value();
                                        form.set(next);
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        circle { cx: "12", cy: "12", r: "9" }
                                        path { d: "M12 7v5l3 2" }
                                    }
                                }
                                span { class: "settings-row__label-text",
                                    span { "Cooldown (sec)" }
                                    span {
                                        class: "settings-help",
                                        title: "Minimum seconds between writing tool requests.",
                                        "?"
                                    }
                                }
                            }
                            div { class: "settings-row__field",
                                input {
                                    class: "editor-input settings-input settings-input--short",
                                    r#type: "text",
                                    value: "{form_value.ai_cooldown_secs}",
                                    placeholder: "5",
                                    oninput: move |evt| {
                                        let mut next = form();
                                        next.ai_cooldown_secs = evt.value();
                                        form.set(next);
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        path { d: "M4 7h16" }
                                        path { d: "M4 12h12" }
                                        path { d: "M4 17h9" }
                                    }
                                }
                                span { "Initial prompt" }
                            }
                            div { class: "settings-row__field settings-row__field--wide",
                                textarea {
                                    class: "editor-input settings-input settings-textarea",
                                    value: "{form_value.ai_system_prompt}",
                                    placeholder: "Optional system prompt for all writing tools.",
                                    oninput: move |evt| {
                                        let mut next = form();
                                        next.ai_system_prompt = evt.value();
                                        form.set(next);
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }
                        }
                    }
                }

                section { class: "settings-section settings-section--subtle",
                    h3 { class: "settings-section-title", "FSRS Core Settings" }
                    div { class: "settings-card",
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        circle { cx: "12", cy: "12", r: "8" }
                                        path { d: "M12 6v6l4 2" }
                                    }
                                }
                                span { "Target retention" }
                            }
                            div { class: "settings-row__field",
                                input {
                                    class: "editor-input settings-input",
                                    r#type: "number",
                                    min: "0.7",
                                    max: "0.99",
                                    step: "0.01",
                                    inputmode: "decimal",
                                    value: "{form_value.target_retention}",
                                    oninput: move |evt| {
                                        let mut next = form();
                                        next.target_retention = evt.value();
                                        form.set(next);
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }
                        }
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        path { d: "M4 19h16" }
                                        path { d: "M7 15l3-3 4 4 3-6" }
                                    }
                                }
                                span { "Enable analytics" }
                            }
                            div { class: "settings-row__field settings-row__field--toggle",
                                button {
                                    class: "settings-toggle",
                                    r#type: "button",
                                    aria_checked: "{form_value.analytics_enabled}",
                                    onclick: move |_| {
                                        let mut next = form();
                                        next.analytics_enabled = !next.analytics_enabled;
                                        form.set(next);
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }
                        }
                    }
                }

                section { class: "settings-section settings-section--subtle",
                    h3 { class: "settings-section-title", "Advanced" }
                    div { class: "settings-card",
                        div { class: "settings-row",
                            div { class: "settings-row__label",
                                span { class: "settings-row__icon",
                                    svg {
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "1.6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        rect {
                                            x: "3",
                                            y: "6",
                                            width: "18",
                                            height: "12",
                                            rx: "2",
                                        }
                                        path { d: "M3 7l9 6 9-6" }
                                    }
                                }
                                span { "Signed in" }
                            }
                            div { class: "settings-row__field settings-row__field--wide",
                                div { class: "settings-inline-input",
                                    input {
                                        class: "editor-input settings-input",
                                        r#type: "text",
                                        value: "{form_value.email}",
                                        readonly: true,
                                    }
                                    button {
                                        class: "button button-secondary settings-signout",
                                        r#type: "button",
                                        "Sign out"
                                    }
                                }
                            }
                        }
                    }
                }

                footer { class: "settings-footer",
                    button {
                        class: "button button-secondary",
                        r#type: "button",
                        onclick: move |_| {
                            form.set(GeneralSettingsForm::default());
                            save_state.set(SaveState::Idle);
                        },
                        "Restore defaults"
                    }
                    div { class: "settings-footer-actions",
                        button {
                            class: "button button-secondary",
                            r#type: "button",
                            disabled: !is_dirty || save_state() == SaveState::Saving,
                            onclick: move |_| {
                                form.set(initial());
                                save_state.set(SaveState::Idle);
                            },
                            "Cancel"
                        }
                        button {
                            class: "button button-primary",
                            r#type: "button",
                            disabled: !is_dirty || save_state() == SaveState::Saving,
                            onclick: move |_| {
                                let snapshot = form();
                                let mut initial = initial;
                                let mut form = form;
                                let mut save_state = save_state;
                                let app_settings = app_settings.clone();
                                spawn(async move {
                                    save_state.set(SaveState::Saving);
                                    let Ok(ai_daily_request_cap) =
                                        parse_optional_u32(&snapshot.ai_daily_request_cap)
                                    else {
                                        save_state.set(SaveState::Error(ViewError::Unknown));
                                        return;
                                    };
                                    let Ok(ai_cooldown_secs) =
                                        parse_optional_u32(&snapshot.ai_cooldown_secs)
                                    else {
                                        save_state.set(SaveState::Error(ViewError::Unknown));
                                        return;
                                    };
                                    let draft = AppSettingsDraft {
                                        api_key: to_optional(&snapshot.ai_api_key),
                                        api_model: to_optional(&snapshot.ai_model),
                                        api_fallback_model: to_optional(
                                            &snapshot.ai_fallback_model,
                                        ),
                                        ai_system_prompt: to_optional(
                                            &snapshot.ai_system_prompt,
                                        ),
                                        ai_daily_request_cap,
                                        ai_cooldown_secs,
                                    };
                                    match app_settings.save(draft).await {
                                        Ok(settings) => {
                                            let mut next = snapshot;
                                            apply_app_settings(&mut next, &settings);
                                            form.set(next.clone());
                                            initial.set(next);
                                            save_state.set(SaveState::Saved);
                                        }
                                        Err(_) => {
                                            save_state.set(SaveState::Error(ViewError::Unknown));
                                        }
                                    }
                                });
                            },
                            "Save"
                        }
                    }
                }
            }
        }
    }
}

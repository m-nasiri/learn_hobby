use dioxus::prelude::*;

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
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveState {
    Idle,
    Saved,
}

#[component]
pub fn GeneralSettingsView() -> Element {
    let mut form = use_signal(GeneralSettingsForm::default);
    let mut initial = use_signal(GeneralSettingsForm::default);
    let mut save_state = use_signal(|| SaveState::Idle);

    let form_value = form();
    let initial_value = initial();
    let is_dirty = form_value != initial_value;

    let status_label = match (is_dirty, save_state()) {
        (true, _) => Some("Unsaved changes"),
        (false, SaveState::Saved) => Some("Saved"),
        _ => None,
    };

    rsx! {
        div { class: "page settings-page",
            section { class: "settings-content",
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
                                            class: if form_value.theme == choice {
                                                "settings-segment__button settings-segment__button--active"
                                            } else {
                                                "settings-segment__button"
                                            },
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
                                    span { class: "settings-row__sub", "Delays extra reviews instead of overwhelming you." }
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
                                    }
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
                                    }
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
                                        rect { x: "3", y: "6", width: "18", height: "12", rx: "2" }
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
                            disabled: !is_dirty,
                            onclick: move |_| {
                                form.set(initial());
                                save_state.set(SaveState::Idle);
                            },
                            "Cancel"
                        }
                        button {
                            class: "button button-primary",
                            r#type: "button",
                            disabled: !is_dirty,
                            onclick: move |_| {
                                let snapshot = form();
                                initial.set(snapshot);
                                save_state.set(SaveState::Saved);
                            },
                            "Save"
                        }
                    }
                }
            }
        }
    }
}

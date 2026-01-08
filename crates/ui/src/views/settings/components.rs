use dioxus::prelude::*;

use super::state::SettingsSection;

#[component]
pub(super) fn SettingsNavItem(
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
pub(super) fn SettingsNavIcon(section: SettingsSection) -> Element {
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
pub(super) fn SettingsAccordionSection(
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

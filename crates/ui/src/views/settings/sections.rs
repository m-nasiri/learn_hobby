use dioxus::prelude::*;

use super::components::SettingsAccordionSection;
use super::state::{DeckSettingsErrors, DeckSettingsForm, SaveState, SettingsSection};

const EASY_DAY_OPTIONS: [(&str, u8, &str); 7] = [
    ("Mon", 1_u8 << 0, "Monday"),
    ("Tue", 1_u8 << 1, "Tuesday"),
    ("Wed", 1_u8 << 2, "Wednesday"),
    ("Thu", 1_u8 << 3, "Thursday"),
    ("Fri", 1_u8 << 4, "Friday"),
    ("Sat", 1_u8 << 5, "Saturday"),
    ("Sun", 1_u8 << 6, "Sunday"),
];

pub(super) fn daily_limits_section(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
    expanded_section: Signal<Option<SettingsSection>>,
) -> Element {
    rsx! {
        SettingsAccordionSection {
            label: "Daily Limits",
            section: SettingsSection::DailyLimits,
            expanded: expanded_section() == Some(SettingsSection::DailyLimits),
            on_toggle: expanded_section,
            help_title: None,
            {daily_limits_card(form, errors, save_state)}
        }
    }
}

fn daily_limits_card(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
) -> Element {
    rsx! {
        div { class: "settings-card",
            {daily_limits_new_cards_row(form, errors, save_state)}
            {daily_limits_review_limit_row(form, errors, save_state)}
            {daily_limits_protect_row(form, save_state)}
        }
    }
}

fn daily_limits_new_cards_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
    }
}

fn daily_limits_review_limit_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
    }
}

fn daily_limits_protect_row(
    mut form: Signal<DeckSettingsForm>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();

    rsx! {
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

pub(super) fn lapses_section(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
    expanded_section: Signal<Option<SettingsSection>>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
                            title: "Small delay after pressing \"Again\" to avoid immediate pressure.",
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
    }
}

pub(super) fn fsrs_section(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
    expanded_section: Signal<Option<SettingsSection>>,
) -> Element {
    rsx! {
        SettingsAccordionSection {
            label: "FSRS",
            section: SettingsSection::Fsrs,
            expanded: expanded_section() == Some(SettingsSection::Fsrs),
            on_toggle: expanded_section,
            help_title: Some("Controls memory retention behavior."),
            {fsrs_card(form, errors, save_state)}
        }
    }
}

fn fsrs_card(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
) -> Element {
    rsx! {
        div { class: "settings-card",
            {fsrs_retention_row(form, errors, save_state)}
            {fsrs_optimize_toggle_row(form, save_state)}
            {fsrs_optimize_after_row(form, errors, save_state)}
        }
    }
}

fn fsrs_retention_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
    }
}

fn fsrs_optimize_toggle_row(
    mut form: Signal<DeckSettingsForm>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();

    rsx! {
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
                        next.fsrs_optimize_enabled = !next.fsrs_optimize_enabled;
                        form.set(next);
                        save_state.set(SaveState::Idle);
                    },
                }
            }
        }
    }
}

fn fsrs_optimize_after_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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

pub(super) fn audio_section(
    autoplay_audio: Signal<bool>,
    replay_audio_after_answer: Signal<bool>,
    audio_delay_ms: Signal<String>,
    expanded_section: Signal<Option<SettingsSection>>,
) -> Element {
    rsx! {
        SettingsAccordionSection {
            label: "Audio",
            section: SettingsSection::Audio,
            expanded: expanded_section() == Some(SettingsSection::Audio),
            on_toggle: expanded_section,
            help_title: Some("Language learning audio support."),
            {audio_card(autoplay_audio, replay_audio_after_answer, audio_delay_ms)}
        }
    }
}

fn audio_card(
    autoplay_audio: Signal<bool>,
    replay_audio_after_answer: Signal<bool>,
    audio_delay_ms: Signal<String>,
) -> Element {
    rsx! {
        div { class: "settings-card",
            {audio_autoplay_row(autoplay_audio)}
            {audio_replay_row(replay_audio_after_answer)}
            {audio_delay_row(audio_delay_ms)}
        }
    }
}

fn audio_autoplay_row(mut autoplay_audio: Signal<bool>) -> Element {
    rsx! {
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
    }
}

fn audio_replay_row(mut replay_audio_after_answer: Signal<bool>) -> Element {
    rsx! {
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
                        replay_audio_after_answer.set(!replay_audio_after_answer());
                    },
                }
            }
        }
    }
}

fn audio_delay_row(mut audio_delay_ms: Signal<String>) -> Element {
    rsx! {
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

pub(super) fn timers_section(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
    expanded_section: Signal<Option<SettingsSection>>,
) -> Element {
    rsx! {
        SettingsAccordionSection {
            label: "Timers",
            section: SettingsSection::Timers,
            expanded: expanded_section() == Some(SettingsSection::Timers),
            on_toggle: expanded_section,
            help_title: Some("Minimize stress during reviews."),
            {timers_card(form, errors, save_state)}
        }
    }
}

fn timers_card(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
) -> Element {
    rsx! {
        div { class: "settings-card",
            {timer_show_row(form, save_state)}
            {timer_soft_reminder_row(form, save_state)}
            {timer_soft_reminder_secs_row(form, errors, save_state)}
            {timer_auto_advance_row(form, save_state)}
            {timer_auto_reveal_row(form, errors, save_state)}
        }
    }
}

fn timer_show_row(
    mut form: Signal<DeckSettingsForm>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();

    rsx! {
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
    }
}

fn timer_soft_reminder_row(
    mut form: Signal<DeckSettingsForm>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();

    rsx! {
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
                        next.soft_time_reminder = !next.soft_time_reminder;
                        form.set(next);
                        save_state.set(SaveState::Idle);
                    },
                }
            }
        }
    }
}

fn timer_soft_reminder_secs_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
    }
}

fn timer_auto_advance_row(
    mut form: Signal<DeckSettingsForm>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();

    rsx! {
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
                        next.auto_advance_cards = !next.auto_advance_cards;
                        form.set(next);
                        save_state.set(SaveState::Idle);
                    },
                }
            }
        }
    }
}

fn timer_auto_reveal_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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

pub(super) fn easy_days_section(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
    expanded_section: Signal<Option<SettingsSection>>,
) -> Element {
    rsx! {
        SettingsAccordionSection {
            label: "Easy Days",
            section: SettingsSection::EasyDays,
            expanded: expanded_section() == Some(SettingsSection::EasyDays),
            on_toggle: expanded_section,
            help_title: Some("Support low-energy or busy days."),
            {easy_days_card(form, errors, save_state)}
        }
    }
}

fn easy_days_card(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
) -> Element {
    rsx! {
        div { class: "settings-card",
            {easy_days_toggle_row(form, save_state)}
            {easy_days_factor_row(form, errors, save_state)}
            {easy_days_picker_row(form, errors, save_state)}
        }
    }
}

fn easy_days_toggle_row(
    mut form: Signal<DeckSettingsForm>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();

    rsx! {
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
    }
}

fn easy_days_factor_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
    }
}

fn easy_days_picker_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
                    for (label, bit, title) in EASY_DAY_OPTIONS {
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

pub(super) fn advanced_section(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
    reset_state: Signal<super::state::ResetState>,
    show_reset_modal: Signal<bool>,
    expanded_section: Signal<Option<SettingsSection>>,
) -> Element {
    rsx! {
        SettingsAccordionSection {
            label: "Advanced",
            section: SettingsSection::Advanced,
            expanded: expanded_section() == Some(SettingsSection::Advanced),
            on_toggle: expanded_section,
            help_title: Some("Power-user settings."),
            {advanced_card(form, errors, save_state)}
            {advanced_reset_button(reset_state, show_reset_modal)}
        }
    }
}

fn advanced_card(
    form: Signal<DeckSettingsForm>,
    errors: Signal<DeckSettingsErrors>,
    save_state: Signal<SaveState>,
) -> Element {
    rsx! {
        div { class: "settings-card",
            {advanced_max_interval_row(form, errors, save_state)}
            {advanced_min_interval_row(form, errors, save_state)}
            {advanced_fsrs_params_row(form)}
        }
    }
}

fn advanced_max_interval_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
    }
}

fn advanced_min_interval_row(
    mut form: Signal<DeckSettingsForm>,
    mut errors: Signal<DeckSettingsErrors>,
    mut save_state: Signal<SaveState>,
) -> Element {
    let form_value = form();
    let errors_value = errors();

    rsx! {
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
    }
}

fn advanced_fsrs_params_row(form: Signal<DeckSettingsForm>) -> Element {
    let form_value = form();

    rsx! {
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
}

fn advanced_reset_button(
    mut reset_state: Signal<super::state::ResetState>,
    mut show_reset_modal: Signal<bool>,
) -> Element {
    rsx! {
        button {
            class: "btn settings-danger",
            r#type: "button",
            onclick: move |_| {
                reset_state.set(super::state::ResetState::Idle);
                show_reset_modal.set(true);
            },
            title: "Resets all scheduling data for this deck.",
            "Reset FSRS data"
        }
    }
}

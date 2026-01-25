use dioxus::document::eval;
use dioxus::prelude::*;
use dioxus_router::use_navigator;
use keyboard_types::{Code, Key, Modifiers};

use learn_core::model::{DeckId, ReviewGrade, TagName};

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::{
    SessionIntent, SessionOutcome, SessionPhase, SessionStartMode, SessionVm, sanitize_html,
    start_session,
};
use super::scripts::session_timer_script;

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastAction {
    StartSession,
    Answer(ReviewGrade),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionDestination {
    Summary(i64),
    Practice,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TimerSettings {
    show_timer: bool,
    soft_time_reminder: bool,
    auto_advance_cards: bool,
    soft_time_reminder_secs: u32,
    auto_reveal_secs: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PracticeCounts {
    total: u32,
    due: u32,
    new_count: u32,
    mistakes: u32,
}

impl PracticeCounts {
    const fn has_pending(self) -> bool {
        self.due > 0 || self.new_count > 0
    }

    const fn has_any(self) -> bool {
        self.total > 0
    }

    const fn has_mistakes(self) -> bool {
        self.mistakes > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompletionFlags {
    can_practice_again: bool,
    can_practice_all: bool,
    can_practice_mistakes: bool,
}

fn focus_target_for_phase(
    completed: bool,
    completion: CompletionFlags,
    phase: Option<SessionPhase>,
) -> &'static str {
    if completed {
        return if completion.can_practice_again {
            "session-complete-primary"
        } else if completion.can_practice_all {
            "session-complete-all"
        } else if completion.can_practice_mistakes {
            "session-complete-mistakes"
        } else {
            "session-complete-secondary"
        };
    }
    match phase {
        Some(SessionPhase::Prompt) => "session-reveal",
        Some(SessionPhase::Answer) => "session-grade-again",
        None => "session-quit",
    }
}

fn focus_cycle_ids_for_phase(
    completed: bool,
    completion: CompletionFlags,
    phase: Option<SessionPhase>,
) -> &'static [&'static str] {
    if completed {
        return if completion.can_practice_again {
            &[
                "session-quit",
                "session-complete-primary",
                "session-complete-all",
                "session-complete-mistakes",
                "session-complete-secondary",
            ]
        } else if completion.can_practice_all {
            &[
                "session-quit",
                "session-complete-all",
                "session-complete-mistakes",
                "session-complete-secondary",
            ]
        } else if completion.can_practice_mistakes {
            &["session-quit", "session-complete-mistakes", "session-complete-secondary"]
        } else {
            &["session-quit", "session-complete-secondary"]
        };
    }
    match phase {
        Some(SessionPhase::Prompt) => &["session-quit", "session-reveal"],
        Some(SessionPhase::Answer) => &[
            "session-quit",
            "session-grade-again",
            "session-grade-hard",
            "session-grade-good",
            "session-grade-easy",
        ],
        None => &["session-quit"],
    }
}

fn format_timer(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    format!("Time: {minutes}:{remainder:02}")
}

#[component]
pub fn SessionView(deck_id: u64, tag: Option<String>, mode: SessionStartMode) -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let deck_id = DeckId::new(deck_id);
    let session_loop = ctx.session_loop();
    let card_service = ctx.card_service();
    let deck_service = ctx.deck_service();
    let parsed_tag = tag.as_deref().map(|value| TagName::new(value.to_string()));
    let (tag_name, invalid_tag) = match parsed_tag {
        Some(Ok(tag)) => (Some(tag), false),
        Some(Err(_)) => (None, true),
        None => (None, false),
    };

    let error = use_signal(|| None::<ViewError>);
    let vm = use_signal(|| None::<SessionVm>);
    let last_action = use_signal(|| None::<LastAction>);
    let mut did_focus = use_signal(|| false);
    let mut last_focus_phase = use_signal(|| None::<SessionPhase>);
    let mut last_focus_completed = use_signal(|| false);
    let mut last_focus_can_practice = use_signal(|| true);
    let mut completion = use_signal(|| None::<CompletionDestination>);
    let counts_resource = {
        let card_service = card_service.clone();
        let tag_name = tag_name.clone();
        use_resource(move || {
            let card_service = card_service.clone();
            let tag_name = tag_name.clone();
            async move {
                if let Some(tag) = tag_name.as_ref() {
                    let stats = card_service
                        .list_tag_practice_stats(deck_id)
                        .await
                        .map_err(|_| ViewError::Unknown)?;
                    let counts = stats
                        .iter()
                        .find(|item| item.name == *tag)
                        .map_or_else(PracticeCounts::default, |item| PracticeCounts {
                            total: item.total,
                            due: item.due,
                            new_count: item.new,
                            mistakes: 0,
                        });
                    let mistakes = card_service
                        .mistakes_count(deck_id)
                        .await
                        .map_err(|_| ViewError::Unknown)?;
                    Ok::<_, ViewError>(PracticeCounts {
                        mistakes,
                        ..counts
                    })
                } else {
                    let mistakes = card_service
                        .mistakes_count(deck_id)
                        .await
                        .map_err(|_| ViewError::Unknown)?;
                    let stats = card_service
                        .deck_practice_stats(deck_id)
                        .await
                        .map_err(|_| ViewError::Unknown)?;
                    Ok::<_, ViewError>(PracticeCounts {
                        total: stats.total,
                        due: stats.due,
                        new_count: stats.new,
                        mistakes,
                    })
                }
            }
        })
    };
    let deck_info_resource = {
        let deck_service = deck_service.clone();
        use_resource(move || {
            let deck_service = deck_service.clone();
            async move {
                let deck = deck_service
                    .get_deck(deck_id)
                    .await
                    .map_err(|_| ViewError::Unknown)?;
                Ok::<_, ViewError>(deck.map(|deck| {
                    let settings = deck.settings();
                    (
                        deck.name().to_string(),
                        TimerSettings {
                            show_timer: settings.show_timer(),
                            soft_time_reminder: settings.soft_time_reminder(),
                            auto_advance_cards: settings.auto_advance_cards(),
                            soft_time_reminder_secs: settings.soft_time_reminder_secs(),
                            auto_reveal_secs: settings.auto_reveal_secs(),
                        },
                    )
                }))
            }
        })
    };

    let session_loop_for_resource = session_loop.clone();
    let resource = use_resource(move || {
        let session_loop = session_loop_for_resource.clone();
        let tag_name = tag_name.clone();
        let mut error = error;
        let mut vm = vm;
        let mut last_action = last_action;

        async move {
            last_action.set(Some(LastAction::StartSession));
            completion.set(None);
            if invalid_tag {
                return Err(ViewError::Unknown);
            }
            let started = start_session(&session_loop, deck_id, tag_name, mode).await?;
            vm.set(Some(started));
            error.set(None);
            Ok::<_, ViewError>(())
        }
    });

    let state = view_state_from_resource(&resource);
    let practice_counts = counts_resource
        .value()
        .read()
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .copied();
    let completion_flags = CompletionFlags {
        can_practice_again: practice_counts.is_none_or(PracticeCounts::has_pending),
        can_practice_all: practice_counts.is_some_and(PracticeCounts::has_any),
        can_practice_mistakes: practice_counts.is_none_or(PracticeCounts::has_mistakes),
    };
    let has_any_cards = completion_flags.can_practice_all;

    use_effect(move || {
        let phase = vm.read().as_ref().map(SessionVm::phase);
        let completed = completion.read().is_some();
        if did_focus()
            && last_focus_phase() == phase
            && last_focus_completed() == completed
            && last_focus_can_practice() == completion_flags.can_practice_again
        {
            return;
        }
        did_focus.set(true);
        last_focus_phase.set(phase);
        last_focus_completed.set(completed);
        last_focus_can_practice.set(completion_flags.can_practice_again);
        let target = focus_target_for_phase(completed, completion_flags, phase);
        let js = format!(
            "document.getElementById({target:?})?.focus();",
        );
        let _ = eval(&js);
    });

    let dispatch_intent = {
        let session_loop = session_loop.clone();
        use_callback(move |intent: SessionIntent| {
            let mut error = error;
            let mut vm = vm;
            let mut last_action = last_action;
            let mut completion = completion;
            let mut counts_resource = counts_resource;

            match intent {
                SessionIntent::Reveal => {
                    if let Some(vm) = vm.write().as_mut() {
                        vm.reveal();
                    }
                }
                SessionIntent::Grade(grade) => {
                    let session_loop = session_loop.clone();
                    spawn(async move {
                        last_action.set(Some(LastAction::Answer(grade)));
                        let mut local_vm = {
                            let mut guard = vm.write();
                            guard.take()
                        };

                        let Some(mut vm_value) = local_vm.take() else {
                            error.set(Some(ViewError::Unknown));
                            return;
                        };

                        let result = vm_value.answer_current(&session_loop, grade).await;

                        // Always put the session back so the UI remains usable even after errors.
                        {
                            let mut guard = vm.write();
                            *guard = Some(vm_value);
                        }

                        match result {
                            Ok(outcome) => {
                                error.set(None);
                                match outcome {
                                    SessionOutcome::Continue => {}
                                    SessionOutcome::Completed { summary_id } => {
                                        let destination = summary_id.map_or(
                                            CompletionDestination::Practice,
                                            CompletionDestination::Summary,
                                        );
                                        completion.set(Some(destination));
                                        counts_resource.restart();
                                    }
                                }
                            }
                            Err(_) => {
                                error.set(Some(ViewError::Unknown));
                            }
                        }
                    });
                }
            }
        })
    };

    #[cfg(test)]
    {
        let mut registered = use_signal(|| false);
        if !registered() {
            registered.set(true);
            if let Some(handles) = try_consume_context::<SessionTestHandles>() {
                handles.register(dispatch_intent, vm);
            }
        }
    }

    let retry_action = use_callback(move |()| {
        match last_action() {
            Some(LastAction::StartSession) | None => {
                let mut resource = resource;
                resource.restart();
            }
            Some(LastAction::Answer(grade)) => {
                dispatch_intent.call(SessionIntent::Grade(grade));
            }
        }
    });
    let on_restart = {
        let mut resource = resource;
        let mut counts_resource = counts_resource;
        let mut completion = completion;
        use_callback(move |()| {
            completion.set(None);
            resource.restart();
            counts_resource.restart();
        })
    };

    let on_key = {
        use_callback(move |evt: KeyboardEvent| {
            if evt.data.key() == Key::Tab {
                evt.prevent_default();
                let shift = evt.data.modifiers().contains(Modifiers::SHIFT);
                let phase = vm.read().as_ref().map(SessionVm::phase);
                let completed = completion.read().is_some();
                let ids = focus_cycle_ids_for_phase(completed, completion_flags, phase);
                let ids_js = ids
                    .iter()
                    .map(|id| format!("{id:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let delta = if shift { -1 } else { 1 };
                let js = format!(
                    r"(function() {{
                        const ids = [{ids_js}];
                        if (!ids.length) return;
                        const active = document.activeElement && document.activeElement.id;
                        let idx = ids.indexOf(active);
                        if (idx === -1) idx = 0;
                        idx = (idx + {delta} + ids.length) % ids.length;
                        const next = ids[idx];
                        const el = document.getElementById(next);
                        if (el) el.focus();
                    }})();",
                );
                let _ = eval(&js);
                return;
            }
            if evt.data.key() == Key::Escape {
                evt.prevent_default();
                navigator.push(Route::Practice {});
                return;
            }

            let has_card = vm.read().as_ref().is_some_and(SessionVm::has_card);
            if !has_card {
                return;
            }

            if evt.data.code() == Code::Space {
                if vm.read().as_ref().map(SessionVm::phase) == Some(SessionPhase::Prompt) {
                    evt.prevent_default();
                    dispatch_intent.call(SessionIntent::Reveal);
                }
                return;
            }

            if vm.read().as_ref().map(SessionVm::phase) != Some(SessionPhase::Answer) {
                return;
            }

            if let Key::Character(value) = evt.data.key() {
                match value.as_str() {
                    "1" => {
                        evt.prevent_default();
                        dispatch_intent.call(SessionIntent::Grade(ReviewGrade::Again));
                    }
                    "2" => {
                        evt.prevent_default();
                        dispatch_intent.call(SessionIntent::Grade(ReviewGrade::Hard));
                    }
                    "3" => {
                        evt.prevent_default();
                        dispatch_intent.call(SessionIntent::Grade(ReviewGrade::Good));
                    }
                    "4" => {
                        evt.prevent_default();
                        dispatch_intent.call(SessionIntent::Grade(ReviewGrade::Easy));
                    }
                    _ => {}
                }
            }
        })
    };

    let vm_guard = vm.read();
    let card_prompt_html = use_memo(move || {
        let vm_guard = vm.read();
        vm_guard
            .as_ref()
            .and_then(SessionVm::prompt_text)
            .map(sanitize_html)
    });
    let card_answer_html = use_memo(move || {
        let vm_guard = vm.read();
        vm_guard
            .as_ref()
            .and_then(SessionVm::answer_text)
            .map(sanitize_html)
    });
    let card_prompt_html_read = card_prompt_html.read();
    let card_answer_html_read = card_answer_html.read();
    let card_prompt_html = card_prompt_html_read.as_deref();
    let card_answer_html = card_answer_html_read.as_deref();
    let phase = vm_guard.as_ref().map(SessionVm::phase);
    let completion_state = *completion.read();
    let (current_index, total_cards) = vm_guard.as_ref().map_or((0, 0), |vm| {
        (vm.current_index(), vm.total_cards())
    });
    let progress_label = format!("{current_index} / {total_cards} Cards");
    let streak_label = vm_guard.as_ref().map_or_else(
        || "Streak: 0 🔥".to_string(),
        |vm| format!("Streak: {} 🔥", vm.streak()),
    );
    let due_label = practice_counts
        .map(|counts| counts.due)
        .map_or_else(|| "Due: --".to_string(), |due| format!("Due: {due}"));
    let empty_session_message = if has_any_cards {
        "All caught up. No cards due right now."
    } else {
        "No cards available yet. Add some cards first."
    };
    let empty_session_cta = if has_any_cards {
        ("Back to Practice", Route::Practice {})
    } else {
        ("Add Cards", Route::Editor {})
    };
    let completion_note = (!completion_flags.can_practice_again)
        .then_some("All caught up. No cards due right now.");
    let deck_info = deck_info_resource
        .value()
        .read()
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .and_then(Clone::clone);
    let (deck_label, timer_settings) = deck_info.map_or(
        (None, TimerSettings::default()),
        |(label, settings)| (Some(label), settings),
    );
    let context_label = match (deck_label.as_deref(), tag.as_deref()) {
        (Some(deck), Some(tag)) => format!("{deck} · Tag: {tag}"),
        (Some(deck), None) => deck.to_string(),
        (None, Some(tag)) => format!("Tag: {tag}"),
        (None, None) => String::new(),
    };
    let timer_enabled = timer_settings.show_timer
        || timer_settings.soft_time_reminder
        || timer_settings.auto_advance_cards;
    let timer_active = timer_enabled
        && matches!(state, ViewState::Ready(()))
        && completion_state.is_none()
        && phase == Some(SessionPhase::Prompt);
    let timer_label = format_timer(0);
    let vm_for_timer = vm;
    let completion_for_timer = completion;
    let state_for_timer = state.clone();
    let timer_settings_for_js = timer_settings;
    use_effect(move || {
        let vm_guard = vm_for_timer.read();
            let phase = vm_guard.as_ref().map(SessionVm::phase);
            let current_index = vm_guard.as_ref().map_or(0, |vm| vm.current_index());
            let completion_state = completion_for_timer.read().is_some();
            let timer_enabled = timer_settings_for_js.show_timer
                || timer_settings_for_js.soft_time_reminder
                || timer_settings_for_js.auto_advance_cards;
            let timer_active = timer_enabled
                && matches!(state_for_timer, ViewState::Ready(()))
                && !completion_state
                && phase == Some(SessionPhase::Prompt);
            let timer_key = format!("{}:{current_index}", completion_state as u8);
            let js = session_timer_script(
                &timer_key,
                timer_active,
                timer_settings_for_js.show_timer,
                timer_settings_for_js.soft_time_reminder,
                timer_settings_for_js.auto_advance_cards,
                timer_settings_for_js.soft_time_reminder_secs,
                timer_settings_for_js.auto_reveal_secs,
            );
            let _ = eval(&js);
    });
    let show_timer = timer_settings.show_timer && timer_active;

    rsx! {
        div { class: "page session-page", id: "session-root", tabindex: "0", onkeydown: on_key,
            div { class: "session-overlay",
                div {
                    class: "session-modal",
                    role: "dialog",
                    aria_modal: "true",
                    aria_labelledby: "session-modal-title",
                    header { class: "session-modal__header",
                        div { class: "session-modal__heading",
                            h2 { class: "session-modal__title", id: "session-modal-title", "Practice Session" }
                            if !context_label.is_empty() {
                                p { class: "session-modal__context", "{context_label}" }
                            }
                        }
                        button {
                            class: "session-modal__quit",
                            id: "session-quit",
                            r#type: "button",
                            onclick: move |_| {
                                let _ = navigator.push(Route::Practice {});
                            },
                            "Quit"
                        }
                    }
                    div { class: "session-modal__body",
                        match state {
                            ViewState::Idle => rsx! {
                                p { "Idle" }
                            },
                            ViewState::Loading => rsx! {
                                p { "Loading..." }
                            },
                            ViewState::Error(err) => rsx! {
                                if err == ViewError::EmptySession {
                                    p { "{empty_session_message}" }
                                    button {
                                        class: "btn btn-secondary",
                                        r#type: "button",
                                        onclick: move |_| {
                                            let _ = navigator.push(empty_session_cta.1.clone());
                                        },
                                        "{empty_session_cta.0}"
                                    }
                                } else {
                                    p { "{err.message()}" }
                                    button {
                                        class: "btn btn-secondary",
                                        r#type: "button",
                                        onclick: move |_| retry_action.call(()),
                                        "Retry"
                                    }
                                }
                            },
                            ViewState::Ready(()) => rsx! {
                                if let Some(err) = *error.read() {
                                    p { "{err.message()}" }
                                    button {
                                        class: "btn btn-secondary",
                                        r#type: "button",
                                        onclick: move |_| retry_action.call(()),
                                        "Retry"
                                    }
                                }
                                if completion_state.is_some() {
                                    div { class: "session-complete",
                                        h3 { class: "session-complete__title", "Session complete" }
                                        p { class: "session-complete__subtitle", "Nice work. You finished this practice." }
                                    }
                                } else if let Some(prompt_html) = card_prompt_html {
                                    div { class: "session-question",
                                        div { class: "session-text", dangerous_inner_html: "{prompt_html}" }
                                    }
                                    match phase {
                                        Some(SessionPhase::Prompt) => rsx! {
                                            button {
                                                class: "session-reveal-btn",
                                                id: "session-reveal",
                                                onclick: move |_| dispatch_intent.call(SessionIntent::Reveal),
                                                "Show Answer"
                                            }
                                        },
                                        Some(SessionPhase::Answer) => rsx! {
                                            if let Some(answer_html) = card_answer_html {
                                                div { class: "session-answer",
                                                    div { class: "session-text", dangerous_inner_html: "{answer_html}" }
                                                }
                                            }
                                            p { class: "session-remember", "How well did you remember?" }
                                            div { class: "session-grades",
                                                GradeButton { label: "Again", grade: ReviewGrade::Again, on_intent: dispatch_intent }
                                                GradeButton { label: "Hard", grade: ReviewGrade::Hard, on_intent: dispatch_intent }
                                                GradeButton { label: "Good", grade: ReviewGrade::Good, on_intent: dispatch_intent }
                                                GradeButton { label: "Easy", grade: ReviewGrade::Easy, on_intent: dispatch_intent }
                                            }
                                        },
                                        None => rsx! {},
                                    }
                                } else {
                                    p { "No cards available." }
                                }
                                if timer_settings.soft_time_reminder {
                                    p {
                                        class: "session-soft-reminder",
                                        id: "session-soft-reminder",
                                        hidden: "true",
                                        "Take a breath. You're doing fine."
                                    }
                                }
                            },
                        }
                    }
                    if let Some(destination) = completion_state {
                        footer { class: "session-modal__footer session-modal__footer--complete",
                            CompletionActions {
                                destination,
                                deck_id: deck_id.value(),
                                on_restart,
                                can_practice_again: completion_flags.can_practice_again,
                                can_practice_all: completion_flags.can_practice_all,
                                can_practice_mistakes: completion_flags.can_practice_mistakes,
                                completion_note,
                            }
                        }
                    } else {
                        footer { class: "session-modal__footer",
                            span { class: "session-footer__item", "{progress_label}" }
                            span { class: "session-footer__item", "{streak_label}" }
                            span { class: "session-footer__item", "{due_label}" }
                            if show_timer {
                                span {
                                    class: "session-footer__item session-footer__timer",
                                    id: "session-timer-label",
                                    hidden: "true",
                                    "{timer_label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn GradeButton(
    label: &'static str,
    grade: ReviewGrade,
    on_intent: EventHandler<SessionIntent>,
) -> Element {
    let variant = match grade {
        ReviewGrade::Again => ("session-grade session-grade--again", "session-grade-again"),
        ReviewGrade::Hard => ("session-grade session-grade--hard", "session-grade-hard"),
        ReviewGrade::Good => ("session-grade session-grade--good", "session-grade-good"),
        ReviewGrade::Easy => ("session-grade session-grade--easy", "session-grade-easy"),
    };
    rsx! {
        button {
            class: "{variant.0}",
            id: "{variant.1}",
            onclick: move |_| on_intent.call(SessionIntent::Grade(grade)),
            "{label}"
        }
    }
}

#[component]
fn CompletionActions(
    destination: CompletionDestination,
    deck_id: u64,
    on_restart: EventHandler<()>,
    can_practice_again: bool,
    can_practice_all: bool,
    can_practice_mistakes: bool,
    completion_note: Option<&'static str>,
) -> Element {
    let navigator = use_navigator();
    let (label, route) = match destination {
        CompletionDestination::Summary(summary_id) => ("View Summary", Route::Summary { summary_id }),
        CompletionDestination::Practice => ("Back to Practice", Route::Practice {}),
    };

    rsx! {
        div { class: "session-complete__actions",
            button {
                class: "session-complete__cta",
                id: "session-complete-primary",
                r#type: "button",
                disabled: !can_practice_again,
                onclick: move |_| on_restart.call(()),
                "Practice Again"
            }
            button {
                class: "session-complete__cta session-complete__cta--secondary",
                id: "session-complete-all",
                r#type: "button",
                disabled: !can_practice_all,
                onclick: move |_| {
                    let _ = navigator.push(Route::SessionAll { deck_id });
                },
                "Practice All Cards"
            }
            button {
                class: "session-complete__cta session-complete__cta--secondary",
                id: "session-complete-mistakes",
                r#type: "button",
                disabled: !can_practice_mistakes,
                onclick: move |_| {
                    let _ = navigator.push(Route::SessionMistakes { deck_id });
                },
                "Re-practice Mistakes"
            }
            button {
                class: "session-complete__cta session-complete__cta--ghost",
                id: "session-complete-secondary",
                r#type: "button",
                onclick: move |_| {
                    let _ = navigator.push(route.clone());
                },
                "{label}"
            }
            if let Some(note) = completion_note {
                p { class: "session-complete__note", "{note}" }
            }
        }
    }
}

#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct SessionTestHandles {
    dispatch: Rc<RefCell<Option<Callback<SessionIntent>>>>,
    vm: Rc<RefCell<Option<Signal<Option<SessionVm>>>>>,
}

#[cfg(test)]
impl SessionTestHandles {
    pub(crate) fn register(
        &self,
        dispatch: Callback<SessionIntent>,
        vm: Signal<Option<SessionVm>>,
    ) {
        *self.dispatch.borrow_mut() = Some(dispatch);
        *self.vm.borrow_mut() = Some(vm);
    }

    pub(crate) fn dispatch(&self) -> Callback<SessionIntent> {
        (*self.dispatch.borrow()).expect("session dispatch registered")
    }

    pub(crate) fn vm(&self) -> Signal<Option<SessionVm>> {
        (*self.vm.borrow()).expect("session vm registered")
    }
}

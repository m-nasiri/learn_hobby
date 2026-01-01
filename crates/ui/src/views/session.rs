use dioxus::document::eval;
use dioxus::prelude::*;
use dioxus_router::use_navigator;
use keyboard_types::{Code, Key};

use learn_core::model::{DeckId, ReviewGrade, TagName};

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::{SessionIntent, SessionOutcome, SessionPhase, SessionVm, sanitize_html, start_session};

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastAction {
    StartSession,
    Answer(ReviewGrade),
}

#[component]
pub fn SessionView(deck_id: u64, tag: Option<String>) -> Element {
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
    let due_resource = {
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
                    let due = stats
                        .iter()
                        .find(|item| item.name == *tag)
                        .map_or(0, |item| item.due);
                    Ok::<_, ViewError>(due)
                } else {
                    let stats = card_service
                        .deck_practice_stats(deck_id)
                        .await
                        .map_err(|_| ViewError::Unknown)?;
                    Ok::<_, ViewError>(stats.due)
                }
            }
        })
    };
    let deck_label_resource = {
        let deck_service = deck_service.clone();
        use_resource(move || {
            let deck_service = deck_service.clone();
            async move {
                let deck = deck_service
                    .get_deck(deck_id)
                    .await
                    .map_err(|_| ViewError::Unknown)?;
                Ok::<_, ViewError>(deck.map(|deck| deck.name().to_string()))
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
            if invalid_tag {
                return Err(ViewError::Unknown);
            }
            let started = start_session(&session_loop, deck_id, tag_name).await?;
            vm.set(Some(started));
            error.set(None);
            Ok::<_, ViewError>(())
        }
    });

    let state = view_state_from_resource(&resource);

    use_effect(move || {
        if did_focus() {
            return;
        }
        did_focus.set(true);
        let _ = eval("document.getElementById('session-root')?.focus();");
    });

    let dispatch_intent = {
        let session_loop = session_loop.clone();
        use_callback(move |intent: SessionIntent| {
            let navigator = navigator;
            let mut error = error;
            let mut vm = vm;
            let mut last_action = last_action;

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
                                        if let Some(summary_id) = summary_id {
                                            navigator.push(Route::Summary { summary_id });
                                        } else {
                                            navigator.push(Route::History {});
                                        }
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

    let on_key = {
        use_callback(move |evt: KeyboardEvent| {
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
    let card_prompt = vm_guard.as_ref().and_then(SessionVm::prompt_text);
    let card_answer = vm_guard.as_ref().and_then(SessionVm::answer_text);
    let card_prompt_html = card_prompt.map(sanitize_html);
    let card_answer_html = card_answer.map(sanitize_html);
    let phase = vm_guard.as_ref().map(SessionVm::phase);
    let progress_label = vm_guard.as_ref().map_or_else(
        || "0 / 0 Cards".to_string(),
        |vm| format!("{} / {} Cards", vm.current_index(), vm.total_cards()),
    );
    let streak_label = vm_guard.as_ref().map_or_else(
        || "Streak: 0 🔥".to_string(),
        |vm| format!("Streak: {} 🔥", vm.streak()),
    );
    let due_label = due_resource
        .value()
        .read()
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .map_or_else(|| "Due: --".to_string(), |due| format!("Due: {due}"));
    let deck_label = deck_label_resource
        .value()
        .read()
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .and_then(Clone::clone);
    let context_label = match (deck_label.as_deref(), tag.as_deref()) {
        (Some(deck), Some(tag)) => format!("{deck} · Tag: {tag}"),
        (Some(deck), None) => deck.to_string(),
        (None, Some(tag)) => format!("Tag: {tag}"),
        (None, None) => String::new(),
    };

    rsx! {
        div { class: "page session-page", id: "session-root", tabindex: "0", onkeydown: on_key,
            div { class: "session-overlay",
                div { class: "session-modal",
                    header { class: "session-modal__header",
                        div { class: "session-modal__heading",
                            h2 { class: "session-modal__title", "Practice Session" }
                            if !context_label.is_empty() {
                                p { class: "session-modal__context", "{context_label}" }
                            }
                        }
                        button {
                            class: "session-modal__quit",
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
                                p { "{err.message()}" }
                                button {
                                    class: "btn btn-secondary",
                                    r#type: "button",
                                    onclick: move |_| retry_action.call(()),
                                    "Retry"
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
                                if let Some(prompt_html) = card_prompt_html {
                                    div { class: "session-question",
                                        div { class: "session-text", dangerous_inner_html: "{prompt_html}" }
                                    }
                                    match phase {
                                        Some(SessionPhase::Prompt) => rsx! {
                                            button {
                                                class: "session-reveal-btn",
                                                onclick: move |_| dispatch_intent.call(SessionIntent::Reveal),
                                                "Show Answer"
                                            }
                                        },
                                        Some(SessionPhase::Answer) => rsx! {
                                            if let Some(answer_html) = card_answer_html.clone() {
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
                            },
                        }
                    }
                    footer { class: "session-modal__footer",
                        span { class: "session-footer__item", "{progress_label}" }
                        span { class: "session-footer__item", "{streak_label}" }
                        span { class: "session-footer__item", "{due_label}" }
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
        ReviewGrade::Again => "session-grade session-grade--again",
        ReviewGrade::Hard => "session-grade session-grade--hard",
        ReviewGrade::Good => "session-grade session-grade--good",
        ReviewGrade::Easy => "session-grade session-grade--easy",
    };
    rsx! {
        button {
            class: "{variant}",
            onclick: move |_| on_intent.call(SessionIntent::Grade(grade)),
            "{label}"
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
        self.dispatch
            .borrow()
            .clone()
            .expect("session dispatch registered")
    }

    pub(crate) fn vm(&self) -> Signal<Option<SessionVm>> {
        self.vm.borrow().clone().expect("session vm registered")
    }
}

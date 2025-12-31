use dioxus::document::eval;
use dioxus::prelude::*;
use dioxus_router::use_navigator;
use keyboard_types::{Code, Key};

use learn_core::model::{DeckId, ReviewGrade};

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::{SessionIntent, SessionOutcome, SessionPhase, SessionVm, sanitize_html, start_session};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastAction {
    StartSession,
    Answer(ReviewGrade),
}

#[component]
pub fn SessionView(deck_id: u64) -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let deck_id = DeckId::new(deck_id);
    let session_loop = ctx.session_loop();

    let error = use_signal(|| None::<ViewError>);
    let vm = use_signal(|| None::<SessionVm>);
    let last_action = use_signal(|| None::<LastAction>);
    let mut did_focus = use_signal(|| false);

    let session_loop_for_resource = session_loop.clone();
    let resource = use_resource(move || {
        let session_loop = session_loop_for_resource.clone();
        let mut error = error;
        let mut vm = vm;
        let mut last_action = last_action;

        async move {
            last_action.set(Some(LastAction::StartSession));
            let started = start_session(&session_loop, deck_id).await?;
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
                navigator.push(Route::Home {});
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

    rsx! {
        div { class: "page", id: "session-root", tabindex: "0", onkeydown: on_key,
            header { class: "view-header",
                h2 { class: "view-title", "Practice" }
                p { class: "view-subtitle", "Review due cards in a short session." }
            }
            div { class: "view-divider" }
            p { class: "view-hint", "Shortcuts: Space to reveal, 1–4 to grade, Esc to exit." }
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
                        div { class: "session-card",
                            p { class: "session-label", "Prompt" }
                            div { class: "session-text", dangerous_inner_html: "{prompt_html}" }
                            match phase {
                                Some(SessionPhase::Prompt) => rsx! {
                                    button {
                                        class: "btn btn-primary session-reveal",
                                        onclick: move |_| dispatch_intent.call(SessionIntent::Reveal),
                                        "Reveal answer"
                                    }
                                },
                                Some(SessionPhase::Answer) => rsx! {
                                    p { class: "session-label", "Answer" }
                                    if let Some(answer_html) = card_answer_html.clone() {
                                        div { class: "session-text", dangerous_inner_html: "{answer_html}" }
                                    }
                                    div { class: "session-grades",
                                        GradeButton { label: "Again", grade: ReviewGrade::Again, on_intent: dispatch_intent }
                                        GradeButton { label: "Hard", grade: ReviewGrade::Hard, on_intent: dispatch_intent }
                                        GradeButton { label: "Good", grade: ReviewGrade::Good, on_intent: dispatch_intent }
                                        GradeButton { label: "Easy", grade: ReviewGrade::Easy, on_intent: dispatch_intent }
                                    }
                                },
                                None => rsx! {},
                            }
                        }
                    } else {
                        p { "No cards available." }
                    }
                },
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
    rsx! {
        button {
            class: "btn btn-secondary grade-button",
            onclick: move |_| on_intent.call(SessionIntent::Grade(grade)),
            "{label}"
        }
    }
}

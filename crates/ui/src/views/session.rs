use dioxus::prelude::*;
use dioxus_router::use_navigator;

use learn_core::model::ReviewGrade;
use services::SessionService;

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionPhase {
    Prompt,
    Answer,
}

#[component]
pub fn SessionView() -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let deck_id = ctx.current_deck_id();
    let session_loop = ctx.session_loop();

    let phase = use_signal(|| SessionPhase::Prompt);
    let error = use_signal(|| None::<ViewError>);
    let session = use_signal(|| None::<SessionService>);

    let session_loop_for_resource = session_loop.clone();
    let resource = use_resource(move || {
        let session_loop = session_loop_for_resource.clone();
        let mut session = session;
        let mut phase = phase;
        let mut error = error;

        async move {
            let started = match session_loop.start_session(deck_id).await {
                Ok(session) => session,
                Err(services::SessionError::Empty) => return Err(ViewError::EmptySession),
                Err(_) => return Err(ViewError::Unknown),
            };
            session.set(Some(started));
            phase.set(SessionPhase::Prompt);
            error.set(None);
            Ok::<_, ViewError>(())
        }
    });

    let state = view_state_from_resource(&resource);

    let reveal = {
        let mut phase = phase;
        move |_| phase.set(SessionPhase::Answer)
    };

    let on_grade = {
        let session_loop = session_loop.clone();
        use_callback(move |grade: ReviewGrade| {
            let session_loop = session_loop.clone();
            let mut session = session;
            let navigator = navigator;
            let mut phase = phase;
            let mut error = error;

            spawn(async move {
                // Take the session out of the signal so we don't hold a write lock across `.await`.
                let mut local_session = {
                    let mut guard = session.write();
                    guard.take()
                };

                let Some(mut session_value) = local_session.take() else {
                    error.set(Some(ViewError::Unknown));
                    return;
                };

                let result = session_loop.answer_current(&mut session_value, grade).await;

                // Always put the session back so the UI remains usable even after errors.
                {
                    let mut guard = session.write();
                    *guard = Some(session_value);
                }

                match result {
                    Ok(result) => {
                        error.set(None);
                        if result.is_complete {
                            if let Some(summary_id) = result.summary_id {
                                navigator.push(Route::Summary { summary_id });
                            } else {
                                navigator.push(Route::History {});
                            }
                        } else {
                            phase.set(SessionPhase::Prompt);
                        }
                    }
                    Err(_) => {
                        error.set(Some(ViewError::Unknown));
                    }
                }
            });
        })
    };

    let session_guard = session.read();
    let card = session_guard
        .as_ref()
        .and_then(SessionService::current_card);

    rsx! {
        div { class: "page",
            h2 { "Practice" }
            match state {
                ViewState::Idle => rsx! {
                    p { "Idle" }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
                },
                ViewState::Error(err) => rsx! {
                    p { "{err.message()}" }
                },
                ViewState::Ready(()) => rsx! {
                    if let Some(err) = *error.read() {
                        p { "{err.message()}" }
                    }
                    if let Some(card) = card {
                        div { class: "session-card",
                            p { class: "session-label", "Prompt" }
                            p { class: "session-text", "{card.prompt().text()}" }
                            match *phase.read() {
                                SessionPhase::Prompt => rsx! {
                                    button { class: "btn btn-primary session-reveal", onclick: reveal, "Reveal answer" }
                                },
                                SessionPhase::Answer => rsx! {
                                    p { class: "session-label", "Answer" }
                                    p { class: "session-text", "{card.answer().text()}" }
                                    div { class: "session-grades",
                                        GradeButton { label: "Again", grade: ReviewGrade::Again, on_grade }
                                        GradeButton { label: "Hard", grade: ReviewGrade::Hard, on_grade }
                                        GradeButton { label: "Good", grade: ReviewGrade::Good, on_grade }
                                        GradeButton { label: "Easy", grade: ReviewGrade::Easy, on_grade }
                                    }
                                },
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
    on_grade: EventHandler<ReviewGrade>,
) -> Element {
    rsx! {
        button {
            class: "btn btn-secondary grade-button",
            onclick: move |_| on_grade.call(grade),
            "{label}"
        }
    }
}

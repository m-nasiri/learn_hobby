use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionPhase {
    Prompt,
    Answer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionCard {
    prompt: &'static str,
    answer: &'static str,
}

#[component]
pub fn SessionView() -> Element {
    let phase = use_signal(|| SessionPhase::Prompt);
    let card = SessionCard {
        prompt: "What is the German word for “hello”?",
        answer: "Hallo",
    };

    let reveal = {
        let mut phase = phase;
        move |_| phase.set(SessionPhase::Answer)
    };

    let grade = {
        let mut phase = phase;
        move |_grade: SessionGrade| {
            // Placeholder: in the real flow, this will call the session service and advance.
            phase.set(SessionPhase::Prompt)
        }
    };

    rsx! {
        div { class: "page",
            h2 { "Practice" }
            div { class: "session-card",
                p { class: "session-label", "Prompt" }
                p { class: "session-text", "{card.prompt}" }
                match *phase.read() {
                    SessionPhase::Prompt => rsx! {
                        button { class: "session-reveal", onclick: reveal, "Reveal answer" }
                    },
                    SessionPhase::Answer => rsx! {
                        p { class: "session-label", "Answer" }
                        p { class: "session-text", "{card.answer}" }
                        div { class: "session-grades",
                            GradeButton { label: "Again", grade: SessionGrade::Again, on_grade: grade }
                            GradeButton { label: "Hard", grade: SessionGrade::Hard, on_grade: grade }
                            GradeButton { label: "Good", grade: SessionGrade::Good, on_grade: grade }
                            GradeButton { label: "Easy", grade: SessionGrade::Easy, on_grade: grade }
                        }
                    },
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionGrade {
    Again,
    Hard,
    Good,
    Easy,
}

#[component]
fn GradeButton(
    label: &'static str,
    grade: SessionGrade,
    on_grade: EventHandler<SessionGrade>,
) -> Element {
    rsx! {
        button { class: "grade-button", onclick: move |_| on_grade.call(grade), "{label}" }
    }
}

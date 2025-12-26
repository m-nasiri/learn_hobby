use dioxus::prelude::*;

use crate::context::AppContext;

#[component]
pub fn EditorView() -> Element {
    let ctx = use_context::<AppContext>();
    let _deck_id = ctx.current_deck_id();
    // UI-only state for now (service wiring comes next step).
    let mut prompt_text = use_signal(String::new);
    let mut answer_text = use_signal(String::new);

    let can_save = {
        let p = prompt_text.read();
        let a = answer_text.read();
        !p.trim().is_empty() && !a.trim().is_empty()
    };

    rsx! {
        div { class: "page",
            section { class: "editor-shell",
                header { class: "editor-header",
                    h2 { class: "editor-title", "New Card" }
                    p { class: "editor-subtitle", "Deck: Default" }
                }

                div { class: "editor-body",
                    div { class: "editor-group",
                        label { class: "editor-label", r#for: "prompt", "Front" }
                        input {
                            id: "prompt",
                            class: "editor-input editor-input--single",
                            r#type: "text",
                            placeholder: "Enter the prompt for the front of the card...",
                            value: "{prompt_text.read()}",
                            oninput: move |evt| prompt_text.set(evt.value()),
                        }
                    }

                    div { class: "editor-group",
                        label { class: "editor-label", r#for: "answer", "Back" }
                        textarea {
                            id: "answer",
                            class: "editor-input editor-input--multi",
                            rows: 6,
                            placeholder: "Enter the answer for the back of the card...",
                            value: "{answer_text.read()}",
                            oninput: move |evt| answer_text.set(evt.value()),
                        }
                    }

                    button { class: "editor-add-inline", r#type: "button",
                        span { class: "editor-add-plus", "+" }
                        span { "Add Image" }
                    }
                }

                footer { class: "editor-footer",
                    div { class: "editor-actions",
                        button { class: "btn editor-cancel", r#type: "button", "Cancel" }
                        button {
                            class: "btn editor-save",
                            r#type: "button",
                            disabled: !can_save,
                            "Save"
                        }
                        button {
                            class: "btn editor-practice",
                            r#type: "button",
                            disabled: !can_save,
                            "Save & Practice"
                        }
                    }
                }
            }
        }
    }
}

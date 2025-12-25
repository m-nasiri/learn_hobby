use dioxus::prelude::*;

use crate::context::AppContext;

#[component]
pub fn EditorView() -> Element {
    let ctx = use_context::<AppContext>();
    let deck_id = ctx.current_deck_id();
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
            div { class: "editor-root",
                section { class: "editor-window",
                    header { class: "editor-titlebar",
                        h2 { class: "editor-title", "New Card" }
                    }

                    div { class: "editor-content",
                        p { class: "editor-deck", "Deck: {deck_id:?}" }

                        div { class: "editor-section",
                            label { class: "editor-label", "Prompt" }
                            div { class: "editor-field",
                                textarea {
                                    class: "editor-input",
                                    rows: 4,
                                    placeholder: "Enter the prompt for the front of the card...",
                                    value: "{prompt_text.read()}",
                                    oninput: move |evt| prompt_text.set(evt.value()),
                                }
                                button { class: "editor-add", r#type: "button",
                                    span { class: "editor-add-icon", "+" }
                                    span { "Add image" }
                                }
                            }
                        }

                        div { class: "editor-divider" }

                        div { class: "editor-section",
                            label { class: "editor-label", "Answer" }
                            div { class: "editor-field",
                                textarea {
                                    class: "editor-input",
                                    rows: 4,
                                    placeholder: "Enter the answer for the back of the card...",
                                    value: "{answer_text.read()}",
                                    oninput: move |evt| answer_text.set(evt.value()),
                                }
                                button { class: "editor-add", r#type: "button",
                                    span { class: "editor-add-icon", "+" }
                                    span { "Add image" }
                                }
                            }
                        }
                    }

                    footer { class: "editor-footer",
                        div { class: "editor-actions",
                            button { class: "btn editor-cancel", r#type: "button", "Cancel" }
                            div { class: "editor-action",
                                button {
                                    class: "btn editor-save",
                                    r#type: "button",
                                    disabled: !can_save,
                                    "Save"
                                }
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
}

use dioxus::prelude::*;

use super::super::state::DeleteState;

#[component]
pub fn EditorOverlays(
    show_deck_overlay: bool,
    show_delete_modal: bool,
    delete_state: DeleteState,
    show_duplicate_modal: bool,
    show_save_overlay: bool,
    show_unsaved_modal: bool,
    on_deck_overlay_close: Callback<()>,
    on_delete_close: Callback<()>,
    on_delete_confirm: Callback<()>,
    on_duplicate_close: Callback<()>,
    on_duplicate_confirm: Callback<()>,
    on_save_overlay_close: Callback<()>,
    on_unsaved_cancel: Callback<()>,
    on_unsaved_confirm: Callback<()>,
) -> Element {
    rsx! {
        if show_deck_overlay {
            div {
                class: "editor-deck-overlay",
                onclick: move |_| on_deck_overlay_close.call(()),
            }
        }
        if show_delete_modal {
            div {
                class: "editor-modal-overlay",
                onclick: move |_| on_delete_close.call(()),
                div {
                    class: "editor-modal",
                    onclick: move |evt| evt.stop_propagation(),
                    h3 { class: "editor-modal-title", "Delete card?" }
                    p { class: "editor-modal-body",
                        "This will remove the card and its review history."
                    }
                    div { class: "editor-modal-actions",
                        button {
                            class: "btn editor-modal-cancel",
                            r#type: "button",
                            onclick: move |_| on_delete_close.call(()),
                            "Cancel"
                        }
                        button {
                            class: "btn editor-modal-confirm",
                            r#type: "button",
                            disabled: delete_state == DeleteState::Deleting,
                            onclick: move |_| on_delete_confirm.call(()),
                            "Delete"
                        }
                    }
                }
            }
        }
        if show_duplicate_modal {
            div {
                class: "editor-modal-overlay",
                onclick: move |_| on_duplicate_close.call(()),
                div {
                    class: "editor-modal",
                    onclick: move |evt| evt.stop_propagation(),
                    h3 { class: "editor-modal-title", "Duplicate front?" }
                    p { class: "editor-modal-body",
                        "A card with the same front already exists in this deck."
                    }
                    div { class: "editor-modal-actions",
                        button {
                            class: "btn editor-modal-cancel",
                            r#type: "button",
                            onclick: move |_| on_duplicate_close.call(()),
                            "Keep Editing"
                        }
                        button {
                            class: "btn btn-primary",
                            r#type: "button",
                            onclick: move |_| on_duplicate_confirm.call(()),
                            "Save Anyway"
                        }
                    }
                }
            }
        }
        if show_save_overlay {
            div {
                class: "editor-save-overlay",
                onclick: move |_| on_save_overlay_close.call(()),
            }
        }
        if show_unsaved_modal {
            div {
                class: "editor-modal-overlay",
                onclick: move |_| on_unsaved_cancel.call(()),
                div {
                    class: "editor-modal",
                    onclick: move |evt| evt.stop_propagation(),
                    h3 { class: "editor-modal-title", "Discard changes?" }
                    p { class: "editor-modal-body",
                        "You have unsaved edits. Discard them and continue?"
                    }
                    div { class: "editor-modal-actions",
                        button {
                            class: "btn editor-modal-cancel",
                            r#type: "button",
                            onclick: move |_| on_unsaved_cancel.call(()),
                            "Keep Editing"
                        }
                        button {
                            class: "btn editor-modal-confirm",
                            r#type: "button",
                            onclick: move |_| on_unsaved_confirm.call(()),
                            "Discard"
                        }
                    }
                }
            }
        }
    }
}

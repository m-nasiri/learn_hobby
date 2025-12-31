use dioxus::document::eval;
use dioxus::prelude::*;

use crate::context::AppContext;
use crate::vm::MarkdownField;
use crate::views::{ViewState, view_state_from_resource};

use super::actions::{EditorIntent, use_editor_dispatcher};
use super::components::{EditorDetailPane, EditorListPane, EditorOverlays};
use super::scripts::{read_editable_html, set_editable_html};
use super::state::{
    DeleteState, EditorServices, SaveMenuState, SaveState, use_editor_state,
};
use crate::vm::build_editor_vm;

#[component]
pub fn EditorView() -> Element {
    let ctx = use_context::<AppContext>();
    let services = EditorServices {
        deck_service: ctx.deck_service(),
        card_service: ctx.card_service(),
    };
    let state = use_editor_state(ctx.current_deck_id(), &services);
    let dispatcher = use_editor_dispatcher(&state, &services);
    let dispatch = dispatcher.dispatch;

    let decks_state = view_state_from_resource(&state.decks_resource);
    let cards_state = view_state_from_resource(&state.cards_resource);
    let deck_tags_state = view_state_from_resource(&state.deck_tags_resource);

    let vm = build_editor_vm(&state, &decks_state, &cards_state, &deck_tags_state);
    let deck_label = vm.deck_label.clone();

    let save_state = state.save_state;
    let mut delete_state = state.delete_state;
    let duplicate_check_state = state.duplicate_check_state;
    let mut show_deck_menu = state.show_deck_menu;
    let mut is_renaming_deck = state.is_renaming_deck;
    let mut show_delete_modal = state.show_delete_modal;
    let show_duplicate_modal = state.show_duplicate_modal;
    let show_unsaved_modal = state.show_unsaved_modal;
    let save_menu_state = state.save_menu_state;
    let mut show_new_deck = state.show_new_deck;
    let mut new_deck_state = state.new_deck_state;
    let mut new_deck_name = state.new_deck_name;
    let mut rename_deck_name = state.rename_deck_name;
    let mut rename_deck_state = state.rename_deck_state;
    let mut rename_deck_error = state.rename_deck_error;
    let sort_mode = state.sort_mode;
    let tag_input = state.tag_input;
    let last_focus_field = state.last_focus_field;
    let prompt_text = state.prompt_text;
    let answer_text = state.answer_text;

    let mut focus_prompt = state.focus_prompt;
    use_effect(move || {
        if !focus_prompt() {
            return;
        }
        focus_prompt.set(false);
        let _ = eval("document.getElementById('prompt')?.focus();");
    });

    let prompt_render_html_for_effect = state.prompt_render_html;
    use_effect(move || {
        let html = prompt_render_html_for_effect.read().to_string();
        spawn(async move {
            set_editable_html("prompt", &html).await;
        });
    });

    let answer_render_html_for_effect = state.answer_render_html;
    use_effect(move || {
        let html = answer_render_html_for_effect.read().to_string();
        spawn(async move {
            set_editable_html("answer", &html).await;
        });
    });

    let deck_overlay_close = {
        let mut show_deck_menu = show_deck_menu;
        let is_renaming_deck = is_renaming_deck;
        use_callback(move |()| {
            show_deck_menu.set(false);
            if is_renaming_deck() {
                dispatch.call(EditorIntent::CancelRename);
            }
        })
    };

    let search_change = {
        let mut search_query = state.search_query;
        use_callback(move |value: String| {
            search_query.set(value);
        })
    };

    let clear_search = {
        let mut search_query = state.search_query;
        use_callback(move |()| {
            search_query.set(String::new());
        })
    };

    let sort_change = {
        let mut sort_mode = state.sort_mode;
        use_callback(move |mode| {
            sort_mode.set(mode);
        })
    };

    let set_tag_filter = {
        use_callback(move |tag| {
            dispatch.call(EditorIntent::SetTagFilter(tag));
        })
    };

    let on_focus_field = {
        let mut last_focus_field = last_focus_field;
        use_callback(move |field: MarkdownField| {
            last_focus_field.set(field);
        })
    };

    let on_prompt_input = use_callback(move |()| {
        let mut prompt_text = prompt_text;
        let mut save_state = save_state;
        spawn(async move {
            if let Some(updated) = read_editable_html("prompt").await {
                prompt_text.set(updated);
                save_state.set(SaveState::Idle);
            }
        });
    });

    let on_answer_input = use_callback(move |()| {
        let mut answer_text = answer_text;
        let mut save_state = save_state;
        spawn(async move {
            if let Some(updated) = read_editable_html("answer").await {
                answer_text.set(updated);
                save_state.set(SaveState::Idle);
            }
        });
    });

    let on_prompt_paste = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::HandlePaste(MarkdownField::Front));
        })
    };

    let on_answer_paste = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::HandlePaste(MarkdownField::Back));
        })
    };

    let on_tag_input_change = {
        let mut tag_input = tag_input;
        use_callback(move |value: String| {
            tag_input.set(value);
        })
    };

    let on_delete_close = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CloseDeleteModal);
        })
    };

    let on_delete_confirm = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::Delete);
        })
    };

    let on_duplicate_close = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CloseDuplicateModal);
        })
    };

    let on_duplicate_confirm = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::ConfirmDuplicate);
        })
    };

    let on_save_overlay_close = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CloseSaveMenu);
        })
    };

    let on_unsaved_cancel = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CancelDiscard);
        })
    };

    let on_unsaved_confirm = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::ConfirmDiscard);
        })
    };

    let on_request_new_card = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::RequestNewCard);
        })
    };

    let on_request_select_card = {
        use_callback(move |card| {
            dispatch.call(EditorIntent::RequestSelectCard(card));
        })
    };

    let on_request_select_deck = {
        use_callback(move |deck_id| {
            dispatch.call(EditorIntent::RequestSelectDeck(deck_id));
        })
    };

    let on_create_deck = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CreateDeck);
        })
    };

    let on_begin_rename = {
        use_callback(move |label: String| {
            dispatch.call(EditorIntent::BeginRename(label));
        })
    };

    let on_commit_rename = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CommitRename);
        })
    };

    let on_cancel_rename = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CancelRename);
        })
    };

    let on_format = {
        use_callback(move |(field, action)| {
            dispatch.call(EditorIntent::ApplyFormat(field, action));
        })
    };

    let on_block_dir = {
        use_callback(move |(field, direction)| {
            dispatch.call(EditorIntent::ApplyBlockDir(field, direction));
        })
    };

    let on_tag_add = {
        use_callback(move |value: String| {
            dispatch.call(EditorIntent::AddTag(value));
        })
    };

    let on_tag_remove = {
        use_callback(move |value: String| {
            dispatch.call(EditorIntent::RemoveTag(value));
        })
    };

    let on_cancel_new = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CancelNew);
        })
    };

    let on_open_delete = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::OpenDeleteModal);
        })
    };

    let on_save = {
        use_callback(move |request| {
            dispatch.call(EditorIntent::Save(request));
        })
    };

    let on_toggle_save_menu = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::ToggleSaveMenu);
        })
    };

    let on_close_save_menu = {
        use_callback(move |()| {
            dispatch.call(EditorIntent::CloseSaveMenu);
        })
    };

    rsx! {
        div { class: "page page--editor", tabindex: "0", onkeydown: dispatcher.on_key,
            EditorOverlays {
                show_deck_overlay: show_deck_menu() || is_renaming_deck(),
                show_delete_modal: show_delete_modal(),
                delete_state: delete_state(),
                show_duplicate_modal: show_duplicate_modal(),
                show_save_overlay: save_menu_state() == SaveMenuState::Open,
                show_unsaved_modal: show_unsaved_modal(),
                on_deck_overlay_close: deck_overlay_close,
                on_delete_close: on_delete_close,
                on_delete_confirm: on_delete_confirm,
                on_duplicate_close: on_duplicate_close,
                on_duplicate_confirm: on_duplicate_confirm,
                on_save_overlay_close: on_save_overlay_close,
                on_unsaved_cancel: on_unsaved_cancel,
                on_unsaved_confirm: on_unsaved_confirm,
            }

            section { class: "editor-shell",
                header { class: "editor-toolbar",
                    div { class: "editor-toolbar-left editor-deck-menu",
                        match decks_state {
                            ViewState::Idle | ViewState::Loading => rsx! {
                                div { class: "editor-deck-trigger editor-deck-trigger--disabled",
                                    span { "Loading decks..." }
                                }
                            },
                            ViewState::Error(_err) => rsx! {
                                div { class: "editor-deck-trigger editor-deck-trigger--disabled",
                                    span { "Decks unavailable" }
                                }
                            },
                            ViewState::Ready(options) => {
                                let deck_label_for_double = deck_label.clone();
                                let deck_label_for_context = deck_label.clone();
                                rsx! {
                                    div { class: "editor-deck-trigger",
                                        if is_renaming_deck() {
                                            input {
                                                class: "editor-deck-rename-input",
                                                r#type: "text",
                                                value: "{rename_deck_name.read()}",
                                                oninput: move |evt| {
                                                    rename_deck_name.set(evt.value());
                                                    rename_deck_state.set(SaveState::Idle);
                                                    rename_deck_error.set(None);
                                                },
                                                onkeydown: move |evt| match evt.data.key() {
                                                    Key::Enter => {
                                                        evt.prevent_default();
                                                        on_commit_rename.call(());
                                                    }
                                                    Key::Escape => {
                                                        evt.prevent_default();
                                                        on_cancel_rename.call(());
                                                    }
                                                    _ => {}
                                                },
                                                onblur: move |_| {
                                                    if rename_deck_state() != SaveState::Saving {
                                                        on_cancel_rename.call(());
                                                    }
                                                },
                                                autofocus: true,
                                            }
                                        } else {
                                            button {
                                                class: "editor-deck-label",
                                                r#type: "button",
                                                title: "Rename deck",
                                                ondoubleclick: move |_| {
                                                    on_begin_rename
                                                        .call(deck_label_for_double.clone());
                                                },
                                                oncontextmenu: move |evt| {
                                                    evt.prevent_default();
                                                    on_begin_rename
                                                        .call(deck_label_for_context.clone());
                                                },
                                                "{deck_label}"
                                            }
                                        }
                                        button {
                                            class: "editor-deck-caret-button",
                                            r#type: "button",
                                            title: "Select deck",
                                            onclick: move |_| {
                                                show_deck_menu.set(!show_deck_menu());
                                                is_renaming_deck.set(false);
                                                rename_deck_state.set(SaveState::Idle);
                                                rename_deck_error.set(None);
                                            },
                                            span { class: "editor-deck-caret" }
                                        }
                                    }
                                    if let Some(error) = rename_deck_error() {
                                        span {
                                            class: "editor-deck-toast editor-deck-toast--error",
                                            "{error}"
                                        }
                                    } else if rename_deck_state() == SaveState::Saving {
                                        span { class: "editor-deck-toast", "Saving..." }
                                    }
                                    if is_renaming_deck() {
                                        span { class: "editor-deck-hint", "Enter to save · Esc to cancel" }
                                    }
                                    if show_deck_menu() {
                                        div { class: "editor-deck-popover",
                                            for opt in options {
                                                button {
                                                    class: if opt.id == *state.selected_deck.read() {
                                                        "editor-deck-item editor-deck-item--active"
                                                    } else {
                                                        "editor-deck-item"
                                                    },
                                                    r#type: "button",
                                                    onclick: move |_| on_request_select_deck.call(opt.id),
                                                    "{opt.label}"
                                                }
                                            }
                                            button {
                                                class: "editor-deck-item editor-deck-item--new",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    show_new_deck.set(true);
                                                    new_deck_state.set(SaveState::Idle);
                                                    show_deck_menu.set(false);
                                                    is_renaming_deck.set(false);
                                                    rename_deck_state.set(SaveState::Idle);
                                                    rename_deck_error.set(None);
                                                    delete_state.set(DeleteState::Idle);
                                                    show_delete_modal.set(false);
                                                },
                                                "+ New deck..."
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "editor-toolbar-right",
                        button {
                            class: "btn btn-primary editor-toolbar-action",
                            r#type: "button",
                            title: "New card",
                            onclick: move |_| on_request_new_card.call(()),
                            "+ New Card"
                        }
                    }
                }

                if show_new_deck() {
                    div { class: "editor-deck-new",
                        input {
                            class: "editor-deck-input",
                            r#type: "text",
                            placeholder: "New deck name",
                            value: "{new_deck_name.read()}",
                            oninput: move |evt| {
                                new_deck_name.set(evt.value());
                                new_deck_state.set(SaveState::Idle);
                            },
                        }
                        button {
                            class: "btn editor-deck-create",
                            r#type: "button",
                            disabled: new_deck_name.read().trim().is_empty()
                                || new_deck_state() == SaveState::Saving,
                            onclick: move |_| on_create_deck.call(()),
                            "Create"
                        }
                        button {
                            class: "btn editor-deck-cancel",
                            r#type: "button",
                            onclick: move |_| {
                                show_new_deck.set(false);
                                new_deck_name.set(String::new());
                                new_deck_state.set(SaveState::Idle);
                            },
                            "Cancel"
                        }
                        span { class: "editor-deck-status",
                            match new_deck_state() {
                                SaveState::Idle => rsx! {},
                                SaveState::Saving => rsx! { "Creating..." },
                                SaveState::Success => rsx! { "Created." },
                                SaveState::Error(err) => rsx! { "{err.message()}" },
                            }
                        }
                    }
                }

                div { class: "editor-split",
                    EditorListPane {
                        cards_state: cards_state.clone(),
                        selected_card_id: vm.selected_card_id,
                        search_value: vm.search_value.clone(),
                        match_count: vm.match_count,
                        sort_mode: sort_mode(),
                        selected_tag: vm.selected_tag.clone(),
                        deck_tags: vm.deck_tags.clone(),
                        deck_tags_loading: vm.deck_tags_loading,
                        deck_tags_error: vm.deck_tags_error,
                        on_search_change: search_change,
                        on_clear_search: clear_search,
                        on_sort_change: sort_change,
                        on_tag_filter_change: set_tag_filter,
                        on_select_card: on_request_select_card,
                        on_new_card: on_request_new_card,
                        on_list_key: dispatcher.list_on_key,
                    }
                    EditorDetailPane {
                        can_edit: vm.can_edit,
                        is_create_mode: vm.is_create_mode,
                        selected_card_id: vm.selected_card_id,
                        has_unsaved_changes: vm.has_unsaved_changes,
                        can_cancel: vm.can_cancel,
                        can_submit: vm.can_submit,
                        prompt_invalid: vm.prompt_invalid,
                        answer_invalid: vm.answer_invalid,
                        prompt_toolbar_disabled: vm.prompt_toolbar_disabled,
                        answer_toolbar_disabled: vm.answer_toolbar_disabled,
                        tag_input_value: vm.tag_input_value.clone(),
                        tag_suggestions: vm.tag_suggestions.clone(),
                        card_tags: vm.card_tags.clone(),
                        save_state: save_state(),
                        delete_state: delete_state(),
                        duplicate_check_state: duplicate_check_state(),
                        save_menu_state: save_menu_state(),
                        on_focus_field,
                        on_prompt_input,
                        on_answer_input,
                        on_prompt_paste,
                        on_answer_paste,
                        on_format: on_format,
                        on_block_dir: on_block_dir,
                        on_tag_input_change,
                        on_tag_add: on_tag_add,
                        on_tag_remove: on_tag_remove,
                        on_cancel: on_cancel_new,
                        on_open_delete: on_open_delete,
                        on_save: on_save,
                        on_toggle_save_menu: on_toggle_save_menu,
                        on_close_save_menu: on_close_save_menu,
                    }
                }
            }
        }
    }
}

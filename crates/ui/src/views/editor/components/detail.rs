use dioxus::prelude::*;
use learn_core::model::CardId;

use crate::vm::{MarkdownAction, MarkdownField};

use super::EditorFormatToolbar;
use super::super::state::{
    DeleteState, DuplicateCheckState, SaveMenuState, SaveRequest, SaveState,
    WritingToolsCommand, WritingToolsMenuState, WritingToolsResultStatus, WritingToolsTone,
};

#[component]
pub fn EditorDetailPane(
    can_edit: bool,
    is_create_mode: bool,
    selected_card_id: Option<CardId>,
    has_unsaved_changes: bool,
    can_cancel: bool,
    can_submit: bool,
    prompt_invalid: bool,
    answer_invalid: bool,
    prompt_toolbar_disabled: bool,
    answer_toolbar_disabled: bool,
    tag_input_value: String,
    tag_suggestions: Vec<String>,
    card_tags: Vec<String>,
    daily_limit_warning: Option<String>,
    save_state: SaveState,
    delete_state: DeleteState,
    duplicate_check_state: DuplicateCheckState,
    save_menu_state: SaveMenuState,
    writing_tools_menu_state: WritingToolsMenuState,
    writing_tools_prompt: String,
    writing_tools_tone: WritingToolsTone,
    writing_tools_result_status: WritingToolsResultStatus,
    writing_tools_result_target: Option<MarkdownField>,
    writing_tools_result_title: String,
    writing_tools_result_html: String,
    on_focus_field: Callback<MarkdownField>,
    on_prompt_input: Callback<()>,
    on_answer_input: Callback<()>,
    on_prompt_paste: Callback<()>,
    on_answer_paste: Callback<()>,
    on_format: Callback<(MarkdownField, MarkdownAction)>,
    on_block_dir: Callback<(MarkdownField, String)>,
    on_tag_input_change: Callback<String>,
    on_tag_add: Callback<String>,
    on_tag_remove: Callback<String>,
    on_cancel: Callback<()>,
    on_open_delete: Callback<()>,
    on_save: Callback<SaveRequest>,
    on_toggle_save_menu: Callback<()>,
    on_close_save_menu: Callback<()>,
    on_toggle_writing_tools: Callback<MarkdownField>,
    on_update_writing_tools_prompt: Callback<String>,
    on_select_writing_tools_tone: Callback<WritingToolsTone>,
    on_select_writing_tools_command: Callback<(MarkdownField, WritingToolsCommand)>,
    on_replace_writing_tools: Callback<MarkdownField>,
    on_copy_writing_tools: Callback<MarkdownField>,
) -> Element {
    let show_dirty = can_edit && has_unsaved_changes;
    let can_show_delete = !is_create_mode && selected_card_id.is_some();
    let tag_input_for_keydown = tag_input_value.clone();
    let tag_input_for_blur = tag_input_value.clone();
    let card_tags_for_backspace = card_tags.clone();

    rsx! {
        section { class: "editor-detail",
            header { class: "editor-detail-header",
                h3 { class: "editor-detail-title",
                    if is_create_mode {
                        "New Card"
                    } else if selected_card_id.is_some() {
                        "Edit Card"
                    } else {
                        "Select a Card"
                    }
                    if show_dirty {
                        span { class: "editor-detail-dirty", "• Unsaved" }
                    }
                }
            }

            div { class: "editor-body",
                if !can_edit {
                    p { class: "editor-empty-hint", "Select a card or click + New Card." }
                }
                div { class: "editor-group editor-group--editor",
                    div { class: "editor-field-header",
                        label { class: "editor-label", r#for: "prompt", "Front" }
                    }
                    EditorFormatToolbar {
                        field: MarkdownField::Front,
                        disabled: prompt_toolbar_disabled,
                        writing_menu_state: writing_tools_menu_state,
                        writing_prompt: writing_tools_prompt.clone(),
                        writing_tone: writing_tools_tone,
                        writing_result_status: writing_tools_result_status,
                        writing_result_target: writing_tools_result_target,
                        writing_result_title: writing_tools_result_title.clone(),
                        writing_result_html: writing_tools_result_html.clone(),
                        on_format,
                        on_block_dir,
                        on_toggle_writing_menu: on_toggle_writing_tools,
                        on_writing_prompt_change: on_update_writing_tools_prompt,
                        on_select_writing_tone: on_select_writing_tools_tone,
                        on_select_writing_command: on_select_writing_tools_command,
                        on_writing_result_replace: on_replace_writing_tools,
                        on_writing_result_copy: on_copy_writing_tools,
                    }
                    div {
                        id: "prompt",
                        class: if prompt_invalid {
                            "editor-input editor-input--multi editor-input--error"
                        } else {
                            "editor-input editor-input--multi"
                        },
                        contenteditable: "{can_edit}",
                        dir: "auto",
                        aria_label: "Front",
                        role: "textbox",
                        aria_multiline: "true",
                        aria_placeholder: "Enter the prompt for the front of the card...",
                        spellcheck: "true",
                        tabindex: "0",
                        onfocus: move |_| on_focus_field.call(MarkdownField::Front),
                        oninput: move |_| on_prompt_input.call(()),
                        onpaste: move |evt| {
                            evt.prevent_default();
                            on_prompt_paste.call(());
                        },
                    }
                    if prompt_invalid {
                        p { class: "editor-error", "Front is required." }
                    }
                }

                div { class: "editor-group editor-group--editor",
                    div { class: "editor-field-header",
                        label { class: "editor-label", r#for: "answer", "Back" }
                    }
                    EditorFormatToolbar {
                        field: MarkdownField::Back,
                        disabled: answer_toolbar_disabled,
                        writing_menu_state: writing_tools_menu_state,
                        writing_prompt: writing_tools_prompt,
                        writing_tone: writing_tools_tone,
                        writing_result_status: writing_tools_result_status,
                        writing_result_target: writing_tools_result_target,
                        writing_result_title: writing_tools_result_title,
                        writing_result_html: writing_tools_result_html,
                        on_format,
                        on_block_dir,
                        on_toggle_writing_menu: on_toggle_writing_tools,
                        on_writing_prompt_change: on_update_writing_tools_prompt,
                        on_select_writing_tone: on_select_writing_tools_tone,
                        on_select_writing_command: on_select_writing_tools_command,
                        on_writing_result_replace: on_replace_writing_tools,
                        on_writing_result_copy: on_copy_writing_tools,
                    }
                    div {
                        id: "answer",
                        class: if answer_invalid {
                            "editor-input editor-input--multi editor-input--error"
                        } else {
                            "editor-input editor-input--multi"
                        },
                        contenteditable: "{can_edit}",
                        dir: "auto",
                        aria_label: "Back",
                        role: "textbox",
                        aria_multiline: "true",
                        aria_placeholder: "Enter the answer for the back of the card...",
                        spellcheck: "true",
                        tabindex: "0",
                        onfocus: move |_| on_focus_field.call(MarkdownField::Back),
                        oninput: move |_| on_answer_input.call(()),
                        onpaste: move |evt| {
                            evt.prevent_default();
                            on_answer_paste.call(());
                        },
                    }
                    if answer_invalid {
                        p { class: "editor-error", "Back is required." }
                    }
                }

                div { class: "editor-group",
                    label { class: "editor-label", "Tags" }
                    div { class: "editor-tag-input",
                        for tag in card_tags {
                            span { class: "editor-tag-chip",
                                "{tag}"
                                if can_edit {
                                    button {
                                        class: "editor-tag-remove",
                                        r#type: "button",
                                        aria_label: "Remove tag",
                                        onclick: move |_| on_tag_remove.call(tag.clone()),
                                        "×"
                                    }
                                }
                            }
                        }
                        input {
                            class: "editor-tag-field",
                            r#type: "text",
                            placeholder: "Add tag",
                            value: "{tag_input_value}",
                            disabled: !can_edit,
                            oninput: move |evt| on_tag_input_change.call(evt.value()),
                            onkeydown: move |evt| match evt.data.key() {
                                Key::Enter => {
                                    evt.prevent_default();
                                    on_tag_add.call(tag_input_for_keydown.clone());
                                }
                                Key::Character(value) if value == "," => {
                                    evt.prevent_default();
                                    on_tag_add.call(tag_input_for_keydown.clone());
                                }
                                Key::Backspace => {
                                    if tag_input_for_keydown.trim().is_empty()
                                        && let Some(last_tag) = card_tags_for_backspace.last()
                                    {
                                        on_tag_remove.call(last_tag.clone());
                                    }
                                }
                                _ => {}
                            },
                            onblur: move |_| {
                                let value = tag_input_for_blur.trim().to_string();
                                if !value.is_empty() {
                                    on_tag_add.call(value);
                                }
                            },
                        }
                    }
                    if can_edit && !tag_suggestions.is_empty() {
                        div { class: "editor-tag-suggestions",
                            for suggestion in tag_suggestions {
                                button {
                                    class: "editor-tag-suggestion",
                                    r#type: "button",
                                    onclick: move |_| on_tag_add.call(suggestion.clone()),
                                    "{suggestion}"
                                }
                            }
                        }
                    }
                }
            }

            footer { class: "editor-footer",
                div { class: "editor-status",
                    if let Some(message) = daily_limit_warning {
                        span { class: "editor-warning", "{message}" }
                    }
                    match delete_state {
                        DeleteState::Idle => match duplicate_check_state {
                            DuplicateCheckState::Checking => rsx! { span { "Checking..." } },
                            DuplicateCheckState::Error(err) => rsx! { span { "{err.message()}" } },
                            DuplicateCheckState::Idle => match save_state {
                                SaveState::Idle => {
                                    if can_edit && !has_unsaved_changes {
                                        rsx! { span { "No changes." } }
                                    } else {
                                        rsx! {}
                                    }
                                }
                                SaveState::Saving => rsx! { span { "Saving..." } },
                                SaveState::Success => rsx! { span { "Saved." } },
                                SaveState::Error(err) => rsx! { span { "{err.message()}" } },
                            },
                        },
                        DeleteState::Deleting => rsx! { span { "Deleting..." } },
                        DeleteState::Success => rsx! { span { "Deleted." } },
                        DeleteState::Error(err) => rsx! { span { "{err.message()}" } },
                    }
                }
                div { class: "editor-actions",
                    button {
                        class: "btn editor-cancel",
                        r#type: "button",
                        disabled: !can_cancel,
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    if can_show_delete {
                        button {
                            class: "btn editor-delete",
                            r#type: "button",
                            disabled: delete_state == DeleteState::Deleting
                                || save_state == SaveState::Saving,
                            onclick: move |_| on_open_delete.call(()),
                            "Delete"
                        }
                    }
                    div { class: "editor-save-wrapper",
                        if is_create_mode {
                            button {
                                class: "btn editor-save editor-save-split",
                                r#type: "button",
                                disabled: !can_submit,
                                onclick: move |_| on_save.call(SaveRequest::new(false)),
                                span { class: "editor-save-label", "Save" }
                                span {
                                    class: "editor-save-caret",
                                    onclick: move |evt| {
                                        evt.stop_propagation();
                                        on_toggle_save_menu.call(());
                                    },
                                    svg {
                                        class: "editor-save-caret-icon",
                                        view_box: "0 0 12 12",
                                        path {
                                            d: "M2.5 4.5l3.5 3.5 3.5-3.5",
                                            stroke_linecap: "round",
                                            stroke_linejoin: "round",
                                        }
                                    }
                                }
                            }
                        } else {
                            button {
                                class: "btn btn-primary editor-save",
                                r#type: "button",
                                disabled: !can_submit,
                                onclick: move |_| on_save.call(SaveRequest::new(false)),
                                "Save"
                            }
                        }
                        if save_menu_state == SaveMenuState::Open {
                            div {
                                class: "editor-save-menu",
                                onclick: move |evt| evt.stop_propagation(),
                                button {
                                    class: "editor-save-item",
                                    r#type: "button",
                                    onclick: move |_| {
                                        on_close_save_menu.call(());
                                        on_save.call(SaveRequest::new(true));
                                    },
                                    "Save & Practice"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

use dioxus::document::eval;
use dioxus::prelude::*;

use crate::vm::{MarkdownAction, MarkdownField, looks_like_markdown, markdown_to_html, sanitize_html};

use super::super::scripts::{
    exec_command_script, insert_html_script, insert_text_script, read_clipboard_snapshot,
    read_editable_html, set_block_dir_script, wrap_selection_script,
};
use super::super::state::{EditorState, SaveState};

pub(super) type FormatCallbacks = (
    Callback<(MarkdownField, MarkdownAction)>,
    Callback<(MarkdownField, String)>,
);

pub(super) fn build_paste_action(state: &EditorState) -> Callback<MarkdownField> {
    let state = state.clone();
    use_callback(move |target: MarkdownField| {
        let mut prompt_text = state.prompt_text;
        let mut answer_text = state.answer_text;
        let mut save_state = state.save_state;
        spawn(async move {
            if let Some(snapshot) = read_clipboard_snapshot().await {
                let element_id = match target {
                    MarkdownField::Front => "prompt",
                    MarkdownField::Back => "answer",
                };
                let html = snapshot.html.trim();
                if !html.is_empty() {
                    let sanitized = sanitize_html(html);
                    let script = insert_html_script(element_id, &sanitized);
                    let _ = eval(&script).await;
                } else if looks_like_markdown(&snapshot.text) {
                    let html = markdown_to_html(&snapshot.text);
                    let script = insert_html_script(element_id, &html);
                    let _ = eval(&script).await;
                } else if !snapshot.text.is_empty() {
                    let script = insert_text_script(element_id, &snapshot.text);
                    let _ = eval(&script).await;
                } else {
                    return;
                }

                if let Some(updated) = read_editable_html(element_id).await {
                    match target {
                        MarkdownField::Front => prompt_text.set(updated),
                        MarkdownField::Back => answer_text.set(updated),
                    }
                }
                save_state.set(SaveState::Idle);
            }
        });
    })
}

pub(super) fn build_format_actions(state: &EditorState) -> FormatCallbacks {
    let state_for_format = state.clone();
    let apply_format_action = use_callback(move |(field, action): (MarkdownField, MarkdownAction)| {
        let mut prompt_text = state_for_format.prompt_text;
        let mut answer_text = state_for_format.answer_text;
        let mut save_state = state_for_format.save_state;
        spawn(async move {
            let element_id = match field {
                MarkdownField::Front => "prompt",
                MarkdownField::Back => "answer",
            };
            let _ = eval(&format!(
                r#"document.getElementById("{element_id}")?.focus();"#
            ))
            .await;
            let script = match action {
                MarkdownAction::Bold => exec_command_script(element_id, "bold", None),
                MarkdownAction::Italic => exec_command_script(element_id, "italic", None),
                MarkdownAction::Link => exec_command_script(element_id, "createLink", Some("https://")),
                MarkdownAction::Quote => exec_command_script(element_id, "formatBlock", Some("blockquote")),
                MarkdownAction::BulletList => {
                    exec_command_script(element_id, "insertUnorderedList", None)
                }
                MarkdownAction::NumberedList => {
                    exec_command_script(element_id, "insertOrderedList", None)
                }
                MarkdownAction::Code => wrap_selection_script(element_id, "code", None),
                MarkdownAction::CodeBlock => wrap_selection_script(element_id, "pre", Some("code")),
            };
            let _ = eval(&script).await;
            if let Some(updated) = read_editable_html(element_id).await {
                match field {
                    MarkdownField::Front => prompt_text.set(updated),
                    MarkdownField::Back => answer_text.set(updated),
                }
            }
            save_state.set(SaveState::Idle);
        });
    });

    let state_for_dir = state.clone();
    let apply_block_dir_action = use_callback(move |(field, direction): (MarkdownField, String)| {
        let mut save_state = state_for_dir.save_state;
        let mut prompt_text = state_for_dir.prompt_text;
        let mut answer_text = state_for_dir.answer_text;
        spawn(async move {
            let element_id = match field {
                MarkdownField::Front => "prompt",
                MarkdownField::Back => "answer",
            };
            let script = set_block_dir_script(element_id, &direction);
            let _ = eval(&script).await;
            if let Some(updated) = read_editable_html(element_id).await {
                match field {
                    MarkdownField::Front => prompt_text.set(updated),
                    MarkdownField::Back => answer_text.set(updated),
                }
            }
            save_state.set(SaveState::Idle);
        });
    });

    (apply_format_action, apply_block_dir_action)
}

use dioxus::prelude::*;

use crate::vm::MarkdownField;

use super::super::scripts::{
    apply_link, read_editable_html, read_selected_link_href, remove_link, save_selection_range,
};
use super::super::state::{EditorState, LinkEditorState, SaveState, SaveMenuState, WritingToolsMenuState, WritingToolsResultStatus};

pub(super) fn build_open_link_editor_action(state: &EditorState) -> Callback<MarkdownField> {
    let state = state.clone();
    use_callback(move |field: MarkdownField| {
        let mut link_editor_state = state.link_editor_state;
        let mut save_menu_state = state.save_menu_state;
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let mut writing_tools_result_status = state.writing_tools_result_status;
        let mut writing_tools_result_target = state.writing_tools_result_target;
        let mut writing_tools_request = state.writing_tools_request;

        if matches!(link_editor_state(), Some(existing) if existing.field == field) {
            link_editor_state.set(None);
            return;
        }

        save_menu_state.set(SaveMenuState::Closed);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
        writing_tools_result_status.set(WritingToolsResultStatus::Idle);
        writing_tools_result_target.set(None);
        writing_tools_request.set(None);

        link_editor_state.set(Some(LinkEditorState {
            field,
            url: "https://".to_string(),
        }));

        let element_id = match field {
            MarkdownField::Front => "prompt",
            MarkdownField::Back => "answer",
        };
        spawn(async move {
            save_selection_range(element_id).await;
            let href = read_selected_link_href(element_id).await.unwrap_or_default();
            let url = if href.trim().is_empty() {
                "https://".to_string()
            } else {
                href
            };
            link_editor_state.set(Some(LinkEditorState { field, url }));
        });
    })
}

pub(super) fn build_close_link_editor_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut link_editor_state = state.link_editor_state;
        link_editor_state.set(None);
    })
}

pub(super) fn build_update_link_url_action(state: &EditorState) -> Callback<String> {
    let state = state.clone();
    use_callback(move |value: String| {
        let mut link_editor_state = state.link_editor_state;
        if let Some(current) = link_editor_state() {
            link_editor_state.set(Some(LinkEditorState {
                field: current.field,
                url: value,
            }));
        }
    })
}

pub(super) fn build_apply_link_action(state: &EditorState) -> Callback<MarkdownField> {
    let state = state.clone();
    use_callback(move |field: MarkdownField| {
        let mut link_editor_state = state.link_editor_state;
        let link_state = link_editor_state();
        let Some(link_state) = link_state else {
            return;
        };
        if link_state.field != field {
            return;
        }
        let url = link_state.url.trim().to_string();
        if url.is_empty() {
            link_editor_state.set(None);
            return;
        }
        let mut save_state = state.save_state;
        let mut prompt_text = state.prompt_text;
        let mut answer_text = state.answer_text;
        let element_id = match field {
            MarkdownField::Front => "prompt",
            MarkdownField::Back => "answer",
        };
        link_editor_state.set(None);
        spawn(async move {
            apply_link(element_id, &url).await;
            if let Some(updated) = read_editable_html(element_id).await {
                match field {
                    MarkdownField::Front => prompt_text.set(updated),
                    MarkdownField::Back => answer_text.set(updated),
                }
            }
            save_state.set(SaveState::Idle);
        });
    })
}

pub(super) fn build_remove_link_action(state: &EditorState) -> Callback<MarkdownField> {
    let state = state.clone();
    use_callback(move |field: MarkdownField| {
        let mut link_editor_state = state.link_editor_state;
        let mut save_state = state.save_state;
        let mut prompt_text = state.prompt_text;
        let mut answer_text = state.answer_text;
        let element_id = match field {
            MarkdownField::Front => "prompt",
            MarkdownField::Back => "answer",
        };
        link_editor_state.set(None);
        spawn(async move {
            remove_link(element_id).await;
            if let Some(updated) = read_editable_html(element_id).await {
                match field {
                    MarkdownField::Front => prompt_text.set(updated),
                    MarkdownField::Back => answer_text.set(updated),
                }
            }
            save_state.set(SaveState::Idle);
        });
    })
}

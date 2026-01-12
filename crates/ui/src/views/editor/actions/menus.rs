use std::rc::Rc;

use dioxus::prelude::*;

use super::super::state::{
    DuplicateCheckState, EditorState, PendingAction, SaveMenuState, SaveRequest, SaveState,
    WritingToolsCommand, WritingToolsMenuState, WritingToolsRequest, WritingToolsResultStatus,
    WritingToolsTone,
};
use crate::vm::{CardListItemVm, MarkdownField, strip_html_tags};
use crate::views::editor::scripts::{read_selection_snapshot, replace_selection_or_all, write_clipboard};

pub(super) fn build_discard_actions(
    state: &EditorState,
    select_card_action: Callback<CardListItemVm>,
    apply_select_deck_action: Callback<learn_core::model::DeckId>,
    new_card_action: Callback<()>,
) -> (Callback<()>, Callback<()>) {
    let state_for_confirm = state.clone();
    let confirm_discard_action = use_callback(move |()| {
        let mut show_unsaved_modal = state_for_confirm.show_unsaved_modal;
        let mut pending_action = state_for_confirm.pending_action;
        let mut save_menu_state = state_for_confirm.save_menu_state;
        let mut writing_tools_menu_state = state_for_confirm.writing_tools_menu_state;
        let mut writing_tools_result_status = state_for_confirm.writing_tools_result_status;
        let mut writing_tools_result_target = state_for_confirm.writing_tools_result_target;
        let mut writing_tools_request = state_for_confirm.writing_tools_request;
        let mut link_editor_state = state_for_confirm.link_editor_state;
        if let Some(action) = pending_action() {
            match action {
                PendingAction::SelectCard(item) => {
                    select_card_action.call(item);
                }
                PendingAction::SelectDeck(deck_id) => {
                    apply_select_deck_action.call(deck_id);
                }
                PendingAction::NewCard => {
                    new_card_action.call(());
                }
            }
        }
        show_unsaved_modal.set(false);
        pending_action.set(None);
        save_menu_state.set(SaveMenuState::Closed);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
        writing_tools_result_status.set(WritingToolsResultStatus::Idle);
        writing_tools_result_target.set(None);
        writing_tools_request.set(None);
        link_editor_state.set(None);
    });

    let state_for_cancel = state.clone();
    let cancel_discard_action = use_callback(move |()| {
        let mut show_unsaved_modal = state_for_cancel.show_unsaved_modal;
        let mut pending_action = state_for_cancel.pending_action;
        let mut save_menu_state = state_for_cancel.save_menu_state;
        let mut writing_tools_menu_state = state_for_cancel.writing_tools_menu_state;
        let mut writing_tools_result_status = state_for_cancel.writing_tools_result_status;
        let mut writing_tools_result_target = state_for_cancel.writing_tools_result_target;
        let mut writing_tools_request = state_for_cancel.writing_tools_request;
        let mut link_editor_state = state_for_cancel.link_editor_state;
        show_unsaved_modal.set(false);
        pending_action.set(None);
        save_menu_state.set(SaveMenuState::Closed);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
        writing_tools_result_status.set(WritingToolsResultStatus::Idle);
        writing_tools_result_target.set(None);
        writing_tools_request.set(None);
        link_editor_state.set(None);
    });

    (confirm_discard_action, cancel_discard_action)
}

pub(super) fn build_open_delete_modal_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    let reset_duplicate_state = Rc::clone(&state.reset_duplicate_state);
    use_callback(move |()| {
        let reset_duplicate_state = Rc::clone(&reset_duplicate_state);
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_deck_menu = state.show_deck_menu;
        let mut show_deck_actions = state.show_deck_actions;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut pending_action = state.pending_action;
        let mut save_menu_state = state.save_menu_state;
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let mut writing_tools_result_status = state.writing_tools_result_status;
        let mut writing_tools_result_target = state.writing_tools_result_target;
        let mut writing_tools_request = state.writing_tools_request;
        let mut link_editor_state = state.link_editor_state;
        let selected_card_id = (state.selected_card_id)();
        if selected_card_id.is_some() {
            show_deck_menu.set(false);
            show_deck_actions.set(false);
            is_renaming_deck.set(false);
            rename_deck_state.set(SaveState::Idle);
            rename_deck_error.set(None);
            show_unsaved_modal.set(false);
            pending_action.set(None);
            save_menu_state.set(SaveMenuState::Closed);
            writing_tools_menu_state.set(WritingToolsMenuState::Closed);
            writing_tools_result_status.set(WritingToolsResultStatus::Idle);
            writing_tools_result_target.set(None);
            writing_tools_request.set(None);
            link_editor_state.set(None);
            reset_duplicate_state.borrow_mut()();
            show_delete_modal.set(true);
        }
    })
}

pub(super) fn build_toggle_save_menu_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut save_menu_state = state.save_menu_state;
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let mut writing_tools_result_status = state.writing_tools_result_status;
        let mut writing_tools_result_target = state.writing_tools_result_target;
        let mut writing_tools_request = state.writing_tools_request;
        let mut link_editor_state = state.link_editor_state;
        if save_menu_state() == SaveMenuState::Open {
            save_menu_state.set(SaveMenuState::Closed);
        } else {
            show_delete_modal.set(false);
            show_unsaved_modal.set(false);
            writing_tools_menu_state.set(WritingToolsMenuState::Closed);
            writing_tools_result_status.set(WritingToolsResultStatus::Idle);
            writing_tools_result_target.set(None);
            writing_tools_request.set(None);
            link_editor_state.set(None);
            save_menu_state.set(SaveMenuState::Open);
        }
    })
}

pub(super) fn build_close_save_menu_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut save_menu_state = state.save_menu_state;
        save_menu_state.set(SaveMenuState::Closed);
    })
}

pub(super) fn build_toggle_writing_tools_action(state: &EditorState) -> Callback<MarkdownField> {
    let state = state.clone();
    use_callback(move |field: MarkdownField| {
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let mut save_menu_state = state.save_menu_state;
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut show_deck_menu = state.show_deck_menu;
        let mut show_deck_actions = state.show_deck_actions;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;
        let mut show_new_deck = state.show_new_deck;
        let mut new_deck_state = state.new_deck_state;
        let mut writing_tools_result_status = state.writing_tools_result_status;
        let mut writing_tools_result_target = state.writing_tools_result_target;
        let mut writing_tools_selection_html = state.writing_tools_selection_html;
        let mut writing_tools_selection_text = state.writing_tools_selection_text;
        let mut link_editor_state = state.link_editor_state;
        match writing_tools_menu_state() {
            WritingToolsMenuState::Open(current) if current == field => {
                writing_tools_menu_state.set(WritingToolsMenuState::Closed);
                writing_tools_selection_html.set(String::new());
                writing_tools_selection_text.set(String::new());
            }
            _ => {
                save_menu_state.set(SaveMenuState::Closed);
                show_delete_modal.set(false);
                show_unsaved_modal.set(false);
                show_deck_menu.set(false);
                show_deck_actions.set(false);
                is_renaming_deck.set(false);
                rename_deck_state.set(SaveState::Idle);
                rename_deck_error.set(None);
                show_new_deck.set(false);
                new_deck_state.set(SaveState::Idle);
                writing_tools_result_status.set(WritingToolsResultStatus::Idle);
                writing_tools_result_target.set(None);
                writing_tools_menu_state.set(WritingToolsMenuState::Open(field));
                link_editor_state.set(None);
                let element_id = match field {
                    MarkdownField::Front => "prompt",
                    MarkdownField::Back => "answer",
                };
                spawn(async move {
                    if let Some(snapshot) = read_selection_snapshot(element_id).await {
                        writing_tools_selection_html.set(snapshot.html);
                        writing_tools_selection_text.set(snapshot.text);
                    } else {
                        writing_tools_selection_html.set(String::new());
                        writing_tools_selection_text.set(String::new());
                    }
                });
            }
        }
    })
}

pub(super) fn build_close_writing_tools_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let mut writing_tools_result_status = state.writing_tools_result_status;
        let mut writing_tools_result_target = state.writing_tools_result_target;
        let mut writing_tools_result_title = state.writing_tools_result_title;
        let mut writing_tools_result_body = state.writing_tools_result_body;
        let mut writing_tools_result_html = state.writing_tools_result_html;
        let mut writing_tools_selection_html = state.writing_tools_selection_html;
        let mut writing_tools_selection_text = state.writing_tools_selection_text;
        let mut writing_tools_request = state.writing_tools_request;
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
        writing_tools_result_status.set(WritingToolsResultStatus::Idle);
        writing_tools_result_target.set(None);
        writing_tools_result_title.set(String::new());
        writing_tools_result_body.set(String::new());
        writing_tools_result_html.set(String::new());
        writing_tools_selection_html.set(String::new());
        writing_tools_selection_text.set(String::new());
        writing_tools_request.set(None);
    })
}

pub(super) fn build_update_writing_tools_prompt_action(state: &EditorState) -> Callback<String> {
    let state = state.clone();
    use_callback(move |value: String| {
        let mut writing_tools_prompt = state.writing_tools_prompt;
        writing_tools_prompt.set(value);
    })
}

pub(super) fn build_select_writing_tools_tone_action(
    state: &EditorState,
) -> Callback<WritingToolsTone> {
    let state = state.clone();
    use_callback(move |tone: WritingToolsTone| {
        let mut writing_tools_tone = state.writing_tools_tone;
        writing_tools_tone.set(tone);
    })
}

pub(super) fn build_select_writing_tools_command_action(
    state: &EditorState,
) -> Callback<(MarkdownField, WritingToolsCommand)> {
    let state = state.clone();
    use_callback(move |(field, command): (MarkdownField, WritingToolsCommand)| {
        let mut writing_tools_last_command = state.writing_tools_last_command;
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let mut writing_tools_result_status = state.writing_tools_result_status;
        let mut writing_tools_result_target = state.writing_tools_result_target;
        let mut writing_tools_result_title = state.writing_tools_result_title;
        let mut writing_tools_result_body = state.writing_tools_result_body;
        let mut writing_tools_request = state.writing_tools_request;
        let mut last_focus_field = state.last_focus_field;
        let selection_html = state.writing_tools_selection_html.read().to_string();
        let selection_text = state.writing_tools_selection_text.read().to_string();
        let tone = (state.writing_tools_tone)();
        let user_prompt = state.writing_tools_prompt.read().to_string();
        writing_tools_last_command.set(Some(command));
        last_focus_field.set(field);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
        writing_tools_result_target.set(Some(field));
        writing_tools_result_status.set(WritingToolsResultStatus::Loading);
        writing_tools_result_title.set(build_writing_tools_result_title(command, tone));
        writing_tools_result_body.set("Waiting for response…".to_string());
        let fallback_html = match field {
            MarkdownField::Front => state.prompt_text.read().to_string(),
            MarkdownField::Back => state.answer_text.read().to_string(),
        };
        spawn(async move {
            let source_html = if !selection_html.trim().is_empty() {
                selection_html.clone()
            } else if !selection_text.trim().is_empty() {
                selection_text.clone()
            } else {
                fallback_html.clone()
            };
            let source_text = strip_html_tags(&source_html);
            let request_prompt = build_writing_tools_prompt(
                command,
                tone,
                &user_prompt,
                &source_text,
            );
            writing_tools_request.set(Some(WritingToolsRequest {
                field,
                command,
                tone,
                user_prompt,
                source_text,
                request_prompt,
            }));
        });
    })
}

pub(super) fn build_replace_writing_tools_action(
    state: &EditorState,
) -> Callback<MarkdownField> {
    let state = state.clone();
    use_callback(move |field: MarkdownField| {
        let html = normalize_writing_tools_html(&state.writing_tools_result_html.read());
        if html.trim().is_empty() {
            return;
        }
        let mut prompt_text = state.prompt_text;
        let mut answer_text = state.answer_text;
        let mut prompt_render_html = state.prompt_render_html;
        let mut answer_render_html = state.answer_render_html;
        let mut save_state = state.save_state;
        let element_id = match field {
            MarkdownField::Front => "prompt",
            MarkdownField::Back => "answer",
        };
        let html_for_script = html.clone();
        spawn(async move {
            replace_selection_or_all(element_id, &html_for_script).await;
            if let Some(updated) = crate::views::editor::scripts::read_editable_html(element_id).await {
                match field {
                    MarkdownField::Front => {
                        prompt_text.set(updated.clone());
                        prompt_render_html.set(updated);
                    }
                    MarkdownField::Back => {
                        answer_text.set(updated.clone());
                        answer_render_html.set(updated);
                    }
                }
                save_state.set(SaveState::Idle);
            }
        });
    })
}

pub(super) fn build_copy_writing_tools_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let html = state.writing_tools_result_html.read().to_string();
        if html.trim().is_empty() {
            return;
        }
        let text = strip_html_tags(&html);
        spawn(async move {
            write_clipboard(&html, &text).await;
        });
    })
}

fn normalize_writing_tools_html(input: &str) -> String {
    let mut out = input.trim().to_string();
    let empty_para = "<p><br></p>";
    loop {
        let trimmed = out.trim().to_string();
        let mut changed = false;
        if trimmed.starts_with(empty_para) {
            out = trimmed[empty_para.len()..].to_string();
            changed = true;
        }
        if trimmed.ends_with(empty_para) {
            out = trimmed[..trimmed.len().saturating_sub(empty_para.len())].to_string();
            changed = true;
        }
        if trimmed.starts_with("<br>") {
            out = trimmed[4..].to_string();
            changed = true;
        }
        if trimmed.ends_with("<br>") {
            out = trimmed[..trimmed.len().saturating_sub(4)].to_string();
            changed = true;
        }
        if !changed {
            break;
        }
    }

    let trimmed = out.trim().to_string();
    if trimmed.starts_with("<p>")
        && trimmed.ends_with("</p>")
        && !trimmed[3..trimmed.len().saturating_sub(4)].contains("<p>")
        && !trimmed[3..trimmed.len().saturating_sub(4)].contains("</p>")
    {
        return trimmed[3..trimmed.len() - 4].to_string();
    }

    trimmed
}

fn writing_tools_command_label(command: WritingToolsCommand) -> &'static str {
    match command {
        WritingToolsCommand::ImproveWording => "Improve wording",
        WritingToolsCommand::Simplify => "Simplify",
        WritingToolsCommand::Concise => "Concise",
        WritingToolsCommand::Summary => "Summary",
        WritingToolsCommand::KeyPoints => "Key points",
        WritingToolsCommand::List => "List",
        WritingToolsCommand::TurnIntoQuestion => "Turn into question",
    }
}

fn build_writing_tools_result_title(
    command: WritingToolsCommand,
    tone: WritingToolsTone,
) -> String {
    match command {
        WritingToolsCommand::ImproveWording => match tone {
            WritingToolsTone::Clear => "Clear",
            WritingToolsTone::Simple => "Simple",
            WritingToolsTone::Formal => "Formal",
        }
        .to_string(),
        _ => writing_tools_command_label(command).to_string(),
    }
}

fn build_writing_tools_prompt(
    command: WritingToolsCommand,
    tone: WritingToolsTone,
    user_prompt: &str,
    source_text: &str,
) -> String {
    let tone_label = match tone {
        WritingToolsTone::Clear => "Clear",
        WritingToolsTone::Simple => "Simple",
        WritingToolsTone::Formal => "Formal",
    };
    let action_label = writing_tools_command_label(command);
    let action_prompt = match command {
        WritingToolsCommand::ImproveWording => {
            "Rewrite for clarity and natural flow while preserving meaning.\nKeep the same intent and key details.\nAvoid adding new information.\nPreserve formatting unless improving readability.\n\nOutput JSON: result=rewritten text, title=\"\", notes=\"\"."
        }
        WritingToolsCommand::Simplify => {
            "Simplify the text to be easier to understand and remember.\nUse simpler words and shorter sentences.\nDo not remove important details.\n\nOutput JSON: result=simplified text, title=\"\", notes=\"\"."
        }
        WritingToolsCommand::Concise => {
            "Make the text significantly shorter while preserving meaning and key details.\nRemove redundancy, filler, and hedging.\nKeep critical constraints, numbers, and names.\n\nOutput JSON: result=shortened text, title=\"\", notes=\"\"."
        }
        WritingToolsCommand::Summary => {
            "Summarize the input into 1-3 sentences.\nKeep only the main idea and critical details.\nNo bullets.\n\nOutput JSON:\n- title: short 2-6 word heading (or \"\")\n- result: summary paragraph\n- notes: \"\""
        }
        WritingToolsCommand::KeyPoints => {
            "Extract 3-7 key points.\nEach bullet should be <= 12 words.\nUse \"- \" bullets.\n\nOutput JSON:\n- title: \"Key points\"\n- result: bullet list as a single string\n- notes: \"\""
        }
        WritingToolsCommand::List => {
            "Convert the content into an actionable numbered list (3-10 items).\nUse imperative phrasing when possible.\n\nOutput JSON:\n- title: \"List\"\n- result: numbered list as a single string\n- notes: \"\""
        }
        WritingToolsCommand::TurnIntoQuestion => {
            "Turn the content into 1-5 questions for recall/testing.\nQuestions should be direct and answerable from the input only.\nPrefer \"why/how/what\" questions when useful.\n\nOutput JSON:\n- title: \"Questions\"\n- result: numbered list of questions\n- notes: \"\""
        }
    };

    let constraints = user_prompt.trim();
    let text = source_text.trim();
    let mut prompt = String::new();
    prompt.push_str(action_prompt);
    prompt.push_str(
        "\n\nReturn only a JSON object with keys: result, title, notes. Do not use code fences.",
    );
    prompt.push_str("\n\nTONE: ");
    prompt.push_str(tone_label);
    prompt.push_str("\nACTION: ");
    prompt.push_str(action_label);
    prompt.push_str("\nCONSTRAINTS: ");
    if !constraints.is_empty() {
        prompt.push_str(constraints);
    }
    prompt.push_str("\nTEXT:\n<<<\n");
    prompt.push_str(text);
    prompt.push_str("\n>>>");

    prompt
}

pub(super) fn build_close_delete_modal_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut show_delete_modal = state.show_delete_modal;
        show_delete_modal.set(false);
    })
}

pub(super) fn build_close_duplicate_modal_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut show_duplicate_modal = state.show_duplicate_modal;
        let mut pending_duplicate_practice = state.pending_duplicate_practice;
        let mut duplicate_check_state = state.duplicate_check_state;
        show_duplicate_modal.set(false);
        pending_duplicate_practice.set(false);
        duplicate_check_state.set(DuplicateCheckState::Idle);
    })
}

pub(super) fn build_confirm_duplicate_action(
    state: &EditorState,
    save_action: Callback<SaveRequest>,
) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut show_duplicate_modal = state.show_duplicate_modal;
        let mut pending_duplicate_practice = state.pending_duplicate_practice;
        let mut duplicate_check_state = state.duplicate_check_state;
        let practice = pending_duplicate_practice();
        show_duplicate_modal.set(false);
        pending_duplicate_practice.set(false);
        duplicate_check_state.set(DuplicateCheckState::Idle);
        save_action.call(SaveRequest::force(practice));
    })
}

use std::rc::Rc;

use dioxus::prelude::*;

use super::super::state::{
    DuplicateCheckState, EditorState, PendingAction, SaveMenuState, SaveRequest, SaveState,
    WritingToolsCommand, WritingToolsMenuState, WritingToolsRequest, WritingToolsResultStatus,
    WritingToolsTone,
};
use crate::vm::{CardListItemVm, MarkdownField, strip_html_tags};

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
        show_unsaved_modal.set(false);
        pending_action.set(None);
        save_menu_state.set(SaveMenuState::Closed);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
        writing_tools_result_status.set(WritingToolsResultStatus::Idle);
        writing_tools_result_target.set(None);
        writing_tools_request.set(None);
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
        if save_menu_state() == SaveMenuState::Open {
            save_menu_state.set(SaveMenuState::Closed);
        } else {
            show_delete_modal.set(false);
            show_unsaved_modal.set(false);
            writing_tools_menu_state.set(WritingToolsMenuState::Closed);
            writing_tools_result_status.set(WritingToolsResultStatus::Idle);
            writing_tools_result_target.set(None);
            writing_tools_request.set(None);
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
        match writing_tools_menu_state() {
            WritingToolsMenuState::Open(current) if current == field => {
                writing_tools_menu_state.set(WritingToolsMenuState::Closed);
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
        let mut writing_tools_request = state.writing_tools_request;
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
        writing_tools_result_status.set(WritingToolsResultStatus::Idle);
        writing_tools_result_target.set(None);
        writing_tools_result_title.set(String::new());
        writing_tools_result_body.set(String::new());
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
        let tone = (state.writing_tools_tone)();
        let user_prompt = state.writing_tools_prompt.read().to_string();
        let source_html = match field {
            MarkdownField::Front => state.prompt_text.read().to_string(),
            MarkdownField::Back => state.answer_text.read().to_string(),
        };
        let source_text = strip_html_tags(&source_html);
        let request_prompt = build_writing_tools_prompt(
            command,
            tone,
            &user_prompt,
            &source_text,
        );
        writing_tools_last_command.set(Some(command));
        last_focus_field.set(field);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
        writing_tools_result_target.set(Some(field));
        writing_tools_result_status.set(WritingToolsResultStatus::Loading);
        writing_tools_result_title.set(build_writing_tools_result_title(command, tone));
        writing_tools_result_body.set("Waiting for response…".to_string());
        writing_tools_request.set(Some(WritingToolsRequest {
            field,
            command,
            tone,
            user_prompt,
            source_text,
            request_prompt,
        }));
    })
}

fn writing_tools_command_label(command: WritingToolsCommand) -> &'static str {
    match command {
        WritingToolsCommand::Proofread => "Proofread",
        WritingToolsCommand::Rewrite => "Rewrite",
        WritingToolsCommand::Summary => "Summary",
        WritingToolsCommand::KeyPoints => "Key Points",
        WritingToolsCommand::List => "List",
        WritingToolsCommand::Table => "Table",
        WritingToolsCommand::Compose => "Compose",
    }
}

fn build_writing_tools_result_title(
    command: WritingToolsCommand,
    tone: WritingToolsTone,
) -> String {
    match command {
        WritingToolsCommand::Rewrite => match tone {
            WritingToolsTone::Friendly => "Friendly",
            WritingToolsTone::Professional => "Professional",
            WritingToolsTone::Concise => "Concise",
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
        WritingToolsTone::Friendly => "friendly",
        WritingToolsTone::Professional => "professional",
        WritingToolsTone::Concise => "concise",
    };
    let mut prompt = match command {
        WritingToolsCommand::Proofread => {
            "Proofread the following text for grammar, spelling, and clarity. Keep the meaning and tone.".to_string()
        }
        WritingToolsCommand::Rewrite => format!(
            "Rewrite the following text in a {tone_label} tone. Preserve the meaning."
        ),
        WritingToolsCommand::Summary => format!(
            "Summarize the following text in a {tone_label} tone."
        ),
        WritingToolsCommand::KeyPoints => {
            "Extract the key points as a short bullet list.".to_string()
        }
        WritingToolsCommand::List => {
            "Convert the content into a list format that is easy to scan.".to_string()
        }
        WritingToolsCommand::Table => {
            "Convert the content into a simple table.".to_string()
        }
        WritingToolsCommand::Compose => format!(
            "Compose a new version in a {tone_label} tone based on the guidance and content."
        ),
    };

    let trimmed_prompt = user_prompt.trim();
    if !trimmed_prompt.is_empty() {
        prompt.push_str(" Instruction: ");
        prompt.push_str(trimmed_prompt);
    }

    let trimmed_text = source_text.trim();
    if !trimmed_text.is_empty() {
        prompt.push_str(" Text: ");
        prompt.push_str(trimmed_text);
    }

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

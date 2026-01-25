use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_router::Navigator;
use learn_core::model::{CardId, ContentDraft, DeckId, TagName};

use crate::routes::Route;
use crate::vm::{build_card_list_item, sanitize_html, strip_html_tags};
use crate::views::ViewError;

use super::super::state::{
    DeleteState, DuplicateCheckState, EditorServices, EditorState, SaveMenuState, SaveRequest,
    SaveState,
};
use super::super::utils::tag_names_from_strings;

struct SavePayload {
    deck_id: DeckId,
    editing_id: Option<CardId>,
    prompt_html: String,
    answer_html: String,
    tag_names: Vec<TagName>,
    practice: bool,
    skip_duplicate_check: bool,
}

fn is_blank_content(prompt_html: &str, answer_html: &str) -> bool {
    let prompt_plain = strip_html_tags(prompt_html);
    let answer_plain = strip_html_tags(answer_html);
    prompt_plain.trim().is_empty() || answer_plain.trim().is_empty()
}

fn build_save_payload(state: &EditorState, request: SaveRequest) -> Option<SavePayload> {
    let save_state = state.save_state;
    let duplicate_check_state = state.duplicate_check_state;
    if save_state() == SaveState::Saving || duplicate_check_state() == DuplicateCheckState::Checking
    {
        return None;
    }

    if !(state.has_unsaved_changes)() {
        return None;
    }

    let raw_prompt = state.prompt_text.read().to_string();
    let raw_answer = state.answer_text.read().to_string();
    let prompt_html = sanitize_html(&raw_prompt);
    let answer_html = sanitize_html(&raw_answer);
    if is_blank_content(&prompt_html, &answer_html) {
        let mut show_validation = state.show_validation;
        show_validation.set(true);
        return None;
    }

    let is_create_mode = state.is_create_mode;
    let selected_card_id = state.selected_card_id;
    let editing_id = if is_create_mode() {
        None
    } else {
        selected_card_id()
    };
    if !is_create_mode() && editing_id.is_none() {
        return None;
    }

    Some(SavePayload {
        deck_id: *state.selected_deck.read(),
        editing_id,
        prompt_html,
        answer_html,
        tag_names: tag_names_from_strings(&state.card_tags.read()),
        practice: request.practice,
        skip_duplicate_check: request.skip_duplicate_check,
    })
}

async fn check_duplicate(
    card_service: &services::CardService,
    payload: &SavePayload,
    mut duplicate_check_state: Signal<DuplicateCheckState>,
    mut show_duplicate_modal: Signal<bool>,
    mut pending_duplicate_practice: Signal<bool>,
) -> Result<bool, ViewError> {
    duplicate_check_state.set(DuplicateCheckState::Checking);
    match card_service
        .prompt_exists(payload.deck_id, &payload.prompt_html, payload.editing_id)
        .await
    {
        Ok(true) => {
            duplicate_check_state.set(DuplicateCheckState::Idle);
            show_duplicate_modal.set(true);
            pending_duplicate_practice.set(payload.practice);
            Ok(true)
        }
        Ok(false) => {
            duplicate_check_state.set(DuplicateCheckState::Idle);
            Ok(false)
        }
        Err(_) => {
            duplicate_check_state.set(DuplicateCheckState::Error(ViewError::Unknown));
            Err(ViewError::Unknown)
        }
    }
}

async fn persist_card(
    card_service: &services::CardService,
    payload: &SavePayload,
) -> Result<Option<CardId>, ViewError> {
    let result = match payload.editing_id {
        None => card_service
            .create_card_with_tags(
                payload.deck_id,
                ContentDraft::new(payload.prompt_html.clone(), None),
                ContentDraft::new(payload.answer_html.clone(), None),
                &payload.tag_names,
            )
            .await
            .map(Some),
        Some(card_id) => card_service
            .update_card_content_with_tags(
                payload.deck_id,
                card_id,
                ContentDraft::new(payload.prompt_html.clone(), None),
                ContentDraft::new(payload.answer_html.clone(), None),
                &payload.tag_names,
            )
            .await
            .map(|()| Some(card_id)),
    };
    result.map_err(|_| ViewError::Unknown)
}

fn apply_save_success(
    state: &EditorState,
    navigator: Navigator,
    payload: &SavePayload,
    card_id: Option<CardId>,
) {
    let mut save_state = state.save_state;
    let mut delete_state = state.delete_state;
    let mut show_delete_modal = state.show_delete_modal;
    let mut show_validation = state.show_validation;
    let mut save_menu_state = state.save_menu_state;
    let mut tag_input = state.tag_input;
    let mut cards_resource = state.cards_resource;
    let mut deck_tags_resource = state.deck_tags_resource;
    let mut daily_limit_resource = state.daily_limit_resource;
    let mut card_tags_resource = state.card_tags_resource;
    let mut selected_card_id = state.selected_card_id;
    let mut last_selected_card = state.last_selected_card;
    let mut last_selected_tags = state.last_selected_tags;
    let mut card_tags = state.card_tags;
    let mut focus_prompt = state.focus_prompt;

    save_state.set(SaveState::Success);
    delete_state.set(DeleteState::Idle);
    show_delete_modal.set(false);
    show_validation.set(false);
    save_menu_state.set(SaveMenuState::Closed);
    tag_input.set(String::new());
    cards_resource.restart();
    deck_tags_resource.restart();
    daily_limit_resource.restart();
    card_tags_resource.restart();

    match ((state.is_create_mode)(), payload.practice) {
        (true, true) => {
            navigator.push(Route::Session {
                deck_id: payload.deck_id.value(),
            });
        }
        (true, false) => {
            state.clear_editor_fields.borrow_mut()();
            card_tags.set(Vec::new());
            focus_prompt.set(true);
        }
        (false, _) => {
            if let Some(card_id) = card_id {
                selected_card_id.set(Some(card_id));
                state
                    .set_editor_fields
                    .borrow_mut()(payload.prompt_html.clone(), payload.answer_html.clone());
                last_selected_card.set(Some(build_card_list_item(
                    card_id,
                    &payload.prompt_html,
                    &payload.answer_html,
                )));
                last_selected_tags.set(card_tags.read().clone());
                focus_prompt.set(true);
            }
        }
    }
}

async fn run_save(
    payload: SavePayload,
    state: EditorState,
    card_service: Arc<services::CardService>,
    navigator: Navigator,
) {
    state.reset_duplicate_state.borrow_mut()();
    let mut show_unsaved_modal = state.show_unsaved_modal;
    let mut pending_action = state.pending_action;
    let mut save_menu_state = state.save_menu_state;
    show_unsaved_modal.set(false);
    pending_action.set(None);
    save_menu_state.set(SaveMenuState::Closed);

    if !payload.skip_duplicate_check {
        let duplicate_result = check_duplicate(
            &card_service,
            &payload,
            state.duplicate_check_state,
            state.show_duplicate_modal,
            state.pending_duplicate_practice,
        )
        .await;
        if matches!(duplicate_result, Ok(true) | Err(_)) {
            return;
        }
    }

    let mut save_state = state.save_state;
    save_state.set(SaveState::Saving);
    match persist_card(&card_service, &payload).await {
        Ok(card_id) => apply_save_success(&state, navigator, &payload, card_id),
        Err(err) => save_state.set(SaveState::Error(err)),
    }
}

pub(super) fn build_save_action(
    state: &EditorState,
    services: &EditorServices,
    navigator: Navigator,
) -> Callback<SaveRequest> {
    let state = state.clone();
    let card_service = services.card_service.clone();
    use_callback(move |request: SaveRequest| {
        let Some(payload) = build_save_payload(&state, request) else {
            return;
        };
        let state = state.clone();
        let card_service = card_service.clone();
        let navigator = navigator;
        spawn(async move {
            run_save(payload, state, card_service, navigator).await;
        });
    })
}

#[cfg(test)]
mod tests {
    use super::is_blank_content;

    #[test]
    fn blank_content_detects_empty_html() {
        assert!(is_blank_content("<p> </p>", "<div>\n</div>"));
        assert!(is_blank_content("", "<p>Answer</p>"));
        assert!(is_blank_content("<p>Prompt</p>", ""));
    }

    #[test]
    fn blank_content_allows_text() {
        assert!(!is_blank_content("<p>Prompt</p>", "<p>Answer</p>"));
        assert!(!is_blank_content("Prompt", "<div>Answer</div>"));
    }
}

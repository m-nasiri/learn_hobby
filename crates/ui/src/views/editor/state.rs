use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use dioxus::prelude::*;
use learn_core::model::{CardId, DeckId};
use services::{CardListFilter, CardListSort, CardService, DeckService};

use crate::vm::{
    CardListItemVm, DailyLimitVm, MarkdownField, map_card_list_items, map_deck_options,
    strip_html_tags,
};
use crate::views::{ViewError, ViewState, view_state_from_resource};

use super::utils::{tag_filter_key, tag_names_from_strings, tags_equal};

type CardTagsResource = Resource<Result<(Option<CardId>, Vec<String>), ViewError>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveState {
    Idle,
    Saving,
    Success,
    Error(ViewError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteState {
    Idle,
    Deleting,
    Success,
    Error(ViewError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetDeckState {
    Idle,
    Resetting,
    Success,
    Error(ViewError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveMenuState {
    Closed,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WritingToolsMenuState {
    Closed,
    Open(MarkdownField),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WritingToolsResultStatus {
    Idle,
    Loading,
    Ready,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WritingToolsTone {
    Clear,
    Simple,
    Formal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WritingToolsCommand {
    ImproveWording,
    Simplify,
    Concise,
    Summary,
    KeyPoints,
    List,
    TurnIntoQuestion,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WritingToolsRequest {
    pub field: MarkdownField,
    pub command: WritingToolsCommand,
    pub tone: WritingToolsTone,
    pub user_prompt: String,
    pub source_text: String,
    pub request_prompt: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuplicateCheckState {
    Idle,
    Checking,
    Error(ViewError),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PendingAction {
    SelectCard(CardListItemVm),
    SelectDeck(DeckId),
    NewCard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveRequest {
    pub practice: bool,
    pub skip_duplicate_check: bool,
}

impl SaveRequest {
    pub const fn new(practice: bool) -> Self {
        Self {
            practice,
            skip_duplicate_check: false,
        }
    }

    pub const fn force(practice: bool) -> Self {
        Self {
            practice,
            skip_duplicate_check: true,
        }
    }
}

pub type VoidAction = Rc<RefCell<dyn FnMut()>>;
pub type SetFieldsAction = Rc<RefCell<dyn FnMut(String, String)>>;

#[derive(Clone)]
pub struct EditorServices {
    pub deck_service: Arc<DeckService>,
    pub card_service: Arc<CardService>,
}

#[derive(Clone)]
pub struct EditorState {
    pub selected_deck: Signal<DeckId>,
    pub save_state: Signal<SaveState>,
    pub delete_state: Signal<DeleteState>,
    pub show_delete_modal: Signal<bool>,
    pub show_validation: Signal<bool>,
    pub focus_prompt: Signal<bool>,
    pub show_unsaved_modal: Signal<bool>,
    pub pending_action: Signal<Option<PendingAction>>,
    pub save_menu_state: Signal<SaveMenuState>,
    pub writing_tools_menu_state: Signal<WritingToolsMenuState>,
    pub writing_tools_prompt: Signal<String>,
    pub writing_tools_tone: Signal<WritingToolsTone>,
    pub writing_tools_last_command: Signal<Option<WritingToolsCommand>>,
    pub writing_tools_request: Signal<Option<WritingToolsRequest>>,
    pub writing_tools_result_status: Signal<WritingToolsResultStatus>,
    pub writing_tools_result_target: Signal<Option<MarkdownField>>,
    pub writing_tools_result_title: Signal<String>,
    pub writing_tools_result_body: Signal<String>,
    pub show_new_deck: Signal<bool>,
    pub new_deck_name: Signal<String>,
    pub new_deck_state: Signal<SaveState>,
    pub show_deck_menu: Signal<bool>,
    pub show_deck_actions: Signal<bool>,
    pub is_renaming_deck: Signal<bool>,
    pub rename_deck_name: Signal<String>,
    pub rename_deck_state: Signal<SaveState>,
    pub rename_deck_error: Signal<Option<String>>,
    pub selected_card_id: Signal<Option<CardId>>,
    pub last_selected_card: Signal<Option<CardListItemVm>>,
    pub is_create_mode: Signal<bool>,
    pub search_query: Signal<String>,
    pub sort_mode: Signal<CardListSort>,
    pub selected_tag_filters: Signal<Vec<String>>,
    pub card_tags: Signal<Vec<String>>,
    pub last_selected_tags: Signal<Vec<String>>,
    pub tag_input: Signal<String>,
    pub last_focus_field: Signal<MarkdownField>,
    pub duplicate_check_state: Signal<DuplicateCheckState>,
    pub show_duplicate_modal: Signal<bool>,
    pub pending_duplicate_practice: Signal<bool>,
    pub show_reset_deck_modal: Signal<bool>,
    pub reset_deck_state: Signal<ResetDeckState>,
    pub prompt_text: Signal<String>,
    pub answer_text: Signal<String>,
    pub prompt_render_html: Signal<String>,
    pub answer_render_html: Signal<String>,
    pub decks_resource: Resource<Result<Vec<crate::vm::DeckOptionVm>, ViewError>>,
    pub cards_resource: Resource<Result<Vec<CardListItemVm>, ViewError>>,
    pub deck_tags_resource: Resource<Result<Vec<String>, ViewError>>,
    pub daily_limit_resource: Resource<Result<DailyLimitVm, ViewError>>,
    pub card_tags_resource: CardTagsResource,
    pub clear_editor_fields: VoidAction,
    pub set_editor_fields: SetFieldsAction,
    pub reset_duplicate_state: VoidAction,
    pub has_unsaved_changes: Rc<dyn Fn() -> bool>,
}

#[allow(clippy::too_many_lines)]
pub fn use_editor_state(deck_id: DeckId, services: &EditorServices) -> EditorState {
    let selected_deck = use_signal(|| deck_id);
    let save_state = use_signal(|| SaveState::Idle);
    let delete_state = use_signal(|| DeleteState::Idle);
    let show_delete_modal = use_signal(|| false);
    let show_validation = use_signal(|| false);
    let focus_prompt = use_signal(|| false);
    let show_unsaved_modal = use_signal(|| false);
    let pending_action = use_signal(|| None::<PendingAction>);
    let save_menu_state = use_signal(|| SaveMenuState::Closed);
    let writing_tools_menu_state = use_signal(|| WritingToolsMenuState::Closed);
    let writing_tools_prompt = use_signal(String::new);
    let writing_tools_tone = use_signal(|| WritingToolsTone::Clear);
    let writing_tools_last_command = use_signal(|| None::<WritingToolsCommand>);
    let writing_tools_request = use_signal(|| None::<WritingToolsRequest>);
    let writing_tools_result_status = use_signal(|| WritingToolsResultStatus::Idle);
    let writing_tools_result_target = use_signal(|| None::<MarkdownField>);
    let writing_tools_result_title = use_signal(String::new);
    let writing_tools_result_body = use_signal(String::new);
    let show_new_deck = use_signal(|| false);
    let new_deck_name = use_signal(String::new);
    let new_deck_state = use_signal(|| SaveState::Idle);
    let show_deck_menu = use_signal(|| false);
    let show_deck_actions = use_signal(|| false);
    let is_renaming_deck = use_signal(|| false);
    let rename_deck_name = use_signal(String::new);
    let rename_deck_state = use_signal(|| SaveState::Idle);
    let rename_deck_error = use_signal(|| None::<String>);
    let selected_card_id = use_signal(|| None::<CardId>);
    let last_selected_card = use_signal(|| None::<CardListItemVm>);
    let is_create_mode = use_signal(|| false);
    let search_query = use_signal(String::new);
    let sort_mode = use_signal(|| CardListSort::Recent);
    let selected_tag_filters = use_signal(Vec::new);
    let card_tags = use_signal(Vec::new);
    let last_selected_tags = use_signal(Vec::new);
    let tag_input = use_signal(String::new);
    let last_focus_field = use_signal(|| MarkdownField::Front);
    let duplicate_check_state = use_signal(|| DuplicateCheckState::Idle);
    let show_duplicate_modal = use_signal(|| false);
    let pending_duplicate_practice = use_signal(|| false);
    let show_reset_deck_modal = use_signal(|| false);
    let reset_deck_state = use_signal(|| ResetDeckState::Idle);

    let deck_service_for_resource = services.deck_service.clone();
    let decks_resource = use_resource(move || {
        let deck_service = deck_service_for_resource.clone();
        async move {
            let decks = deck_service
                .list_decks(64)
                .await
                .map_err(|_| ViewError::Unknown)?;
            Ok::<_, ViewError>(map_deck_options(&decks))
        }
    });

    let card_service_for_list = services.card_service.clone();
    let cards_resource = use_resource(move || {
        let card_service = card_service_for_list.clone();
        let deck_id = *selected_deck.read();
        let sort = sort_mode();
        let filter = CardListFilter::All;
        let tag_filters = selected_tag_filters();
        let tag_names = tag_names_from_strings(&tag_filters);
        async move {
            let cards = card_service
                .list_cards_filtered(deck_id, 100, sort, filter, &tag_names)
                .await
                .map_err(|_| ViewError::Unknown)?;
            Ok::<_, ViewError>(map_card_list_items(&cards))
        }
    });

    let mut last_cards_query = use_signal(|| (deck_id, CardListSort::Recent, String::new()));
    use_effect(move || {
        let current = (
            *selected_deck.read(),
            sort_mode(),
            tag_filter_key(&selected_tag_filters()),
        );
        if last_cards_query() != current {
            last_cards_query.set(current);
            let mut cards_resource = cards_resource;
            cards_resource.restart();
        }
    });

    let card_service_for_deck_tags = services.card_service.clone();
    let deck_tags_resource = use_resource(move || {
        let card_service = card_service_for_deck_tags.clone();
        let deck_id = *selected_deck.read();
        async move {
            let tags = card_service
                .list_tags_for_deck(deck_id)
                .await
                .map_err(|_| ViewError::Unknown)?;
            let tags: Vec<String> = tags
                .into_iter()
                .map(|tag| tag.name().as_str().to_string())
                .collect();
            Ok::<Vec<String>, ViewError>(tags)
        }
    });

    let card_service_for_daily = services.card_service.clone();
    let deck_service_for_daily = services.deck_service.clone();
    let daily_limit_resource = use_resource(move || {
        let card_service = card_service_for_daily.clone();
        let deck_service = deck_service_for_daily.clone();
        let deck_id = *selected_deck.read();
        async move {
            let deck = deck_service
                .get_deck(deck_id)
                .await
                .map_err(|_| ViewError::Unknown)?
                .ok_or(ViewError::Unknown)?;
            let created_today = card_service
                .new_cards_created_today(deck_id)
                .await
                .map_err(|_| ViewError::Unknown)?;
            Ok::<DailyLimitVm, ViewError>(DailyLimitVm {
                limit: deck.settings().new_cards_per_day(),
                created_today,
            })
        }
    });

    let mut last_daily_limit_deck = use_signal(|| deck_id);
    use_effect(move || {
        let current = *selected_deck.read();
        if last_daily_limit_deck() != current {
            last_daily_limit_deck.set(current);
            let mut daily_limit_resource = daily_limit_resource;
            daily_limit_resource.restart();
        }
    });

    let card_service_for_card_tags = services.card_service.clone();
    let card_tags_resource = use_resource(move || {
        let card_service = card_service_for_card_tags.clone();
        let deck_id = *selected_deck.read();
        let card_id = selected_card_id();
        async move {
            let tags = if let Some(card_id) = card_id {
                card_service
                    .list_tags_for_card(deck_id, card_id)
                    .await
                    .map_err(|_| ViewError::Unknown)?
                    .into_iter()
                    .map(|tag| tag.name().as_str().to_string())
                    .collect()
            } else {
                Vec::new()
            };
            Ok::<_, ViewError>((card_id, tags))
        }
    });

    let mut last_deck_for_tags = use_signal(|| deck_id);
    use_effect(move || {
        let current = *selected_deck.read();
        if last_deck_for_tags() != current {
            last_deck_for_tags.set(current);
            let mut deck_tags_resource = deck_tags_resource;
            deck_tags_resource.restart();
        }
    });

    let mut last_card_tags_key = use_signal(|| (deck_id, None::<CardId>));
    use_effect(move || {
        let current = (*selected_deck.read(), selected_card_id());
        if last_card_tags_key() != current {
            last_card_tags_key.set(current);
            let mut card_tags_resource = card_tags_resource;
            card_tags_resource.restart();
        }
    });

    let mut selected_tag_filters_for_effect = selected_tag_filters;
    use_effect(move || {
        let state = view_state_from_resource(&deck_tags_resource);
        if let ViewState::Ready(tags) = &state {
            let current = selected_tag_filters_for_effect();
            let filtered: Vec<String> =
                current.iter().filter(|tag| tags.contains(tag)).cloned().collect();
            if filtered.len() != current.len() {
                selected_tag_filters_for_effect.set(filtered);
            }
        }
    });

    let mut card_tags_for_effect = card_tags;
    let mut last_selected_tags_for_effect = last_selected_tags;
    let mut tag_input_for_effect = tag_input;
    use_effect(move || {
        let state = view_state_from_resource(&card_tags_resource);
        if let ViewState::Ready((card_id, tags)) = &state
            && *card_id == selected_card_id()
            && !is_create_mode()
        {
            card_tags_for_effect.set(tags.clone());
            last_selected_tags_for_effect.set(tags.clone());
            tag_input_for_effect.set(String::new());
        }
    });

    let prompt_text = use_signal(String::new);
    let answer_text = use_signal(String::new);
    let prompt_render_html = use_signal(String::new);
    let answer_render_html = use_signal(String::new);

    let clear_editor_fields = {
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut prompt_render_html = prompt_render_html;
        let mut answer_render_html = answer_render_html;
        Rc::new(RefCell::new(move || {
            prompt_text.set(String::new());
            answer_text.set(String::new());
            prompt_render_html.set(String::new());
            answer_render_html.set(String::new());
        }))
    };

    let set_editor_fields = {
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut prompt_render_html = prompt_render_html;
        let mut answer_render_html = answer_render_html;
        Rc::new(RefCell::new(move |prompt_html: String, answer_html: String| {
            let prompt_clone = prompt_html.clone();
            let answer_clone = answer_html.clone();
            prompt_text.set(prompt_clone);
            answer_text.set(answer_clone);
            prompt_render_html.set(prompt_html);
            answer_render_html.set(answer_html);
        }))
    };

    let reset_duplicate_state = {
        let mut duplicate_check_state = duplicate_check_state;
        let mut show_duplicate_modal = show_duplicate_modal;
        let mut pending_duplicate_practice = pending_duplicate_practice;
        Rc::new(RefCell::new(move || {
            duplicate_check_state.set(DuplicateCheckState::Idle);
            show_duplicate_modal.set(false);
            pending_duplicate_practice.set(false);
        }))
    };

    let has_unsaved_changes = Rc::new(move || {
            if !(is_create_mode() || selected_card_id().is_some()) {
                return false;
            }
            let prompt_html = prompt_text.read().to_string();
            let answer_html = answer_text.read().to_string();
            let prompt_plain = strip_html_tags(&prompt_html);
            let answer_plain = strip_html_tags(&answer_html);
            let tags = card_tags.read().clone();
            if is_create_mode() {
                return !prompt_plain.trim().is_empty()
                    || !answer_plain.trim().is_empty()
                    || !tags.is_empty();
            }
            if let Some(original) = last_selected_card() {
                prompt_html.trim() != original.prompt_html.trim()
                    || answer_html.trim() != original.answer_html.trim()
                    || !tags_equal(&tags, &last_selected_tags())
            } else {
                !prompt_plain.trim().is_empty()
                    || !answer_plain.trim().is_empty()
                    || !tags.is_empty()
            }
    });

    EditorState {
        selected_deck,
        save_state,
        delete_state,
        show_delete_modal,
        show_validation,
        focus_prompt,
        show_unsaved_modal,
        pending_action,
        save_menu_state,
        writing_tools_menu_state,
        writing_tools_prompt,
        writing_tools_tone,
        writing_tools_last_command,
        writing_tools_request,
        writing_tools_result_status,
        writing_tools_result_target,
        writing_tools_result_title,
        writing_tools_result_body,
        show_new_deck,
        new_deck_name,
        new_deck_state,
        show_deck_menu,
        show_deck_actions,
        is_renaming_deck,
        rename_deck_name,
        rename_deck_state,
        rename_deck_error,
        selected_card_id,
        last_selected_card,
        is_create_mode,
        search_query,
        sort_mode,
        selected_tag_filters,
        card_tags,
        last_selected_tags,
        tag_input,
        last_focus_field,
        duplicate_check_state,
        show_duplicate_modal,
        pending_duplicate_practice,
        show_reset_deck_modal,
        reset_deck_state,
        prompt_text,
        answer_text,
        prompt_render_html,
        answer_render_html,
        decks_resource,
        cards_resource,
        deck_tags_resource,
        daily_limit_resource,
        card_tags_resource,
        clear_editor_fields,
        set_editor_fields,
        reset_duplicate_state,
        has_unsaved_changes,
    }
}

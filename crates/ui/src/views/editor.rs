use std::time::Duration;

use dioxus::prelude::*;
use dioxus_router::use_navigator;
use learn_core::model::{CardId, ContentDraft, DeckSettings};

use crate::context::AppContext;
use crate::routes::Route;
use crate::vm::{CardListItemVm, build_card_list_item, map_card_list_items, map_deck_options};
use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveState {
    Idle,
    Saving,
    Success,
    Error(ViewError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeleteState {
    Idle,
    Deleting,
    Success,
    Error(ViewError),
}

#[component]
pub fn EditorView() -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let deck_id = ctx.current_deck_id();
    let deck_service = ctx.deck_service();
    let deck_service_for_resource = deck_service.clone();
    let deck_service_for_create = deck_service.clone();
    let deck_service_for_rename = deck_service.clone();
    let card_service = ctx.card_service();
    let card_service_for_list = card_service.clone();
    let card_service_for_save = card_service.clone();
    let card_service_for_delete = card_service.clone();
    let mut selected_deck = use_signal(|| deck_id);
    let mut save_state = use_signal(|| SaveState::Idle);
    let mut delete_state = use_signal(|| DeleteState::Idle);
    let mut show_delete_modal = use_signal(|| false);
    let mut show_validation = use_signal(|| false);
    let mut show_new_deck = use_signal(|| false);
    let mut new_deck_name = use_signal(String::new);
    let mut new_deck_state = use_signal(|| SaveState::Idle);
    let mut show_deck_menu = use_signal(|| false);
    let mut is_renaming_deck = use_signal(|| false);
    let mut rename_deck_name = use_signal(String::new);
    let mut rename_deck_state = use_signal(|| SaveState::Idle);
    let mut rename_deck_error = use_signal(|| None::<String>);
    let mut selected_card_id = use_signal(|| None::<CardId>);
    let mut last_selected_card = use_signal(|| None::<CardListItemVm>);
    let mut is_create_mode = use_signal(|| false);

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
    let decks_state = view_state_from_resource(&decks_resource);

    let cards_resource = use_resource(move || {
        let card_service = card_service_for_list.clone();
        let deck_id = *selected_deck.read();
        async move {
            let cards = card_service
                .list_cards(deck_id, 100)
                .await
                .map_err(|_| ViewError::Unknown)?;
            Ok::<_, ViewError>(map_card_list_items(&cards))
        }
    });
    let cards_state = view_state_from_resource(&cards_resource);

    let mut last_deck_for_cards = use_signal(|| deck_id);
    use_effect(move || {
        let current = *selected_deck.read();
        if last_deck_for_cards() != current {
            last_deck_for_cards.set(current);
            let mut cards_resource = cards_resource;
            cards_resource.restart();
        }
    });

    // UI-only state for now (service wiring comes next step).
    let mut prompt_text = use_signal(String::new);
    let mut answer_text = use_signal(String::new);

    let can_edit = is_create_mode() || selected_card_id().is_some();
    let can_submit = can_edit
        && save_state() != SaveState::Saving
        && delete_state() != DeleteState::Deleting;
    let save_action = use_callback(move |practice: bool| {
        let card_service = card_service_for_save.clone();
        let navigator = navigator;
        let mut save_state = save_state;
        let mut delete_state = delete_state;
        let mut show_delete_modal = show_delete_modal;
        let mut show_validation = show_validation;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut cards_resource = cards_resource;
        let mut selected_card_id = selected_card_id;
        let mut last_selected_card = last_selected_card;
        let is_create_mode = is_create_mode;
        let deck_id = *selected_deck.read();

        let prompt = prompt_text.read().trim().to_owned();
        let answer = answer_text.read().trim().to_owned();

        if save_state() == SaveState::Saving {
            return;
        }

        if prompt.is_empty() || answer.is_empty() {
            show_validation.set(true);
            return;
        }

        let editing_id = if is_create_mode() {
            None
        } else {
            selected_card_id()
        };
        if !is_create_mode() && editing_id.is_none() {
            return;
        }

        spawn(async move {
            save_state.set(SaveState::Saving);
            delete_state.set(DeleteState::Idle);
            show_delete_modal.set(false);
            show_validation.set(false);
            let result = match editing_id {
                Some(card_id) => {
                    card_service
                        .update_card_content(
                            deck_id,
                            card_id,
                            ContentDraft::text_only(prompt.clone()),
                            ContentDraft::text_only(answer.clone()),
                        )
                        .await
                        .map(|()| Some(card_id))
                }
                None => card_service
                    .create_card(
                        deck_id,
                        ContentDraft::text_only(prompt.clone()),
                        ContentDraft::text_only(answer.clone()),
                    )
                    .await
                    .map(Some),
            };

            match result {
                Ok(card_id) => {
                    save_state.set(SaveState::Success);
                    delete_state.set(DeleteState::Idle);
                    show_delete_modal.set(false);
                    show_validation.set(false);
                    cards_resource.restart();
                    match (is_create_mode(), practice) {
                        (true, true) => {
                            navigator.push(Route::Session {
                                deck_id: deck_id.value(),
                            });
                        }
                        (true, false) => {
                            prompt_text.set(String::new());
                            answer_text.set(String::new());
                        }
                        (false, _) => {
                            if let Some(card_id) = card_id {
                                selected_card_id.set(Some(card_id));
                                last_selected_card
                                    .set(Some(build_card_list_item(card_id, &prompt, &answer)));
                            }
                        }
                    }
                }
                Err(_) => {
                    save_state.set(SaveState::Error(ViewError::Unknown));
                }
            }
        });
    });

    let create_deck_action = use_callback(move |()| {
        let deck_service = deck_service_for_create.clone();
        let mut show_new_deck = show_new_deck;
        let mut new_deck_state = new_deck_state;
        let mut new_deck_name = new_deck_name;
        let mut selected_deck = selected_deck;
        let mut decks_resource = decks_resource;
        let mut cards_resource = cards_resource;
        let mut selected_card_id = selected_card_id;
        let mut last_selected_card = last_selected_card;
        let mut is_create_mode = is_create_mode;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut show_deck_menu = show_deck_menu;
        let mut is_renaming_deck = is_renaming_deck;
        let mut rename_deck_state = rename_deck_state;
        let mut rename_deck_error = rename_deck_error;
        let mut delete_state = delete_state;
        let mut show_validation = show_validation;
        let mut show_delete_modal = show_delete_modal;

        let name = new_deck_name.read().trim().to_owned();
        if name.is_empty() || new_deck_state() == SaveState::Saving {
            return;
        }

        spawn(async move {
            new_deck_state.set(SaveState::Saving);
            let result = deck_service
                .create_deck(name.clone(), None, DeckSettings::default_for_adhd())
                .await;

            match result {
                Ok(deck_id) => {
                    selected_deck.set(deck_id);
                    new_deck_name.set(String::new());
                    show_new_deck.set(false);
                    show_deck_menu.set(false);
                    is_renaming_deck.set(false);
                    rename_deck_state.set(SaveState::Idle);
                    rename_deck_error.set(None);
                    delete_state.set(DeleteState::Idle);
                    show_delete_modal.set(false);
                    show_validation.set(false);
                    new_deck_state.set(SaveState::Success);
                    decks_resource.restart();
                    cards_resource.restart();
                    selected_card_id.set(None);
                    last_selected_card.set(None);
                    is_create_mode.set(true);
                    prompt_text.set(String::new());
                    answer_text.set(String::new());
                }
                Err(_) => {
                    new_deck_state.set(SaveState::Error(ViewError::Unknown));
                }
            }
        });
    });

    let cancel_rename_action = use_callback(move |()| {
        let mut is_renaming_deck = is_renaming_deck;
        let mut rename_deck_state = rename_deck_state;
        let mut rename_deck_error = rename_deck_error;
        let mut rename_deck_name = rename_deck_name;

        is_renaming_deck.set(false);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
        rename_deck_name.set(String::new());
    });

    let commit_rename_action = use_callback(move |()| {
        let deck_service = deck_service_for_rename.clone();
        let mut rename_deck_state = rename_deck_state;
        let mut rename_deck_error = rename_deck_error;
        let mut is_renaming_deck = is_renaming_deck;
        let mut decks_resource = decks_resource;
        let deck_id = *selected_deck.read();
        let name = rename_deck_name.read().trim().to_owned();

        if name.is_empty() || rename_deck_state() == SaveState::Saving {
            rename_deck_error.set(Some("Name cannot be empty.".to_string()));
            return;
        }

        spawn(async move {
            rename_deck_state.set(SaveState::Saving);
            rename_deck_error.set(None);

            if deck_service.rename_deck(deck_id, name).await.is_ok() {
                rename_deck_state.set(SaveState::Success);
                is_renaming_deck.set(false);
                decks_resource.restart();
            } else {
                rename_deck_state.set(SaveState::Error(ViewError::Unknown));
                let message = "Rename failed. Please try again.".to_string();
                rename_deck_error.set(Some(message.clone()));
                let mut rename_deck_error = rename_deck_error;
                spawn(async move {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    if rename_deck_error.read().as_ref() == Some(&message) {
                        rename_deck_error.set(None);
                    }
                });
            }
        });
    });

    let begin_rename_action = use_callback(move |label: String| {
        let mut is_renaming_deck = is_renaming_deck;
        let mut rename_deck_name = rename_deck_name;
        let mut rename_deck_state = rename_deck_state;
        let mut rename_deck_error = rename_deck_error;
        let mut show_deck_menu = show_deck_menu;
        let mut show_new_deck = show_new_deck;
        let mut new_deck_state = new_deck_state;

        rename_deck_name.set(label);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
        show_deck_menu.set(false);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        is_renaming_deck.set(true);
    });

    let select_card_action = use_callback(move |item: CardListItemVm| {
        let mut selected_card_id = selected_card_id;
        let mut last_selected_card = last_selected_card;
        let mut is_create_mode = is_create_mode;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut save_state = save_state;
        let mut show_new_deck = show_new_deck;
        let mut new_deck_state = new_deck_state;
        let mut show_deck_menu = show_deck_menu;
        let mut is_renaming_deck = is_renaming_deck;
        let mut rename_deck_state = rename_deck_state;
        let mut rename_deck_error = rename_deck_error;
        let mut delete_state = delete_state;

        selected_card_id.set(Some(item.id));
        last_selected_card.set(Some(item.clone()));
        is_create_mode.set(false);
        prompt_text.set(item.prompt);
        answer_text.set(item.answer);
        save_state.set(SaveState::Idle);
        delete_state.set(DeleteState::Idle);
        show_validation.set(false);
        show_delete_modal.set(false);
        show_validation.set(false);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        show_deck_menu.set(false);
        is_renaming_deck.set(false);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
    });

    let new_card_action = use_callback(move |()| {
        let mut selected_card_id = selected_card_id;
        let mut is_create_mode = is_create_mode;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut save_state = save_state;
        let mut show_new_deck = show_new_deck;
        let mut new_deck_state = new_deck_state;
        let mut show_deck_menu = show_deck_menu;
        let mut new_deck_name = new_deck_name;
        let mut is_renaming_deck = is_renaming_deck;
        let mut rename_deck_state = rename_deck_state;
        let mut rename_deck_error = rename_deck_error;
        let mut delete_state = delete_state;

        selected_card_id.set(None);
        is_create_mode.set(true);
        prompt_text.set(String::new());
        answer_text.set(String::new());
        save_state.set(SaveState::Idle);
        delete_state.set(DeleteState::Idle);
        show_delete_modal.set(false);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        new_deck_name.set(String::new());
        show_deck_menu.set(false);
        is_renaming_deck.set(false);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
    });

    let open_delete_modal_action = use_callback(move |()| {
        let mut show_delete_modal = show_delete_modal;
        let mut show_deck_menu = show_deck_menu;
        let mut is_renaming_deck = is_renaming_deck;
        let mut rename_deck_state = rename_deck_state;
        let mut rename_deck_error = rename_deck_error;
        let selected_card_id = selected_card_id();
        if selected_card_id.is_some() {
            show_deck_menu.set(false);
            is_renaming_deck.set(false);
            rename_deck_state.set(SaveState::Idle);
            rename_deck_error.set(None);
            show_delete_modal.set(true);
        }
    });

    let close_delete_modal_action = use_callback(move |()| {
        let mut show_delete_modal = show_delete_modal;
        show_delete_modal.set(false);
    });

    let delete_action = use_callback(move |()| {
        let card_service = card_service_for_delete.clone();
        let mut delete_state = delete_state;
        let mut save_state = save_state;
        let mut cards_resource = cards_resource;
        let mut selected_card_id = selected_card_id;
        let mut last_selected_card = last_selected_card;
        let mut is_create_mode = is_create_mode;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut show_delete_modal = show_delete_modal;
        let deck_id = *selected_deck.read();
        let Some(card_id) = selected_card_id() else {
            return;
        };

        if delete_state() == DeleteState::Deleting {
            return;
        }

        spawn(async move {
            delete_state.set(DeleteState::Deleting);
            save_state.set(SaveState::Idle);
            show_delete_modal.set(false);
            let result = card_service.delete_card(deck_id, card_id).await;
            match result {
                Ok(()) => {
                    delete_state.set(DeleteState::Success);
                    selected_card_id.set(None);
                    last_selected_card.set(None);
                    is_create_mode.set(false);
                    prompt_text.set(String::new());
                    answer_text.set(String::new());
                    cards_resource.restart();
                    let mut delete_state = delete_state;
                    spawn(async move {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        if delete_state() == DeleteState::Success {
                            delete_state.set(DeleteState::Idle);
                        }
                    });
                }
                Err(_) => {
                    delete_state.set(DeleteState::Error(ViewError::Unknown));
                }
            }
        });
    });

    let cancel_new_action = use_callback(move |()| {
        let mut selected_card_id = selected_card_id;
        let last_selected_card = last_selected_card;
        let mut is_create_mode = is_create_mode;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut save_state = save_state;
        let mut show_deck_menu = show_deck_menu;
        let mut delete_state = delete_state;
        let mut show_delete_modal = show_delete_modal;
        let mut show_validation = show_validation;

        if !is_create_mode() {
            return;
        }

        if let Some(card) = last_selected_card() {
            selected_card_id.set(Some(card.id));
            prompt_text.set(card.prompt.clone());
            answer_text.set(card.answer.clone());
            is_create_mode.set(false);
        } else {
            selected_card_id.set(None);
            prompt_text.set(String::new());
            answer_text.set(String::new());
            is_create_mode.set(true);
        }

        save_state.set(SaveState::Idle);
        delete_state.set(DeleteState::Idle);
        show_delete_modal.set(false);
        show_validation.set(false);
        show_deck_menu.set(false);
    });

    let auto_select_action = select_card_action;
    let mut selected_card_id_for_effect = selected_card_id;
    let mut last_selected_card_for_effect = last_selected_card;
    let mut is_create_mode_for_effect = is_create_mode;
    let mut prompt_text_for_effect = prompt_text;
    let mut answer_text_for_effect = answer_text;
    let mut save_state_for_effect = save_state;
    let mut delete_state_for_effect = delete_state;
    let mut show_delete_modal_for_effect = show_delete_modal;
    let mut show_validation_for_effect = show_validation;
    use_effect(move || {
        let cards_state_effect = view_state_from_resource(&cards_resource);
        if let ViewState::Ready(items) = &cards_state_effect {
            if items.is_empty() {
                if !is_create_mode_for_effect() {
                    selected_card_id_for_effect.set(None);
                    last_selected_card_for_effect.set(None);
                    is_create_mode_for_effect.set(true);
                    prompt_text_for_effect.set(String::new());
                    answer_text_for_effect.set(String::new());
                    save_state_for_effect.set(SaveState::Idle);
                    delete_state_for_effect.set(DeleteState::Idle);
                    show_delete_modal_for_effect.set(false);
                    show_validation_for_effect.set(false);
                }
            } else if selected_card_id_for_effect().is_none()
                && !is_create_mode_for_effect()
                && let Some(first) = items.first()
            {
                auto_select_action.call(first.clone());
            }
        }
    });

    let deck_label = match &decks_state {
        ViewState::Ready(options) => options
            .iter()
            .find(|opt| opt.id == *selected_deck.read())
            .map_or_else(|| format!("{}", selected_deck.read().value()), |opt| {
                opt.label.clone()
            }),
        _ => "--".to_string(),
    };

    let can_cancel = is_create_mode() && last_selected_card().is_some();
    let prompt_invalid = show_validation() && prompt_text.read().trim().is_empty();
    let answer_invalid = show_validation() && answer_text.read().trim().is_empty();

    let on_key = {
        let deck_label = deck_label.clone();
        let decks_state = decks_state.clone();
        let mut is_renaming_deck = is_renaming_deck;
        let mut rename_deck_name = rename_deck_name;
        let mut rename_deck_state = rename_deck_state;
        let mut rename_deck_error = rename_deck_error;
        let mut show_deck_menu = show_deck_menu;
        let mut show_delete_modal = show_delete_modal;
        let is_create_mode = is_create_mode;
        let selected_card_id = selected_card_id;
        let delete_state = delete_state;
        let save_action = save_action;
        let cancel_new_action = cancel_new_action;
        let open_delete_modal_action = open_delete_modal_action;
        let close_delete_modal_action = close_delete_modal_action;
        use_callback(move |evt: KeyboardEvent| {
            if show_delete_modal() && evt.data.key() == Key::Escape {
                evt.prevent_default();
                close_delete_modal_action.call(());
                return;
            }

            if is_renaming_deck() {
                return;
            }

            if evt.data.modifiers().contains(Modifiers::META) {
                if evt.data.key() == Key::Enter {
                    evt.prevent_default();
                    save_action.call(false);
                    return;
                }

                if evt.data.key() == Key::Backspace
                    && selected_card_id().is_some()
                    && !is_create_mode()
                    && delete_state() != DeleteState::Deleting
                {
                    evt.prevent_default();
                    open_delete_modal_action.call(());
                    return;
                }

                if matches!(decks_state, ViewState::Ready(_))
                    && let Key::Character(value) = evt.data.key()
                    && value.eq_ignore_ascii_case("r")
                {
                    evt.prevent_default();
                    rename_deck_name.set(deck_label.clone());
                    rename_deck_state.set(SaveState::Idle);
                    rename_deck_error.set(None);
                    show_deck_menu.set(false);
                    show_delete_modal.set(false);
                    is_renaming_deck.set(true);
                    return;
                }
            }

            if evt.data.key() == Key::Escape && is_create_mode() {
                evt.prevent_default();
                cancel_new_action.call(());
            }
        })
    };

    rsx! {
        div { class: "page page--editor", tabindex: "0", onkeydown: on_key,
            if show_deck_menu() || is_renaming_deck() {
                div {
                    class: "editor-deck-overlay",
                    onclick: move |_| {
                        show_deck_menu.set(false);
                        if is_renaming_deck() {
                            cancel_rename_action.call(());
                        }
                    }
                }
            }
            if show_delete_modal() {
                div {
                    class: "editor-modal-overlay",
                    onclick: move |_| close_delete_modal_action.call(()),
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
                                onclick: move |_| close_delete_modal_action.call(()),
                                "Cancel"
                            }
                            button {
                                class: "btn editor-modal-confirm",
                                r#type: "button",
                                disabled: delete_state() == DeleteState::Deleting,
                                onclick: move |_| delete_action.call(()),
                                "Delete"
                            }
                        }
                    }
                }
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
                                                        commit_rename_action.call(());
                                                    }
                                                    Key::Escape => {
                                                        evt.prevent_default();
                                                        cancel_rename_action.call(());
                                                    }
                                                    _ => {}
                                                },
                                                onblur: move |_| {
                                                    if rename_deck_state() != SaveState::Saving {
                                                        cancel_rename_action.call(());
                                                    }
                                                },
                                                autofocus: true,
                                            }
                                        } else {
                                            button {
                                                class: "editor-deck-label",
                                                r#type: "button",
                                                ondoubleclick: move |_| {
                                                    begin_rename_action.call(deck_label_for_double.clone());
                                                },
                                                oncontextmenu: move |evt| {
                                                    evt.prevent_default();
                                                    begin_rename_action.call(deck_label_for_context.clone());
                                                },
                                                "{deck_label}"
                                            }
                                        }
                                        button {
                                            class: "editor-deck-caret-button",
                                            r#type: "button",
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
                                        span { class: "editor-deck-toast editor-deck-toast--error", "{error}" }
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
                                                    class: if opt.id == *selected_deck.read() {
                                                        "editor-deck-item editor-deck-item--active"
                                                    } else {
                                                        "editor-deck-item"
                                                    },
                                                    r#type: "button",
                                                    onclick: move |_| {
                                                        selected_deck.set(opt.id);
                                                        show_new_deck.set(false);
                                                        new_deck_state.set(SaveState::Idle);
                                                    selected_card_id.set(None);
                                                    last_selected_card.set(None);
                                                    is_create_mode.set(false);
                                                    prompt_text.set(String::new());
                                                    answer_text.set(String::new());
                                                    save_state.set(SaveState::Idle);
                                                    delete_state.set(DeleteState::Idle);
                                                    show_delete_modal.set(false);
                                                    show_deck_menu.set(false);
                                                    new_deck_name.set(String::new());
                                                    is_renaming_deck.set(false);
                                                    rename_deck_state.set(SaveState::Idle);
                                                    rename_deck_error.set(None);
                                                    },
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
                            onclick: move |_| new_card_action.call(()),
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
                            onclick: move |_| create_deck_action.call(()),
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
                    aside { class: "editor-list-pane",
                        div { class: "editor-list-header",
                            h3 { class: "editor-list-title", "Cards" }
                        }
                        match cards_state {
                            ViewState::Idle => rsx! {
                                p { class: "editor-list-empty", "Idle" }
                            },
                            ViewState::Loading => rsx! {
                                p { class: "editor-list-empty", "Loading cards..." }
                            },
                            ViewState::Error(err) => rsx! {
                                p { class: "editor-list-empty", "{err.message()}" }
                            },
                            ViewState::Ready(items) => {
                                let active_id = selected_card_id();
                                if items.is_empty() {
                                    rsx! {
                                        p { class: "editor-list-empty", "No cards yet." }
                                        button {
                                            class: "btn editor-list-cta",
                                            r#type: "button",
                                            onclick: move |_| new_card_action.call(()),
                                            "Create your first card"
                                        }
                                    }
                                } else {
                                    rsx! {
                                        ul { class: "editor-list-items",
                                            for item in items {
                                                li {
                                                    class: if Some(item.id) == active_id {
                                                        "editor-list-item editor-list-item--active"
                                                    } else {
                                                        "editor-list-item"
                                                    },
                                                    key: "{item.id.value()}",
                                                    onclick: move |_| select_card_action.call(item.clone()),
                                                    div { class: "editor-list-front", "{item.prompt_preview}" }
                                                    div { class: "editor-list-back", "{item.answer_preview}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    section { class: "editor-detail",
                        header { class: "editor-detail-header",
                            h3 { class: "editor-detail-title",
                                if is_create_mode() {
                                    "New Card"
                                } else if selected_card_id().is_some() {
                                    "Edit Card"
                                } else {
                                    "Select a Card"
                                }
                            }
                        }

                        div { class: "editor-body",
                            if !can_edit {
                                p { class: "editor-empty-hint", "Select a card or click + New Card." }
                            }
                            div { class: "editor-group",
                                label { class: "editor-label", r#for: "prompt", "Front" }
                                textarea {
                                    id: "prompt",
                                    class: if prompt_invalid {
                                        "editor-input editor-input--multi editor-input--error"
                                    } else {
                                        "editor-input editor-input--multi"
                                    },
                                    rows: 6,
                                    placeholder: "Enter the prompt for the front of the card...",
                                    value: "{prompt_text.read()}",
                                    disabled: !can_edit,
                                    oninput: move |evt| {
                                        prompt_text.set(evt.value());
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                                if prompt_invalid {
                                    p { class: "editor-error", "Front is required." }
                                }
                            }

                            div { class: "editor-group",
                                label { class: "editor-label", r#for: "answer", "Back" }
                                textarea {
                                    id: "answer",
                                    class: if answer_invalid {
                                        "editor-input editor-input--multi editor-input--error"
                                    } else {
                                        "editor-input editor-input--multi"
                                    },
                                    rows: 6,
                                    placeholder: "Enter the answer for the back of the card...",
                                    value: "{answer_text.read()}",
                                    disabled: !can_edit,
                                    oninput: move |evt| {
                                        answer_text.set(evt.value());
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                                if answer_invalid {
                                    p { class: "editor-error", "Back is required." }
                                }
                            }

                            button { class: "editor-add-inline", r#type: "button", disabled: !can_edit,
                                span { class: "editor-add-plus", "+" }
                                span { "Add Image" }
                            }
                        }

                        footer { class: "editor-footer",
                            div { class: "editor-status",
                                match delete_state() {
                                    DeleteState::Idle => match save_state() {
                                        SaveState::Idle => rsx! {},
                                        SaveState::Saving => rsx! { span { "Saving..." } },
                                        SaveState::Success => rsx! { span { "Saved." } },
                                        SaveState::Error(err) => rsx! { span { "{err.message()}" } },
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
                                    onclick: move |_| cancel_new_action.call(()),
                                    "Cancel"
                                }
                                if !is_create_mode() && selected_card_id().is_some() {
                                    button {
                                        class: "btn editor-delete",
                                        r#type: "button",
                                        disabled: delete_state() == DeleteState::Deleting
                                            || save_state() == SaveState::Saving,
                                        onclick: move |_| open_delete_modal_action.call(()),
                                        "Delete"
                                    }
                                }
                                button {
                                    class: "btn btn-primary editor-save",
                                    r#type: "button",
                                    disabled: !can_submit,
                                    onclick: move |_| save_action.call(false),
                                    "Save"
                                }
                                if is_create_mode() {
                                    button {
                                        class: "btn editor-practice",
                                        r#type: "button",
                                        disabled: !can_submit,
                                        onclick: move |_| save_action.call(true),
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
}

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

#[component]
pub fn EditorView() -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let deck_id = ctx.current_deck_id();
    let deck_service = ctx.deck_service();
    let deck_service_for_resource = deck_service.clone();
    let card_service = ctx.card_service();
    let card_service_for_list = card_service.clone();
    let mut selected_deck = use_signal(|| deck_id);
    let mut save_state = use_signal(|| SaveState::Idle);
    let mut show_new_deck = use_signal(|| false);
    let mut new_deck_name = use_signal(String::new);
    let mut new_deck_state = use_signal(|| SaveState::Idle);
    let mut show_deck_menu = use_signal(|| false);
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
    let can_save = {
        let p = prompt_text.read();
        let a = answer_text.read();
        can_edit && !p.trim().is_empty() && !a.trim().is_empty()
    };
    let save_action = use_callback(move |practice: bool| {
        let card_service = card_service.clone();
        let navigator = navigator.clone();
        let mut save_state = save_state;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut cards_resource = cards_resource;
        let mut selected_card_id = selected_card_id;
        let mut last_selected_card = last_selected_card;
        let is_create_mode = is_create_mode;
        let deck_id = *selected_deck.read();

        let prompt = prompt_text.read().trim().to_owned();
        let answer = answer_text.read().trim().to_owned();

        if prompt.is_empty() || answer.is_empty() || save_state() == SaveState::Saving {
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
                        .map(|_| Some(card_id))
                }
                None => card_service
                    .create_card(
                        deck_id,
                        ContentDraft::text_only(prompt.clone()),
                        ContentDraft::text_only(answer.clone()),
                    )
                    .await
                    .map(|id| Some(id)),
            };

            match result {
                Ok(card_id) => {
                    save_state.set(SaveState::Success);
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
        let deck_service = deck_service.clone();
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

        selected_card_id.set(Some(item.id));
        last_selected_card.set(Some(item.clone()));
        is_create_mode.set(false);
        prompt_text.set(item.prompt);
        answer_text.set(item.answer);
        save_state.set(SaveState::Idle);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        show_deck_menu.set(false);
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

        selected_card_id.set(None);
        is_create_mode.set(true);
        prompt_text.set(String::new());
        answer_text.set(String::new());
        save_state.set(SaveState::Idle);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        new_deck_name.set(String::new());
        show_deck_menu.set(false);
    });

    let cancel_new_action = use_callback(move |()| {
        let mut selected_card_id = selected_card_id;
        let last_selected_card = last_selected_card;
        let mut is_create_mode = is_create_mode;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let mut save_state = save_state;
        let mut show_deck_menu = show_deck_menu;

        if !is_create_mode() {
            return;
        }

        match last_selected_card() {
            Some(card) => {
                selected_card_id.set(Some(card.id));
                prompt_text.set(card.prompt.clone());
                answer_text.set(card.answer.clone());
                is_create_mode.set(false);
            }
            None => {
                selected_card_id.set(None);
                prompt_text.set(String::new());
                answer_text.set(String::new());
                is_create_mode.set(true);
            }
        }

        save_state.set(SaveState::Idle);
        show_deck_menu.set(false);
    });

    let auto_select_action = select_card_action.clone();
    let mut selected_card_id_for_effect = selected_card_id.clone();
    let mut last_selected_card_for_effect = last_selected_card.clone();
    let mut is_create_mode_for_effect = is_create_mode.clone();
    let mut prompt_text_for_effect = prompt_text.clone();
    let mut answer_text_for_effect = answer_text.clone();
    let mut save_state_for_effect = save_state.clone();
    use_effect(move || {
        let cards_state_effect = view_state_from_resource(&cards_resource);
        match &cards_state_effect {
            ViewState::Ready(items) => {
                if items.is_empty() {
                    if !is_create_mode_for_effect() {
                        selected_card_id_for_effect.set(None);
                        last_selected_card_for_effect.set(None);
                        is_create_mode_for_effect.set(true);
                        prompt_text_for_effect.set(String::new());
                        answer_text_for_effect.set(String::new());
                        save_state_for_effect.set(SaveState::Idle);
                    }
                } else if selected_card_id_for_effect().is_none()
                    && !is_create_mode_for_effect()
                {
                    if let Some(first) = items.first() {
                        auto_select_action.call(first.clone());
                    }
                }
            }
            _ => {}
        }
    });

    let deck_label = match &decks_state {
        ViewState::Ready(options) => options
            .iter()
            .find(|opt| opt.id == *selected_deck.read())
            .map(|opt| opt.label.clone())
            .unwrap_or_else(|| format!("{}", selected_deck.read().value())),
        _ => "--".to_string(),
    };

    let can_cancel = is_create_mode() && last_selected_card().is_some();

    rsx! {
        div { class: "page page--editor",
            if show_deck_menu() {
                div { class: "editor-deck-overlay", onclick: move |_| show_deck_menu.set(false) }
            }
            section { class: "editor-shell",
                header { class: "editor-toolbar",
                    div { class: "editor-toolbar-left editor-deck-menu",
                        match decks_state {
                            ViewState::Idle | ViewState::Loading => rsx! {
                                button { class: "editor-deck-trigger", disabled: true,
                                    span { "Loading decks..." }
                                }
                            },
                            ViewState::Error(_err) => rsx! {
                                button { class: "editor-deck-trigger", disabled: true,
                                    span { "Decks unavailable" }
                                }
                            },
                            ViewState::Ready(options) => rsx! {
                                button {
                                    class: "editor-deck-trigger",
                                    r#type: "button",
                                    onclick: move |_| show_deck_menu.set(!show_deck_menu()),
                                    span { class: "editor-deck-trigger-label", "{deck_label}" }
                                    span { class: "editor-deck-caret" }
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
                                                    show_deck_menu.set(false);
                                                    new_deck_name.set(String::new());
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
                                            },
                                            "+ New deck..."
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
                                    rsx! { p { class: "editor-list-empty", "No cards yet." } }
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
                                    class: "editor-input editor-input--multi",
                                    rows: 6,
                                    placeholder: "Enter the prompt for the front of the card...",
                                    value: "{prompt_text.read()}",
                                    disabled: !can_edit,
                                    oninput: move |evt| {
                                        prompt_text.set(evt.value());
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }

                            div { class: "editor-group",
                                label { class: "editor-label", r#for: "answer", "Back" }
                                textarea {
                                    id: "answer",
                                    class: "editor-input editor-input--multi",
                                    rows: 6,
                                    placeholder: "Enter the answer for the back of the card...",
                                    value: "{answer_text.read()}",
                                    disabled: !can_edit,
                                    oninput: move |evt| {
                                        answer_text.set(evt.value());
                                        save_state.set(SaveState::Idle);
                                    },
                                }
                            }

                            button { class: "editor-add-inline", r#type: "button", disabled: !can_edit,
                                span { class: "editor-add-plus", "+" }
                                span { "Add Image" }
                            }
                        }

                        footer { class: "editor-footer",
                            div { class: "editor-status",
                                match save_state() {
                                    SaveState::Idle => rsx! {},
                                    SaveState::Saving => rsx! { span { "Saving..." } },
                                    SaveState::Success => rsx! { span { "Saved." } },
                                    SaveState::Error(err) => rsx! { span { "{err.message()}" } },
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
                                        disabled: true,
                                        "Delete"
                                    }
                                }
                                button {
                                    class: "btn btn-primary editor-save",
                                    r#type: "button",
                                    disabled: !can_save || save_state() == SaveState::Saving,
                                    onclick: move |_| save_action.call(false),
                                    "Save"
                                }
                                if is_create_mode() {
                                    button {
                                        class: "btn editor-practice",
                                        r#type: "button",
                                        disabled: !can_save || save_state() == SaveState::Saving,
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

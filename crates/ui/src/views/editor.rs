use dioxus::prelude::*;
use dioxus_router::use_navigator;
use learn_core::model::{ContentDraft, DeckId, DeckSettings};

use crate::context::AppContext;
use crate::routes::Route;
use crate::vm::map_deck_options;
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
    let mut selected_deck = use_signal(|| deck_id);
    let mut save_state = use_signal(|| SaveState::Idle);
    let mut show_new_deck = use_signal(|| false);
    let mut new_deck_name = use_signal(String::new);
    let mut new_deck_state = use_signal(|| SaveState::Idle);

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

    // UI-only state for now (service wiring comes next step).
    let mut prompt_text = use_signal(String::new);
    let mut answer_text = use_signal(String::new);

    let can_save = {
        let p = prompt_text.read();
        let a = answer_text.read();
        !p.trim().is_empty() && !a.trim().is_empty()
    };
    let save_action = use_callback(move |practice: bool| {
        let card_service = card_service.clone();
        let navigator = navigator.clone();
        let mut save_state = save_state;
        let mut prompt_text = prompt_text;
        let mut answer_text = answer_text;
        let deck_id = *selected_deck.read();

        let prompt = prompt_text.read().trim().to_owned();
        let answer = answer_text.read().trim().to_owned();

        if prompt.is_empty() || answer.is_empty() || save_state() == SaveState::Saving {
            return;
        }

        spawn(async move {
            save_state.set(SaveState::Saving);
            let result = card_service
                .create_card(
                    deck_id,
                    ContentDraft::text_only(prompt),
                    ContentDraft::text_only(answer),
                )
                .await;

            match result {
                Ok(_) => {
                    save_state.set(SaveState::Success);
                    if practice {
                        navigator.push(Route::Session {
                            deck_id: deck_id.value(),
                        });
                    } else {
                        prompt_text.set(String::new());
                        answer_text.set(String::new());
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
                    new_deck_state.set(SaveState::Success);
                    decks_resource.restart();
                }
                Err(_) => {
                    new_deck_state.set(SaveState::Error(ViewError::Unknown));
                }
            }
        });
    });

    rsx! {
        div { class: "page",
            section { class: "editor-shell",
                header { class: "editor-header",
                    h2 { class: "editor-title", "New Card" }
                    div { class: "editor-deck-row",
                        span { class: "editor-deck-label", "Deck" }
                        match decks_state {
                            ViewState::Idle | ViewState::Loading => rsx! {
                                select { class: "editor-deck-select", disabled: true,
                                    option { "Loading decks..." }
                                }
                            },
                            ViewState::Error(_err) => rsx! {
                                select { class: "editor-deck-select", disabled: true,
                                    option { "Decks unavailable" }
                                }
                            },
                            ViewState::Ready(options) => {
                                let selected_id = selected_deck.read().value().to_string();
                                rsx! {
                                    select {
                                        class: "editor-deck-select",
                                        value: "{selected_id}",
                                        onchange: move |evt| {
                                            let value = evt.value();
                                            if value == "__new_deck__" {
                                                show_new_deck.set(true);
                                                new_deck_state.set(SaveState::Idle);
                                                return;
                                            }
                                            if let Ok(parsed) = value.parse::<u64>() {
                                                selected_deck.set(DeckId::new(parsed));
                                                show_new_deck.set(false);
                                                new_deck_state.set(SaveState::Idle);
                                            }
                                        },
                                        for opt in options {
                                            option {
                                                key: "{opt.id.value()}",
                                                value: "{opt.id.value()}",
                                                "{opt.label}"
                                            }
                                        }
                                        option { value: "__new_deck__", "+ New deck..." }
                                    }
                                }
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
                }

                div { class: "editor-body",
                    div { class: "editor-group",
                        label { class: "editor-label", r#for: "prompt", "Front" }
                        input {
                            id: "prompt",
                            class: "editor-input editor-input--single",
                            r#type: "text",
                            placeholder: "Enter the prompt for the front of the card...",
                            value: "{prompt_text.read()}",
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
                            oninput: move |evt| {
                                answer_text.set(evt.value());
                                save_state.set(SaveState::Idle);
                            },
                        }
                    }

                    button { class: "editor-add-inline", r#type: "button",
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
                        button { class: "btn editor-cancel", r#type: "button", "Cancel" }
                        button {
                            class: "btn editor-save",
                            r#type: "button",
                            disabled: !can_save || save_state() == SaveState::Saving,
                            onclick: move |_| save_action.call(false),
                            "Save"
                        }
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

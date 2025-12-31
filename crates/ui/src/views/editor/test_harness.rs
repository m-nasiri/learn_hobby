use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use dioxus::core::NoOpMutations;
use dioxus::prelude::*;
use dioxus_router::{Routable, Router};
use learn_core::model::DeckSettings;
use learn_core::time::fixed_now;
use services::{CardService, Clock, DeckService};
use storage::repository::Storage;

use super::actions::{EditorIntent, use_editor_dispatcher};
use super::state::{EditorServices, EditorState, use_editor_state};

#[derive(Clone, Default)]
pub struct HarnessHandles {
    dispatch: Rc<RefCell<Option<Callback<EditorIntent>>>>,
    state: Rc<RefCell<Option<EditorState>>>,
}

impl HarnessHandles {
    pub fn dispatch(&self) -> Callback<EditorIntent> {
        self.dispatch
            .borrow()
            .clone()
            .expect("dispatch registered")
    }

    pub fn state(&self) -> EditorState {
        self.state.borrow().clone().expect("state registered")
    }
}

#[derive(Props, Clone)]
struct HarnessProps {
    deck_id: learn_core::model::DeckId,
    services: EditorServices,
    handles: HarnessHandles,
}

impl PartialEq for HarnessProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for HarnessProps {}

#[component]
fn EditorIntentHarness(props: HarnessProps) -> Element {
    let state = use_editor_state(props.deck_id, &props.services);
    let dispatcher = use_editor_dispatcher(&state, &props.services);
    let mut registered = use_signal(|| false);
    if !registered() {
        registered.set(true);
        *props.handles.dispatch.borrow_mut() = Some(dispatcher.dispatch);
        *props.handles.state.borrow_mut() = Some(state.clone());
    }
    rsx! { div {} }
}

#[component]
fn EditorRouterHarness(props: HarnessProps) -> Element {
    use_context_provider(|| props);
    rsx! { Router::<TestRoute> {} }
}

#[derive(Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum TestRoute {
    #[route("/")]
    Root {},
}

#[component]
fn Root() -> Element {
    let props = use_context::<HarnessProps>();
    rsx! {
        EditorIntentHarness {
            deck_id: props.deck_id,
            services: props.services.clone(),
            handles: props.handles.clone(),
        }
    }
}

pub struct EditorHarness {
    pub dom: VirtualDom,
    handles: HarnessHandles,
}

impl EditorHarness {
    pub fn dispatch(&self) -> Callback<EditorIntent> {
        self.handles.dispatch()
    }

    pub fn state(&self) -> EditorState {
        self.handles.state()
    }

    pub fn drive(&mut self) {
        drive_dom(&mut self.dom);
    }
}

pub fn drive_dom(dom: &mut VirtualDom) {
    dom.process_events();
    dom.render_immediate(&mut NoOpMutations);
    dom.process_events();
}

pub fn set_fields(state: &EditorState, prompt: &str, answer: &str) {
    let mut prompt_text = state.prompt_text;
    let mut answer_text = state.answer_text;
    prompt_text.set(prompt.to_string());
    answer_text.set(answer.to_string());
}

pub async fn setup_editor_harness(
    deck_name: &str,
) -> (
    EditorHarness,
    Arc<DeckService>,
    Arc<CardService>,
    learn_core::model::DeckId,
) {
    let storage = Storage::in_memory();
    let clock = Clock::fixed(fixed_now());
    let deck_service = Arc::new(DeckService::new(clock, Arc::clone(&storage.decks)));
    let card_service = Arc::new(CardService::new(clock, Arc::clone(&storage.cards)));

    let deck_id = deck_service
        .create_deck(
            deck_name.to_string(),
            None,
            DeckSettings::default_for_adhd(),
        )
        .await
        .expect("create deck");

    let services = EditorServices {
        deck_service: Arc::clone(&deck_service),
        card_service: Arc::clone(&card_service),
    };
    let handles = HarnessHandles::default();
    let mut dom = VirtualDom::new_with_props(
        EditorRouterHarness,
        HarnessProps {
            deck_id,
            services,
            handles: handles.clone(),
        },
    );
    dom.rebuild_in_place();
    drive_dom(&mut dom);

    let harness = EditorHarness { dom, handles };
    (harness, deck_service, card_service, deck_id)
}

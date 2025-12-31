use std::sync::Arc;

use dioxus::core::NoOpMutations;
use dioxus::prelude::*;
use dioxus_router::{Routable, Router};
use learn_core::model::DeckId;
use learn_core::model::DeckSettings;
use learn_core::time::fixed_now;
use services::{
    CardService, Clock, DeckService, SessionLoopService, SessionSummaryService,
};
use storage::repository::{SessionSummaryRepository, Storage};

use crate::context::{UiApp, build_app_context};
use crate::views::{HistoryView, HomeView, SummaryView, SessionView};
use crate::views::session::SessionTestHandles;

#[derive(Clone)]
struct TestApp {
    deck_id: DeckId,
    session_summaries: Arc<SessionSummaryService>,
    session_loop: Arc<SessionLoopService>,
    card_service: Arc<CardService>,
    deck_service: Arc<DeckService>,
}

impl UiApp for TestApp {
    fn current_deck_id(&self) -> DeckId {
        self.deck_id
    }

    fn open_editor_on_launch(&self) -> bool {
        false
    }

    fn session_summaries(&self) -> Arc<SessionSummaryService> {
        Arc::clone(&self.session_summaries)
    }

    fn session_loop(&self) -> Arc<SessionLoopService> {
        Arc::clone(&self.session_loop)
    }

    fn card_service(&self) -> Arc<CardService> {
        Arc::clone(&self.card_service)
    }

    fn deck_service(&self) -> Arc<DeckService> {
        Arc::clone(&self.deck_service)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    Home,
    History,
    Summary(i64),
    Session(u64),
}

#[derive(Props, Clone)]
struct ViewHarnessProps {
    app: Arc<TestApp>,
    view: ViewKind,
    session_handles: Option<SessionTestHandles>,
}

impl PartialEq for ViewHarnessProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for ViewHarnessProps {}

#[component]
fn ViewRouterHarness(props: ViewHarnessProps) -> Element {
    let app: Arc<dyn UiApp> = props.app.clone();
    use_context_provider(|| build_app_context(&app));
    use_context_provider(|| props.view);
    if let Some(handles) = props.session_handles.clone() {
        use_context_provider(|| handles);
    }
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
    let view = use_context::<ViewKind>();
    match view {
        ViewKind::Home => rsx! { HomeView {} },
        ViewKind::History => rsx! { HistoryView {} },
        ViewKind::Summary(summary_id) => rsx! { SummaryView { summary_id } },
        ViewKind::Session(deck_id) => rsx! { SessionView { deck_id } },
    }
}

pub struct ViewHarness {
    pub dom: VirtualDom,
    pub storage: Storage,
    pub deck_id: DeckId,
    pub card_service: Arc<CardService>,
    pub session_handles: Option<SessionTestHandles>,
}

impl ViewHarness {
    pub fn rebuild(&mut self) {
        self.dom.rebuild_in_place();
        drive_dom(&mut self.dom);
    }

    pub async fn drive_async(&mut self) {
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            self.dom.wait_for_work(),
        )
        .await;
        self.dom.render_immediate(&mut NoOpMutations);
        self.dom.process_events();
    }

    pub fn render(&self) -> String {
        dioxus_ssr::render(&self.dom)
    }
}

pub fn drive_dom(dom: &mut VirtualDom) {
    dom.process_events();
    dom.render_immediate(&mut NoOpMutations);
    dom.process_events();
}

pub async fn setup_view_harness(view: ViewKind, deck_name: &str) -> ViewHarness {
    let storage = Storage::in_memory();
    let summaries = Arc::clone(&storage.session_summaries);
    setup_view_harness_with_summary_repo(view, deck_name, storage, summaries).await
}

pub async fn setup_view_harness_with_summary_repo(
    view: ViewKind,
    deck_name: &str,
    storage: Storage,
    summaries: Arc<dyn SessionSummaryRepository>,
) -> ViewHarness {
    let clock = Clock::fixed(fixed_now());
    let deck_service = Arc::new(DeckService::new(clock, Arc::clone(&storage.decks)));
    let card_service = Arc::new(CardService::new(clock, Arc::clone(&storage.cards)));
    let card_service_for_harness = Arc::clone(&card_service);
    let session_summaries = Arc::new(SessionSummaryService::new(clock, Arc::clone(&summaries)));
    let session_loop = Arc::new(SessionLoopService::new(
        clock,
        Arc::clone(&storage.decks),
        Arc::clone(&storage.cards),
        Arc::clone(&storage.reviews),
        Arc::clone(&summaries),
    ));

    let deck_id = deck_service
        .create_deck(
            deck_name.to_string(),
            None,
            DeckSettings::default_for_adhd(),
        )
        .await
        .expect("create deck");

    let view = match view {
        ViewKind::Session(_) => ViewKind::Session(deck_id.value()),
        other => other,
    };
    let session_handles = match view {
        ViewKind::Session(_) => Some(SessionTestHandles::default()),
        _ => None,
    };

    let app = Arc::new(TestApp {
        deck_id,
        session_summaries,
        session_loop,
        card_service,
        deck_service,
    });

    let dom = VirtualDom::new_with_props(
        ViewRouterHarness,
        ViewHarnessProps {
            app,
            view,
            session_handles: session_handles.clone(),
        },
    );

    ViewHarness {
        dom,
        storage,
        deck_id,
        card_service: card_service_for_harness,
        session_handles,
    }
}

pub async fn setup_view_harness_with_session_loop(
    view: ViewKind,
    deck_name: &str,
    storage: Storage,
    session_loop: Arc<SessionLoopService>,
) -> ViewHarness {
    let clock = Clock::fixed(fixed_now());
    let deck_service = Arc::new(DeckService::new(clock, Arc::clone(&storage.decks)));
    let card_service = Arc::new(CardService::new(clock, Arc::clone(&storage.cards)));
    let card_service_for_harness = Arc::clone(&card_service);
    let session_summaries = Arc::new(SessionSummaryService::new(
        clock,
        Arc::clone(&storage.session_summaries),
    ));

    let deck_id = deck_service
        .create_deck(
            deck_name.to_string(),
            None,
            DeckSettings::default_for_adhd(),
        )
        .await
        .expect("create deck");

    let view = match view {
        ViewKind::Session(_) => ViewKind::Session(deck_id.value()),
        other => other,
    };
    let session_handles = match view {
        ViewKind::Session(_) => Some(SessionTestHandles::default()),
        _ => None,
    };

    let app = Arc::new(TestApp {
        deck_id,
        session_summaries,
        session_loop,
        card_service,
        deck_service,
    });

    let dom = VirtualDom::new_with_props(
        ViewRouterHarness,
        ViewHarnessProps {
            app,
            view,
            session_handles: session_handles.clone(),
        },
    );

    ViewHarness {
        dom,
        storage,
        deck_id,
        card_service: card_service_for_harness,
        session_handles,
    }
}

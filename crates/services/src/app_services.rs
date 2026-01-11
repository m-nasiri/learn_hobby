use std::sync::Arc;

use learn_core::model::{Deck, DeckId, DeckSettings};
use storage::repository::{DeckRepository, NewDeckRecord, Storage};

use crate::ai::AiUsageService;
use crate::card_service::CardService;
use crate::deck_service::DeckService;
use crate::error::AppServicesError;
use crate::app_settings_service::AppSettingsService;
use crate::sessions::{SessionLoopService, SessionSummaryService};
use crate::writing_tools_service::WritingToolsService;
use crate::Clock;

/// Assembles app-facing services and resolves a usable deck id.
#[derive(Clone)]
pub struct AppServices {
    deck_id: DeckId,
    open_editor_on_launch: bool,
    session_summaries: Arc<SessionSummaryService>,
    session_loop: Arc<SessionLoopService>,
    card_service: Arc<CardService>,
    deck_service: Arc<DeckService>,
    app_settings: Arc<AppSettingsService>,
    writing_tools: Arc<WritingToolsService>,
}

impl AppServices {
    /// Build services backed by `SQLite` storage.
    ///
    /// # Errors
    ///
    /// Returns `AppServicesError` if storage initialization or default deck setup fails.
    pub async fn new_sqlite(
        db_url: &str,
        clock: Clock,
        preferred_deck_id: DeckId,
    ) -> Result<Self, AppServicesError> {
        let storage = Storage::sqlite(db_url).await?;
        let (deck_id, open_editor_on_launch) =
            ensure_default_deck(storage.decks.as_ref(), clock, preferred_deck_id).await?;

        let session_summaries = Arc::new(SessionSummaryService::new(
            clock,
            Arc::clone(&storage.session_summaries),
        ));
        let session_loop = Arc::new(SessionLoopService::new(
            clock,
            Arc::clone(&storage.decks),
            Arc::clone(&storage.cards),
            Arc::clone(&storage.reviews),
            Arc::clone(&storage.session_summaries),
        ));
        let app_settings = Arc::new(AppSettingsService::new(Arc::clone(&storage.app_settings)));
        let ai_usage = Arc::new(AiUsageService::new(
            clock,
            Arc::clone(&storage.app_settings),
            Arc::clone(&storage.ai_usage),
            Arc::clone(&storage.ai_price_book),
        ));
        let card_service = Arc::new(CardService::new(clock, Arc::clone(&storage.cards)));
        let deck_service = Arc::new(DeckService::new(clock, Arc::clone(&storage.decks)));
        let writing_tools = Arc::new(WritingToolsService::from_env(
            Arc::clone(&storage.app_settings),
            Arc::clone(&ai_usage),
        ));

        Ok(Self {
            deck_id,
            open_editor_on_launch,
            session_summaries,
            session_loop,
            card_service,
            deck_service,
            app_settings,
            writing_tools,
        })
    }

    #[must_use]
    pub fn deck_id(&self) -> DeckId {
        self.deck_id
    }

    #[must_use]
    pub fn open_editor_on_launch(&self) -> bool {
        self.open_editor_on_launch
    }

    #[must_use]
    pub fn session_summaries(&self) -> Arc<SessionSummaryService> {
        Arc::clone(&self.session_summaries)
    }

    #[must_use]
    pub fn session_loop(&self) -> Arc<SessionLoopService> {
        Arc::clone(&self.session_loop)
    }

    #[must_use]
    pub fn card_service(&self) -> Arc<CardService> {
        Arc::clone(&self.card_service)
    }

    #[must_use]
    pub fn deck_service(&self) -> Arc<DeckService> {
        Arc::clone(&self.deck_service)
    }

    #[must_use]
    pub fn app_settings(&self) -> Arc<AppSettingsService> {
        Arc::clone(&self.app_settings)
    }

    #[must_use]
    pub fn writing_tools(&self) -> Arc<WritingToolsService> {
        Arc::clone(&self.writing_tools)
    }
}

async fn ensure_default_deck(
    decks: &dyn DeckRepository,
    clock: Clock,
    preferred_id: DeckId,
) -> Result<(DeckId, bool), AppServicesError> {
    if decks.get_deck(preferred_id).await?.is_some() {
        return Ok((preferred_id, false));
    }

    let existing = decks.list_decks(128).await?;
    if let Some(first) = existing.first() {
        return Ok((first.id(), false));
    }

    let now = clock.now();
    let deck = Deck::new(
        DeckId::new(1),
        "Default Deck",
        None,
        DeckSettings::default_for_adhd(),
        now,
    )?;
    let deck_id = decks
        .insert_new_deck(NewDeckRecord::from_deck(&deck))
        .await?;

    Ok((deck_id, true))
}

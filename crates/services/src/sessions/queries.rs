use chrono::{DateTime, Datelike, Utc};
use rand::rng;
use rand::seq::SliceRandom;

use learn_core::model::{CardPhase, Deck, DeckId, SessionSummary, TagName};
use storage::repository::{
    CardRepository, DeckRepository, SessionSummaryRepository, SessionSummaryRow,
};

use crate::error::SessionError;
use super::plan::{SessionBuilder, SessionPlan};
use super::service::SessionService;

/// Storage-backed session queries and builders.
pub(crate) struct SessionQueries;

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn apply_easy_day_limit(limit: u32, factor: f32) -> u32 {
    let scaled = f64::from(limit) * f64::from(factor);
    if scaled <= 0.0 {
        return 0;
    }
    if scaled >= f64::from(u32::MAX) {
        return u32::MAX;
    }
    scaled.floor() as u32
}

fn effective_daily_limits(
    settings: &learn_core::model::DeckSettings,
    now: DateTime<Utc>,
) -> (u32, u32) {
    if !settings.is_easy_day(now.weekday()) {
        return (settings.review_limit_per_day(), settings.new_cards_per_day());
    }
    let factor = settings.easy_day_load_factor();
    (
        apply_easy_day_limit(settings.review_limit_per_day(), factor),
        apply_easy_day_limit(settings.new_cards_per_day(), factor),
    )
}

// Some query helpers are used only in tests or planned UI flows.
#[allow(dead_code)]
impl SessionQueries {
    /// Build a session plan using repository data.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` when repository access fails.
    pub async fn build_plan_from_storage(
        deck_id: DeckId,
        decks: &dyn DeckRepository,
        cards: &dyn CardRepository,
        now: DateTime<Utc>,
        shuffle_new: bool,
    ) -> Result<(Deck, SessionPlan), SessionError> {
        let deck = decks
            .get_deck(deck_id)
            .await?
            .ok_or(storage::repository::StorageError::NotFound)?;
        let settings = deck.settings();
        let (review_limit, new_limit) = effective_daily_limits(settings, now);
        let due = cards
            .due_cards(deck_id, now, review_limit)
            .await?;
        let new_cards = cards
            .new_cards(deck_id, new_limit)
            .await?;

        let plan = SessionBuilder::new(&deck)
            .with_shuffle_new(shuffle_new)
            .build(due, new_cards);

        Ok((deck, plan))
    }

    /// Create a session directly from storage-backed data.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are available, or
    /// `SessionError::Storage` on repository failures.
    pub async fn start_from_storage(
        deck_id: DeckId,
        decks: &dyn DeckRepository,
        cards: &dyn CardRepository,
        now: DateTime<Utc>,
        shuffle_new: bool,
    ) -> Result<(Deck, SessionService), SessionError> {
        let (deck, plan) =
            Self::build_plan_from_storage(deck_id, decks, cards, now, shuffle_new).await?;
        let session = SessionService::new(&deck, plan.cards, now)?;
        Ok((deck, session))
    }

    /// Create a session from all cards in a deck.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are available, or
    /// `SessionError::Storage` on repository failures.
    pub async fn start_from_storage_all_cards(
        deck_id: DeckId,
        decks: &dyn DeckRepository,
        cards: &dyn CardRepository,
        now: DateTime<Utc>,
    ) -> Result<(Deck, SessionService), SessionError> {
        let deck = decks
            .get_deck(deck_id)
            .await?
            .ok_or(storage::repository::StorageError::NotFound)?;
        let cards = cards.list_cards(deck_id, u32::MAX).await?;
        let session = SessionService::new_all(&deck, cards, now)?;
        Ok((deck, session))
    }

    /// Create a session from cards currently in relearning (mistakes).
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards match, or
    /// `SessionError::Storage` on repository failures.
    pub async fn start_from_storage_mistakes(
        deck_id: DeckId,
        decks: &dyn DeckRepository,
        cards: &dyn CardRepository,
        now: DateTime<Utc>,
    ) -> Result<(Deck, SessionService), SessionError> {
        let deck = decks
            .get_deck(deck_id)
            .await?
            .ok_or(storage::repository::StorageError::NotFound)?;
        let mut mistakes: Vec<_> = cards
            .list_cards(deck_id, u32::MAX)
            .await?
            .into_iter()
            .filter(|card| card.phase() == CardPhase::Relearning)
            .collect();
        mistakes.sort_by_key(|card| (card.next_review_at(), card.id().value()));
        let session = SessionService::new(&deck, mistakes, now)?;
        Ok((deck, session))
    }

    /// Create a session from storage filtered by tags.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are available, or
    /// `SessionError::Storage` on repository failures.
    pub async fn start_from_storage_with_tags(
        deck_id: DeckId,
        decks: &dyn DeckRepository,
        cards: &dyn CardRepository,
        now: DateTime<Utc>,
        shuffle_new: bool,
        tag_names: &[TagName],
    ) -> Result<(Deck, SessionService), SessionError> {
        if tag_names.is_empty() {
            return Self::start_from_storage(deck_id, decks, cards, now, shuffle_new).await;
        }

        let deck = decks
            .get_deck(deck_id)
            .await?
            .ok_or(storage::repository::StorageError::NotFound)?;
        let settings = deck.settings();
        let (review_limit, new_limit) = effective_daily_limits(settings, now);

        let tagged_cards = cards.list_cards_by_tags(deck_id, tag_names).await?;
        let mut due = Vec::new();
        let mut new_cards = Vec::new();

        for card in tagged_cards {
            if card.is_new() {
                new_cards.push(card);
            } else if card.is_due(now) {
                due.push(card);
            }
        }

        if due.is_empty() && new_cards.is_empty() {
            return Err(SessionError::Empty);
        }

        if due.len() > review_limit as usize {
            due.sort_by_key(|card| (card.next_review_at(), card.id().value()));
            due.truncate(review_limit as usize);
        }
        if new_cards.len() > new_limit as usize {
            if shuffle_new {
                let mut rng = rng();
                new_cards.as_mut_slice().shuffle(&mut rng);
            } else {
                new_cards.sort_by_key(|card| (card.created_at(), card.id().value()));
            }
            new_cards.truncate(new_limit as usize);
        }

        let plan = SessionBuilder::new(&deck)
            .with_shuffle_new(shuffle_new)
            .build(due, new_cards);

        let session = SessionService::new(&deck, plan.cards, now)?;
        Ok((deck, session))
    }

    /// Create a session directly from storage and return the plan for UI summary.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are available, or
    /// `SessionError::Storage` on repository failures.
    pub async fn start_from_storage_with_plan(
        deck_id: DeckId,
        decks: &dyn DeckRepository,
        cards: &dyn CardRepository,
        now: DateTime<Utc>,
        shuffle_new: bool,
    ) -> Result<(Deck, SessionPlan, SessionService), SessionError> {
        let (deck, plan) =
            Self::build_plan_from_storage(deck_id, decks, cards, now, shuffle_new).await?;
        let session = SessionService::new(&deck, plan.cards.clone(), now)?;
        Ok((deck, plan, session))
    }

    /// List persisted session summaries for a deck within an optional time range.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` on repository failures.
    pub async fn list_summaries(
        deck_id: DeckId,
        summaries: &dyn SessionSummaryRepository,
        completed_from: Option<DateTime<Utc>>,
        completed_until: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummary>, SessionError> {
        let items = summaries
            .list_summaries(deck_id, completed_from, completed_until, limit)
            .await?;
        Ok(items)
    }

    /// List persisted session summaries for a deck within an optional time range, preserving IDs.
    ///
    /// This is useful for UI navigation (e.g. “open summary details”) without requiring a follow-up lookup.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` on repository failures.
    pub async fn list_summary_rows(
        deck_id: DeckId,
        summaries: &dyn SessionSummaryRepository,
        completed_from: Option<DateTime<Utc>>,
        completed_until: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummaryRow>, SessionError> {
        let items = summaries
            .list_summary_rows(deck_id, completed_from, completed_until, limit)
            .await?;
        Ok(items)
    }

    /// Fetch a persisted session summary by ID.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` if the summary is missing or storage fails.
    pub async fn get_summary(
        id: i64,
        summaries: &dyn SessionSummaryRepository,
    ) -> Result<SessionSummary, SessionError> {
        let summary = summaries.get_summary(id).await?;
        Ok(summary)
    }

    /// Fetch a persisted session summary row by ID.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` if the summary is missing or storage fails.
    pub async fn get_summary_row(
        id: i64,
        summaries: &dyn SessionSummaryRepository,
    ) -> Result<SessionSummaryRow, SessionError> {
        let summary = summaries.get_summary(id).await?;
        Ok(SessionSummaryRow::new(id, summary))
    }

    /// List recent summaries for a deck with a default time window.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` on repository failures.
    pub async fn list_recent_summaries(
        deck_id: DeckId,
        summaries: &dyn SessionSummaryRepository,
        now: DateTime<Utc>,
        days: i64,
        limit: u32,
    ) -> Result<Vec<SessionSummary>, SessionError> {
        let from = now - chrono::Duration::days(days);
        Self::list_summaries(deck_id, summaries, Some(from), Some(now), limit).await
    }

    /// List recent persisted session summaries for a deck within a default time window, preserving IDs.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` on repository failures.
    pub async fn list_recent_summary_rows(
        deck_id: DeckId,
        summaries: &dyn SessionSummaryRepository,
        now: DateTime<Utc>,
        days: i64,
        limit: u32,
    ) -> Result<Vec<SessionSummaryRow>, SessionError> {
        let from = now - chrono::Duration::days(days);
        Self::list_summary_rows(deck_id, summaries, Some(from), Some(now), limit).await
    }

    /// List the latest summary row for each deck.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Storage` on repository failures.
    pub async fn list_latest_summary_rows(
        deck_ids: &[DeckId],
        summaries: &dyn SessionSummaryRepository,
    ) -> Result<Vec<SessionSummaryRow>, SessionError> {
        let rows = summaries.list_latest_summary_rows(deck_ids).await?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use learn_core::model::{CardId, ReviewGrade, ReviewLog, SessionSummary, TagName};
    use learn_core::time::fixed_now;
    use storage::repository::{CardRepository, DeckRepository, InMemoryRepository};

    fn build_deck() -> Deck {
        Deck::new(
            DeckId::new(1),
            "Test",
            None,
            learn_core::model::DeckSettings::default_for_adhd(),
            fixed_now(),
        )
        .unwrap()
    }

    fn build_card(id: u64) -> learn_core::model::Card {
        let prompt = learn_core::model::content::ContentDraft::text_only("Q")
            .validate(fixed_now(), None, None)
            .unwrap();
        let answer = learn_core::model::content::ContentDraft::text_only("A")
            .validate(fixed_now(), None, None)
            .unwrap();
        let now = fixed_now();
        learn_core::model::Card::new(CardId::new(id), DeckId::new(1), prompt, answer, now, now)
            .unwrap()
    }

    #[tokio::test]
    async fn start_from_storage_builds_session() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card1 = build_card(1);
        repo.upsert_card(&card1).await.unwrap();

        let now = fixed_now();
        let (loaded, session) =
            SessionQueries::start_from_storage(deck.id(), &repo, &repo, now, false)
                .await
                .unwrap();

        assert_eq!(loaded.id(), deck.id());
        assert_eq!(session.total_cards(), 1);
    }

    #[tokio::test]
    async fn start_from_storage_with_plan_returns_summary() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card1 = build_card(1);
        let card2 = build_card(2);
        repo.upsert_card(&card1).await.unwrap();
        repo.upsert_card(&card2).await.unwrap();

        let now = fixed_now();
        let (loaded, plan, session) =
            SessionQueries::start_from_storage_with_plan(deck.id(), &repo, &repo, now, false)
                .await
                .unwrap();

        assert_eq!(loaded.id(), deck.id());
        assert_eq!(plan.total(), session.total_cards());
        assert!(plan.total() > 0);
    }

    #[tokio::test]
    async fn start_from_storage_with_tags_filters_cards() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card1 = build_card(1);
        repo.upsert_card(&card1).await.unwrap();
        let tag = TagName::new("Language").unwrap();
        repo.set_tags_for_card(deck.id(), card1.id(), &[tag.clone()])
            .await
            .unwrap();

        let now = fixed_now();
        let (_loaded, session) = SessionQueries::start_from_storage_with_tags(
            deck.id(),
            &repo,
            &repo,
            now,
            false,
            &[tag],
        )
        .await
        .unwrap();

        assert_eq!(session.total_cards(), 1);

        let other_tag = TagName::new("Other").unwrap();
        let err = SessionQueries::start_from_storage_with_tags(
            deck.id(),
            &repo,
            &repo,
            now,
            false,
            &[other_tag],
        )
        .await
        .unwrap_err();

        assert!(matches!(err, SessionError::Empty));
    }

    #[tokio::test]
    async fn list_summaries_returns_recent_first() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let now = fixed_now();
        let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];

        let summary_recent = SessionSummary::from_logs(
            deck.id(),
            now - chrono::Duration::days(2),
            now - chrono::Duration::days(1),
            &logs,
        )
        .unwrap();
        let summary_old = SessionSummary::from_logs(
            deck.id(),
            now - chrono::Duration::days(10),
            now - chrono::Duration::days(9),
            &logs,
        )
        .unwrap();

        let _id_recent = repo.append_summary(&summary_recent).await.unwrap();
        let _id_old = repo.append_summary(&summary_old).await.unwrap();

        let listed = SessionQueries::list_summaries(deck.id(), &repo, None, None, 10)
            .await
            .unwrap();

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].completed_at(), summary_recent.completed_at());
        assert_eq!(listed[1].completed_at(), summary_old.completed_at());
    }

    #[tokio::test]
    async fn list_summary_rows_returns_ids() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let now = fixed_now();
        let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];
        let summary = SessionSummary::from_logs(
            deck.id(),
            now - chrono::Duration::days(1),
            now,
            &logs,
        )
        .unwrap();

        let id = repo.append_summary(&summary).await.unwrap();

        let rows = SessionQueries::list_summary_rows(deck.id(), &repo, None, None, 10)
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].summary.completed_at(), summary.completed_at());
    }

    #[tokio::test]
    async fn get_summary_row_returns_id_and_summary() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let now = fixed_now();
        let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];
        let summary = SessionSummary::from_logs(deck.id(), now, now, &logs).unwrap();
        let id = repo.append_summary(&summary).await.unwrap();

        let row = SessionQueries::get_summary_row(id, &repo).await.unwrap();
        assert_eq!(row.id, id);
        assert_eq!(row.summary, summary);
    }

    #[tokio::test]
    async fn list_recent_summaries_uses_window() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let now = fixed_now();
        let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];

        let summary_old = SessionSummary::from_logs(
            deck.id(),
            now - chrono::Duration::days(11),
            now - chrono::Duration::days(10),
            &logs,
        )
        .unwrap();
        let summary_recent = SessionSummary::from_logs(
            deck.id(),
            now - chrono::Duration::days(3),
            now - chrono::Duration::days(2),
            &logs,
        )
        .unwrap();

        repo.append_summary(&summary_old).await.unwrap();
        repo.append_summary(&summary_recent).await.unwrap();

        let listed = SessionQueries::list_recent_summaries(deck.id(), &repo, now, 7, 10)
            .await
            .unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].completed_at(), summary_recent.completed_at());
    }
}

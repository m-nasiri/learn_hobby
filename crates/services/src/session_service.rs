use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashSet;
use std::fmt;
use thiserror::Error;

use learn_core::{
    model::{Card, CardId, Deck, DeckId, ReviewGrade, SessionSummary, SessionSummaryError},
    scheduler::Scheduler,
};
use storage::repository::{
    CardRepository, DeckRepository, ReviewPersistence, SessionSummaryRepository, SessionSummaryRow,
    StorageError,
};

use crate::review_service::{ReviewResult, ReviewService, ReviewServiceError};

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SessionError {
    #[error("no cards available for session")]
    Empty,
    #[error("session already completed")]
    Completed,
    #[error("not enough grades to complete session")]
    InsufficientGrades,
    #[error(transparent)]
    Summary(#[from] SessionSummaryError),
    #[error(transparent)]
    Review(#[from] ReviewServiceError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

//
// ─── REVIEW RESULT WITH CARD ───────────────────────────────────────────────────
//

/// Captures the outcome of reviewing a card within a session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionReview {
    pub card_id: CardId,
    pub result: ReviewResult,
}

/// Selection result for a session build.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionPlan {
    pub cards: Vec<Card>,
    pub due_selected: usize,
    pub new_selected: usize,
    pub future_selected: usize,
}

impl SessionPlan {
    /// Total number of cards in this plan.
    #[must_use]
    pub fn total(&self) -> usize {
        self.cards.len()
    }

    /// Returns true when no cards were selected for this session.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

/// Aggregated view of session progress, useful for UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionProgress {
    pub total: usize,
    pub answered: usize,
    pub remaining: usize,
    pub is_complete: bool,
}

/// Summary of a persisted session run.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionRunSummary {
    pub total: usize,
    pub answered: usize,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub results: Vec<SessionReview>,
    pub summary_id: Option<i64>,
}

/// Builds a micro-session by picking due and new cards according to deck settings.
pub struct SessionBuilder<'a> {
    deck: &'a Deck,
    shuffle_new: bool,
}

impl<'a> SessionBuilder<'a> {
    #[must_use]
    pub fn new(deck: &'a Deck) -> Self {
        Self {
            deck,
            shuffle_new: false,
        }
    }

    /// Enable or disable shuffling among new cards before selection.
    #[must_use]
    pub fn with_shuffle_new(mut self, shuffle: bool) -> Self {
        self.shuffle_new = shuffle;
        self
    }

    /// Build a session plan from storage-provided lists of due and new cards.
    ///
    /// - `due_cards` are assumed to already be due; they are sorted by `next_review_at`.
    /// - `new_cards` are unreviewed; they are optionally shuffled.
    /// - Selection respects deck `review_limit_per_day`, `new_cards_per_day`, and `micro_session_size`.
    pub fn build(
        self,
        due_cards: impl IntoIterator<Item = Card>,
        new_cards: impl IntoIterator<Item = Card>,
    ) -> SessionPlan {
        let settings = self.deck.settings();
        let micro_cap = usize::try_from(settings.micro_session_size()).unwrap_or(usize::MAX);
        let due_cap = usize::try_from(settings.review_limit_per_day()).unwrap_or(usize::MAX);
        let new_cap = usize::try_from(settings.new_cards_per_day()).unwrap_or(usize::MAX);

        let mut due: Vec<Card> = due_cards.into_iter().collect();
        due.sort_by_key(|c| (c.next_review_at(), c.id().value()));

        let mut selected = Vec::new();

        let due_take = due_cap.min(micro_cap);
        let due_selected = due.into_iter().take(due_take).collect::<Vec<_>>();
        let due_count = due_selected.len();
        selected.extend(due_selected);

        let mut selected_ids: HashSet<_> = selected.iter().map(Card::id).collect();

        let remaining = micro_cap.saturating_sub(selected.len());
        let mut new_count = 0;
        if remaining > 0 && new_cap > 0 {
            let take = new_cap.min(remaining);
            let mut new_candidates: Vec<Card> = new_cards
                .into_iter()
                .filter(|c| !selected_ids.contains(&c.id()))
                .collect();

            if self.shuffle_new {
                let mut rng = thread_rng();
                new_candidates.as_mut_slice().shuffle(&mut rng);
            } else {
                new_candidates.sort_by_key(|c| (c.created_at(), c.id().value()));
            }

            let new_cards: Vec<Card> = new_candidates.into_iter().take(take).collect();
            new_count = new_cards.len();
            selected_ids.extend(new_cards.iter().map(Card::id));
            selected.extend(new_cards);
        }

        SessionPlan {
            cards: selected,
            due_selected: due_count,
            new_selected: new_count,
            future_selected: 0,
        }
    }
}

//
// ─── SESSION ───────────────────────────────────────────────────────────────────
//

/// In-memory micro-session for a deck.
///
/// Selects up to `micro_session_size` cards from the provided list and steps through
/// them sequentially, applying grades via `ReviewService`.
pub struct SessionService {
    deck_id: DeckId,
    cards: Vec<Card>,
    current: usize,
    results: Vec<SessionReview>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    summary_id: Option<i64>,
    review_service: ReviewService,
}

impl SessionService {
    /// Create a new session for the given deck, selecting up to `micro_session_size` cards.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are provided.
    pub fn new(
        deck: &Deck,
        mut cards: Vec<Card>,
        review_service: ReviewService,
    ) -> Result<Self, SessionError> {
        let limit = usize::try_from(deck.settings().micro_session_size()).unwrap_or(usize::MAX);
        cards.truncate(limit);

        if cards.is_empty() {
            return Err(SessionError::Empty);
        }

        let now = review_service.now();
        Ok(Self {
            deck_id: deck.id(),
            cards,
            current: 0,
            results: Vec::new(),
            started_at: now,
            completed_at: None,
            summary_id: None,
            review_service,
        })
    }

    /// Convenience constructor with a default scheduler-backed review service.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Empty` if no cards are provided.
    pub fn with_scheduler(
        deck: &Deck,
        cards: Vec<Card>,
        scheduler: Scheduler,
    ) -> Result<Self, SessionError> {
        let service = ReviewService::with_scheduler(scheduler);
        Self::new(deck, cards, service)
    }

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
        let deck = decks.get_deck(deck_id).await?;
        let settings = deck.settings();
        let due = cards
            .due_cards(deck_id, now, settings.review_limit_per_day())
            .await?;
        let new_cards = cards
            .new_cards(deck_id, settings.new_cards_per_day())
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
        review_service: ReviewService,
    ) -> Result<(Deck, SessionService), SessionError> {
        let (deck, plan) =
            Self::build_plan_from_storage(deck_id, decks, cards, now, shuffle_new).await?;
        let session = SessionService::new(&deck, plan.cards, review_service)?;
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
        review_service: ReviewService,
    ) -> Result<(Deck, SessionPlan, SessionService), SessionError> {
        let (deck, plan) =
            Self::build_plan_from_storage(deck_id, decks, cards, now, shuffle_new).await?;
        let session = SessionService::new(&deck, plan.cards.clone(), review_service)?;
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

    #[must_use]
    pub fn deck_id(&self) -> DeckId {
        self.deck_id
    }

    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    #[must_use]
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    #[must_use]
    pub fn results(&self) -> &[SessionReview] {
        &self.results
    }

    /// Total number of cards in this session.
    #[must_use]
    pub fn total_cards(&self) -> usize {
        self.cards.len()
    }

    /// Number of cards that have already been answered.
    #[must_use]
    pub fn answered_count(&self) -> usize {
        self.results.len()
    }

    /// Number of remaining cards that have not been answered yet.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.cards.len().saturating_sub(self.current)
    }

    /// Returns a summary of the current session progress.
    #[must_use]
    pub fn progress(&self) -> SessionProgress {
        SessionProgress {
            total: self.total_cards(),
            answered: self.answered_count(),
            remaining: self.remaining(),
            is_complete: self.is_complete(),
        }
    }

    #[must_use]
    pub fn current_card(&self) -> Option<&Card> {
        if self.current < self.cards.len() {
            Some(&self.cards[self.current])
        } else {
            None
        }
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.completed_at.is_some()
    }

    /// Apply a grade to the current card and advance the session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Completed` if the session is already finished.
    /// Propagates scheduler/review errors via `SessionError::Review`.
    pub fn answer_current(&mut self, grade: ReviewGrade) -> Result<&SessionReview, SessionError> {
        if self.is_complete() {
            return Err(SessionError::Completed);
        }

        let Some(card) = self.cards.get_mut(self.current) else {
            return Err(SessionError::Completed);
        };

        let reviewed_at = self.review_service.now();
        let result = self.review_service.review_card(card, grade, reviewed_at)?;

        self.results.push(SessionReview {
            card_id: card.id(),
            result,
        });

        self.current += 1;
        if self.current >= self.cards.len() {
            self.completed_at = Some(reviewed_at);
        }

        self.results.last().ok_or(SessionError::Completed)
    }

    /// Apply a grade to the current card, persist the update, and advance the session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Completed` if the session is finished.
    /// Returns `SessionError::Review` for scheduler failures.
    /// Returns `SessionError::Storage` if persistence fails.
    pub async fn answer_current_persisted(
        &mut self,
        grade: ReviewGrade,
        reviews: &dyn ReviewPersistence,
        summaries: &dyn SessionSummaryRepository,
    ) -> Result<&SessionReview, SessionError> {
        self.answer_current_persisted_with_log_id(grade, reviews, summaries)
            .await?;
        self.results.last().ok_or(SessionError::Completed)
    }

    /// Apply a grade to the current card, persist the update, and return the log ID.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::Completed` if the session is finished.
    /// Returns `SessionError::Review` for scheduler failures.
    /// Returns `SessionError::Storage` if persistence fails.
    pub async fn answer_current_persisted_with_log_id(
        &mut self,
        grade: ReviewGrade,
        reviews: &dyn ReviewPersistence,
        summaries: &dyn SessionSummaryRepository,
    ) -> Result<(SessionReview, i64), SessionError> {
        if self.is_complete() {
            return Err(SessionError::Completed);
        }

        let Some(card) = self.cards.get_mut(self.current) else {
            return Err(SessionError::Completed);
        };

        let reviewed_at = self.review_service.now();

        // ReviewService owns persistence orchestration (including atomic card+log persistence).
        let (result, log_id) = self
            .review_service
            .review_card_persisted(card, grade, reviewed_at, reviews)
            .await?;

        let review = SessionReview {
            card_id: card.id(),
            result,
        };
        self.results.push(review.clone());

        self.current += 1;
        if self.current >= self.cards.len() {
            self.completed_at = Some(reviewed_at);
            if self.summary_id.is_none() {
                let summary = self.build_summary()?;
                let id = summaries.append_summary(&summary).await?;
                self.summary_id = Some(id);
            }
        }

        Ok((review, log_id))
    }

    /// Run the entire session with provided grades and persist each review.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InsufficientGrades` if the session is not completed
    /// after consuming all grades.
    pub async fn run_persisted(
        &mut self,
        grades: impl IntoIterator<Item = ReviewGrade>,
        reviews: &dyn ReviewPersistence,
        summaries: &dyn SessionSummaryRepository,
    ) -> Result<SessionRunSummary, SessionError> {
        for grade in grades {
            if self.is_complete() {
                break;
            }
            self.answer_current_persisted(grade, reviews, summaries)
                .await?;
        }

        if !self.is_complete() {
            return Err(SessionError::InsufficientGrades);
        }

        let completed_at = self.completed_at.ok_or(SessionError::Completed)?;
        Ok(SessionRunSummary {
            total: self.total_cards(),
            answered: self.answered_count(),
            started_at: self.started_at,
            completed_at,
            results: self.results.clone(),
            summary_id: self.summary_id,
        })
    }

    /// Run the entire session with provided grades from a `Vec`.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InsufficientGrades` if the session is not completed.
    pub async fn run_persisted_with_grades(
        &mut self,
        grades: Vec<ReviewGrade>,
        reviews: &dyn ReviewPersistence,
        summaries: &dyn SessionSummaryRepository,
    ) -> Result<SessionRunSummary, SessionError> {
        self.run_persisted(grades, reviews, summaries).await
    }

    /// Run the session and return both summary and persisted log IDs.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InsufficientGrades` if the session is not completed.
    pub async fn run_persisted_with_log_ids(
        &mut self,
        grades: impl IntoIterator<Item = ReviewGrade>,
        reviews: &dyn ReviewPersistence,
        summaries: &dyn SessionSummaryRepository,
    ) -> Result<(SessionRunSummary, Vec<i64>), SessionError> {
        let mut log_ids = Vec::new();
        for grade in grades {
            if self.is_complete() {
                break;
            }
            let (_review, log_id) = self
                .answer_current_persisted_with_log_id(grade, reviews, summaries)
                .await?;
            log_ids.push(log_id);
        }

        if !self.is_complete() {
            return Err(SessionError::InsufficientGrades);
        }

        let completed_at = self.completed_at.ok_or(SessionError::Completed)?;
        let summary = SessionRunSummary {
            total: self.total_cards(),
            answered: self.answered_count(),
            started_at: self.started_at,
            completed_at,
            results: self.results.clone(),
            summary_id: self.summary_id,
        };

        Ok((summary, log_ids))
    }

    fn build_summary(&self) -> Result<SessionSummary, SessionError> {
        let completed_at = self.completed_at.ok_or(SessionError::Completed)?;
        let logs: Vec<_> = self
            .results
            .iter()
            .map(|review| review.result.applied.log.clone())
            .collect();
        Ok(SessionSummary::from_logs(
            self.deck_id,
            self.started_at,
            completed_at,
            &logs,
        )?)
    }
}

impl fmt::Debug for SessionService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionService")
            .field("deck_id", &self.deck_id)
            .field("cards_len", &self.cards.len())
            .field("current", &self.current)
            .field("results_len", &self.results.len())
            .field("started_at", &self.started_at)
            .field("completed_at", &self.completed_at)
            .finish_non_exhaustive()
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::Clock;
    use learn_core::model::{CardPhase, DeckId, ReviewLog, content::ContentDraft};
    use learn_core::scheduler::Scheduler;
    use learn_core::time::fixed_now;
    use storage::repository::{
        CardRepository, DeckRepository, InMemoryRepository, ReviewLogRepository,
        SessionSummaryRepository,
    };

    fn build_card(id: u64) -> Card {
        let prompt = ContentDraft::text_only("Q")
            .validate(fixed_now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("A")
            .validate(fixed_now(), None, None)
            .unwrap();
        let now = fixed_now();
        Card::new(
            learn_core::model::CardId::new(id),
            DeckId::new(1),
            prompt,
            answer,
            now,
            now,
        )
        .unwrap()
    }

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

    fn build_due_card(id: u64, reviewed_days_ago: i64) -> Card {
        let mut card = build_card(id);
        let scheduler = Scheduler::new().unwrap();
        let reviewed_at = fixed_now() - chrono::Duration::days(reviewed_days_ago);
        let applied = scheduler
            .apply_review(card.id(), None, ReviewGrade::Good, reviewed_at, 0.0)
            .unwrap();
        card.apply_review(&applied.outcome, reviewed_at);
        card
    }

    #[test]
    fn session_honors_micro_session_size() {
        let deck = build_deck();
        let cards = vec![build_card(1), build_card(2), build_card(3)];
        let session = SessionService::new(&deck, cards, ReviewService::new().unwrap()).unwrap();

        let expected = deck.settings().micro_session_size().min(3) as usize;
        assert_eq!(session.cards.len(), expected);
    }

    #[test]
    fn builder_prioritizes_due_and_limits_new() {
        let deck = build_deck();
        let due = build_due_card(1, 2);
        let new1 = build_card(2);
        let new2 = build_card(3);

        let plan =
            SessionBuilder::new(&deck).build(vec![due.clone()], vec![new1.clone(), new2.clone()]);

        assert_eq!(plan.due_selected, 1);
        assert!(plan.cards.iter().any(|c| c.id() == due.id()));
        assert!(plan.new_selected <= deck.settings().new_cards_per_day() as usize);
        assert!(plan.cards.len() <= deck.settings().micro_session_size() as usize);
        assert_eq!(plan.future_selected, 0);
    }

    #[test]
    fn builder_caps_micro_session_size() {
        let mut due_cards = Vec::new();
        let mut new_cards = Vec::new();
        for i in 0..10 {
            if i % 2 == 0 {
                due_cards.push(build_due_card(i, 2));
            } else {
                new_cards.push(build_card(i));
            }
        }
        let deck = build_deck();
        let plan = SessionBuilder::new(&deck).build(due_cards, new_cards);
        assert!(plan.cards.len() <= deck.settings().micro_session_size() as usize);
    }

    #[test]
    fn empty_session_returns_error() {
        let deck = build_deck();
        let service = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(fixed_now()));
        let err = SessionService::new(&deck, Vec::new(), service).unwrap_err();
        assert!(matches!(err, SessionError::Empty));
    }

    #[test]
    fn session_advances_and_completes() {
        let deck = build_deck();
        let review_service = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(fixed_now()));
        let mut session =
            SessionService::new(&deck, vec![build_card(1), build_card(2)], review_service).unwrap();

        assert!(!session.is_complete());
        let first_card_id = session.current_card().unwrap().id();
        let res1 = session.answer_current(ReviewGrade::Good).unwrap();
        assert_eq!(res1.card_id, first_card_id);
        assert_eq!(session.results.len(), 1);
        assert!(!session.is_complete());

        let second_card_id = session.current_card().unwrap().id();
        let res2 = session.answer_current(ReviewGrade::Hard).unwrap();
        assert_eq!(res2.card_id, second_card_id);
        assert!(session.is_complete());
        assert!(session.completed_at().is_some());
        assert_eq!(session.completed_at(), Some(fixed_now()));
    }

    #[test]
    fn integration_session_runs_with_review_logs_and_phase_updates() {
        let deck = build_deck();
        let review_service = ReviewService::new()
            .unwrap()
            .with_clock(Clock::fixed(fixed_now()));
        let mut session =
            SessionService::new(&deck, vec![build_card(1), build_card(2)], review_service).unwrap();

        session.answer_current(ReviewGrade::Good).unwrap();
        let first = session.results.last().unwrap();
        assert_eq!(first.card_id, session.results[0].card_id);
        assert_eq!(first.result.applied.log.grade, ReviewGrade::Good);
        assert_eq!(session.cards[0].review_count(), 1);
        assert_eq!(session.cards[0].phase(), CardPhase::Learning);

        session.answer_current(ReviewGrade::Hard).unwrap();
        let second = session.results.last().unwrap();
        assert_eq!(second.card_id, session.results[1].card_id);
        assert_eq!(second.result.applied.log.grade, ReviewGrade::Hard);
        assert_eq!(session.cards[1].review_count(), 1);
        assert_eq!(session.cards[1].phase(), CardPhase::Learning);

        assert!(session.is_complete());
        assert_eq!(session.results.len(), 2);
    }

    #[tokio::test]
    async fn persisted_session_updates_storage() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card1 = build_card(1);
        let card2 = build_card(2);
        repo.upsert_card(&card1).await.unwrap();
        repo.upsert_card(&card2).await.unwrap();

        let now = fixed_now();
        let (deck, plan) =
            SessionService::build_plan_from_storage(deck.id(), &repo, &repo, now, false)
                .await
                .unwrap();

        let review_service = ReviewService::new().unwrap().with_clock(Clock::fixed(now));
        let mut session = SessionService::new(&deck, plan.cards, review_service).unwrap();

        session
            .answer_current_persisted(ReviewGrade::Good, &repo, &repo)
            .await
            .unwrap();
        session
            .answer_current_persisted(ReviewGrade::Hard, &repo, &repo)
            .await
            .unwrap();

        let logs1 = repo.logs_for_card(deck.id(), CardId::new(1)).await.unwrap();
        let logs2 = repo.logs_for_card(deck.id(), CardId::new(2)).await.unwrap();
        assert_eq!(logs1.len(), 1);
        assert_eq!(logs2.len(), 1);
        let summary_id = session.summary_id.expect("summary persisted");
        let summary = repo.get_summary(summary_id).await.unwrap();
        assert_eq!(summary.deck_id(), deck.id());
        assert!(session.is_complete());
    }

    #[tokio::test]
    async fn start_from_storage_builds_session() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card1 = build_card(1);
        repo.upsert_card(&card1).await.unwrap();

        let now = fixed_now();
        let review_service = ReviewService::new().unwrap().with_clock(Clock::fixed(now));
        let (loaded, session) =
            SessionService::start_from_storage(deck.id(), &repo, &repo, now, false, review_service)
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
        let review_service = ReviewService::new().unwrap().with_clock(Clock::fixed(now));
        let (loaded, plan, session) = SessionService::start_from_storage_with_plan(
            deck.id(),
            &repo,
            &repo,
            now,
            false,
            review_service,
        )
        .await
        .unwrap();

        assert_eq!(loaded.id(), deck.id());
        assert_eq!(plan.total(), session.total_cards());
        assert!(plan.total() > 0);
    }

    #[tokio::test]
    async fn list_summaries_returns_recent_first() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let now = fixed_now();
        let logs = vec![ReviewLog::new(CardId::new(1), ReviewGrade::Good, now)];

        let summary1 = SessionSummary::from_logs(deck.id(), now, now, &logs).unwrap();
        let summary2 =
            SessionSummary::from_logs(deck.id(), now, now + chrono::Duration::days(1), &logs)
                .unwrap();

        let id1 = repo.append_summary(&summary1).await.unwrap();
        let id2 = repo.append_summary(&summary2).await.unwrap();
        assert_ne!(id1, id2);

        let listed = SessionService::list_summaries(
            deck.id(),
            &repo,
            Some(now),
            Some(now + chrono::Duration::days(1)),
            10,
        )
        .await
        .unwrap();

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].completed_at(), summary2.completed_at());
        assert_eq!(listed[1].completed_at(), summary1.completed_at());

        let rows = SessionService::list_summary_rows(
            deck.id(),
            &repo,
            Some(now),
            Some(now + chrono::Duration::days(1)),
            10,
        )
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].summary.completed_at(), summary2.completed_at());
        assert_eq!(rows[1].summary.completed_at(), summary1.completed_at());
        assert!(rows[0].id != rows[1].id);
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

        let row = SessionService::get_summary_row(id, &repo).await.unwrap();
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

        let listed = SessionService::list_recent_summaries(deck.id(), &repo, now, 7, 10)
            .await
            .unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].completed_at(), summary_recent.completed_at());
    }

    #[tokio::test]
    async fn run_persisted_completes_and_logs() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card1 = build_card(1);
        let card2 = build_card(2);
        repo.upsert_card(&card1).await.unwrap();
        repo.upsert_card(&card2).await.unwrap();

        let now = fixed_now();
        let review_service = ReviewService::new().unwrap().with_clock(Clock::fixed(now));
        let (deck, plan, mut session) = SessionService::start_from_storage_with_plan(
            deck.id(),
            &repo,
            &repo,
            now,
            false,
            review_service,
        )
        .await
        .unwrap();

        let summary = session
            .run_persisted([ReviewGrade::Good, ReviewGrade::Hard], &repo, &repo)
            .await
            .unwrap();

        assert_eq!(summary.total, plan.total());
        assert_eq!(summary.answered, plan.total());
        assert!(summary.completed_at >= summary.started_at);
        let summary_id = summary.summary_id.expect("summary persisted");
        let stored = repo.get_summary(summary_id).await.unwrap();
        assert_eq!(stored.total_reviews(), summary.total as u32);

        let logs1 = repo.logs_for_card(deck.id(), CardId::new(1)).await.unwrap();
        let logs2 = repo.logs_for_card(deck.id(), CardId::new(2)).await.unwrap();
        assert_eq!(logs1.len(), 1);
        assert_eq!(logs2.len(), 1);
    }

    #[tokio::test]
    async fn run_persisted_fails_with_insufficient_grades() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card1 = build_card(1);
        let card2 = build_card(2);
        repo.upsert_card(&card1).await.unwrap();
        repo.upsert_card(&card2).await.unwrap();

        let now = fixed_now();
        let review_service = ReviewService::new().unwrap().with_clock(Clock::fixed(now));
        let (_deck, _plan, mut session) = SessionService::start_from_storage_with_plan(
            deck.id(),
            &repo,
            &repo,
            now,
            false,
            review_service,
        )
        .await
        .unwrap();

        let err = session
            .run_persisted([ReviewGrade::Good], &repo, &repo)
            .await
            .unwrap_err();

        assert!(matches!(err, SessionError::InsufficientGrades));
    }

    #[tokio::test]
    async fn run_persisted_with_log_ids_returns_log_ids() {
        let repo = InMemoryRepository::new();
        let deck = build_deck();
        repo.upsert_deck(&deck).await.unwrap();

        let card1 = build_card(1);
        let card2 = build_card(2);
        repo.upsert_card(&card1).await.unwrap();
        repo.upsert_card(&card2).await.unwrap();

        let now = fixed_now();
        let review_service = ReviewService::new().unwrap().with_clock(Clock::fixed(now));
        let (_deck, _plan, mut session) = SessionService::start_from_storage_with_plan(
            deck.id(),
            &repo,
            &repo,
            now,
            false,
            review_service,
        )
        .await
        .unwrap();

        let (summary, log_ids) = session
            .run_persisted_with_log_ids([ReviewGrade::Good, ReviewGrade::Hard], &repo, &repo)
            .await
            .unwrap();

        assert_eq!(summary.answered, summary.total);
        assert_eq!(log_ids.len(), summary.total);
    }
}

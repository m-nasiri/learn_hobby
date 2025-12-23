use chrono::{DateTime, Utc};
use learn_core::{
    model::{Card, CardId, DeckId, ReviewGrade},
    scheduler::{AppliedReview, MemoryState, Scheduler},
    time::Clock,
};
use storage::repository::{CardRepository, ReviewLogRecord, ReviewPersistence, StorageError};

const SECONDS_PER_DAY: f64 = 86_400.0;
use crate::error::ReviewServiceError;

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewResult {
    pub applied: AppliedReview,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistedReview {
    pub card: Card,
    pub log_id: i64,
    pub result: ReviewResult,
}

#[must_use]
pub fn compute_elapsed_days(
    last_review_at: Option<DateTime<Utc>>,
    reviewed_at: DateTime<Utc>,
) -> f64 {
    match last_review_at {
        Some(last) => {
            let seconds = reviewed_at.signed_duration_since(last).num_seconds();
            #[allow(clippy::cast_precision_loss)]
            let seconds_f = seconds as f64;
            seconds_f / SECONDS_PER_DAY
        }
        None => 0.0,
    }
}

pub struct ReviewService {
    clock: Clock,
    scheduler: Scheduler,
}

impl ReviewService {
    /// Create a new review service using default scheduler and clock.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` if scheduler initialization fails.
    pub fn new() -> Result<Self, ReviewServiceError> {
        Ok(Self {
            clock: Clock::default(),
            scheduler: Scheduler::new()?,
        })
    }

    #[must_use]
    pub fn with_scheduler(scheduler: Scheduler) -> Self {
        Self {
            clock: Clock::default(),
            scheduler,
        }
    }

    #[must_use]
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Apply a grade to a card and return the scheduler output.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` on scheduler failures.
    pub fn review_card(
        &self,
        card: &mut Card,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
    ) -> Result<ReviewResult, ReviewServiceError> {
        let previous_state: Option<MemoryState> = card.memory_state();
        let elapsed_days = compute_elapsed_days(card.last_review_at(), reviewed_at);

        let applied = self.scheduler.apply_review(
            card.id(),
            previous_state.as_ref(),
            grade,
            reviewed_at,
            elapsed_days,
        )?;

        card.apply_review_with_phase(grade, &applied.outcome, reviewed_at);

        Ok(ReviewResult { applied })
    }

    /// Apply a grade, persist the updated card and review log atomically.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Scheduler` on scheduler failures.
    /// Returns `ReviewServiceError::Storage` on persistence failures.
    pub async fn review_card_persisted(
        &self,
        card: &mut Card,
        grade: ReviewGrade,
        reviewed_at: DateTime<Utc>,
        reviews: &dyn ReviewPersistence,
    ) -> Result<(ReviewResult, i64), ReviewServiceError> {
        let original = card.clone();
        let result = self.review_card(card, grade, reviewed_at)?;

        let record = ReviewLogRecord::from_applied(
            card.deck_id(),
            &result.applied.log,
            &result.applied.outcome,
        );

        match reviews.apply_review(card, record).await {
            Ok(id) => Ok((result, id)),
            Err(err) => {
                *card = original;
                Err(err.into())
            }
        }
    }

    /// Load a card by ID, apply a grade, and persist the update atomically.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Storage` if the card is missing or persistence fails.
    /// Returns `ReviewServiceError::Scheduler` on scheduler failures.
    pub async fn review_card_persisted_by_id(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        cards: &dyn CardRepository,
        reviews: &dyn ReviewPersistence,
        grade: ReviewGrade,
    ) -> Result<PersistedReview, ReviewServiceError> {
        let mut card = cards
            .get_cards(deck_id, &[card_id])
            .await?
            .into_iter()
            .next()
            .ok_or(StorageError::NotFound)?;

        let reviewed_at = self.now();
        let (result, log_id) = self
            .review_card_persisted(&mut card, grade, reviewed_at, reviews)
            .await?;

        Ok(PersistedReview {
            card,
            log_id,
            result,
        })
    }

    /// Persist a batch of already-applied reviews.
    ///
    /// # Errors
    ///
    /// Returns `ReviewServiceError::Storage` if any persistence call fails.
    pub async fn persist_applied_reviews(
        &self,
        deck_id: DeckId,
        items: impl IntoIterator<Item = (Card, AppliedReview)>,
        reviews: &dyn ReviewPersistence,
    ) -> Result<Vec<i64>, ReviewServiceError> {
        let mut ids = Vec::new();

        for (card, applied) in items {
            let record = ReviewLogRecord::from_applied(deck_id, &applied.log, &applied.outcome);
            let id = reviews.apply_review(&card, record).await?;
            ids.push(id);
        }

        Ok(ids)
    }
}

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use learn_core::model::{Card, CardError, CardId, CardPhase, ContentDraft, DeckId, Tag, TagName};
use storage::repository::{CardRepository, NewCardRecord};

use crate::error::CardServiceError;
use crate::Clock;

/// Orchestrates card creation and persistence.
#[derive(Clone)]
pub struct CardService {
    clock: Clock,
    cards: Arc<dyn CardRepository>,
}

/// Aggregate counts for a deck in the practice view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeckPracticeStats {
    pub total: u32,
    pub due: u32,
    pub new: u32,
}

/// Aggregate counts for a deck with identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeckPracticeStatsRow {
    pub deck_id: DeckId,
    pub stats: DeckPracticeStats,
}

/// Aggregate counts for a tag scoped to a deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagPracticeStats {
    pub name: TagName,
    pub total: u32,
    pub due: u32,
    pub new: u32,
}

fn dedup_tags(tags: &[TagName]) -> Vec<TagName> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for tag in tags {
        if seen.insert(tag.as_str().to_string()) {
            out.push(tag.clone());
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CardListSort {
    /// Newest first (`created_at` DESC).
    Recent,
    /// Oldest first (`created_at` ASC).
    Created,
    /// Alphabetical by prompt text.
    Alpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CardListFilter {
    /// No filtering.
    All,
    /// Cards due within the next 24 hours (reviewed cards only).
    DueSoon,
}

impl CardService {
    #[must_use]
    pub fn new(clock: Clock, cards: Arc<dyn CardRepository>) -> Self {
        Self { clock, cards }
    }

    /// Create a new card from validated drafts and persist it.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Card` for validation failures.
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn create_card(
        &self,
        deck_id: DeckId,
        prompt: ContentDraft,
        answer: ContentDraft,
    ) -> Result<CardId, CardServiceError> {
        self.create_card_with_tags(deck_id, prompt, answer, &[]).await
    }

    /// Create a new card with tags and persist it.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Card` for validation failures.
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn create_card_with_tags(
        &self,
        deck_id: DeckId,
        prompt: ContentDraft,
        answer: ContentDraft,
        tag_names: &[TagName],
    ) -> Result<CardId, CardServiceError> {
        let now = self.clock.now();
        let prompt = prompt
            .validate(now, None, None)
            .map_err(CardError::InvalidPrompt)?;
        let answer = answer
            .validate(now, None, None)
            .map_err(CardError::InvalidAnswer)?;

        let record = NewCardRecord {
            deck_id,
            prompt_text: prompt.text().to_owned(),
            prompt_media_id: prompt.media_id().map(|m| m.value()),
            answer_text: answer.text().to_owned(),
            answer_media_id: answer.media_id().map(|m| m.value()),
            phase: CardPhase::New,
            created_at: now,
            next_review_at: now,
            last_review_at: None,
            review_count: 0,
            stability: None,
            difficulty: None,
        };

        let card_id = self.cards.insert_new_card(record).await?;
        if !tag_names.is_empty() {
            let tags = dedup_tags(tag_names);
            self.cards.set_tags_for_card(deck_id, card_id, &tags).await?;
        }
        Ok(card_id)
    }

    /// Count cards created today for a deck (UTC day).
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn new_cards_created_today(
        &self,
        deck_id: DeckId,
    ) -> Result<u32, CardServiceError> {
        let now = self.clock.now();
        let (start, end) = day_bounds(now);
        let count = self
            .cards
            .count_cards_created_between(deck_id, start, end)
            .await?;
        Ok(count)
    }

    /// Persist an existing card update.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn save_card(&self, card: &Card) -> Result<(), CardServiceError> {
        self.cards.upsert_card(card).await?;
        Ok(())
    }

    /// List cards for a deck up to the given limit.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn list_cards(
        &self,
        deck_id: DeckId,
        limit: u32,
    ) -> Result<Vec<Card>, CardServiceError> {
        let cards = self.cards.list_cards(deck_id, limit).await?;
        Ok(cards)
    }

    /// Count cards currently in a mistake/relearning state.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` on persistence failures.
    pub async fn mistakes_count(&self, deck_id: DeckId) -> Result<u32, CardServiceError> {
        let count = self.cards.mistakes_count(deck_id).await?;
        Ok(count)
    }

    /// Reset learning state for all cards in a deck.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` on persistence failures.
    pub async fn reset_deck_learning(&self, deck_id: DeckId) -> Result<u64, CardServiceError> {
        let now = self.clock.now();
        let updated = self.cards.reset_deck_learning(deck_id, now).await?;
        Ok(updated)
    }

    /// Compute practice-ready card counts for a deck.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn deck_practice_stats(
        &self,
        deck_id: DeckId,
    ) -> Result<DeckPracticeStats, CardServiceError> {
        let now = self.clock.now();
        let counts = self.cards.deck_practice_counts(deck_id, now).await?;
        Ok(DeckPracticeStats {
            total: counts.total,
            due: counts.due,
            new: counts.new,
        })
    }

    /// Compute practice-ready card counts for multiple decks.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn list_deck_practice_stats(
        &self,
        deck_ids: &[DeckId],
    ) -> Result<Vec<DeckPracticeStatsRow>, CardServiceError> {
        let now = self.clock.now();
        let rows = self.cards.list_deck_practice_counts(deck_ids, now).await?;
        Ok(rows
            .into_iter()
            .map(|row| DeckPracticeStatsRow {
                deck_id: row.deck_id,
                stats: DeckPracticeStats {
                    total: row.counts.total,
                    due: row.counts.due,
                    new: row.counts.new,
                },
            })
            .collect())
    }

    /// Compute practice-ready tag counts for a deck.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn list_tag_practice_stats(
        &self,
        deck_id: DeckId,
    ) -> Result<Vec<TagPracticeStats>, CardServiceError> {
        let now = self.clock.now();
        let counts = self
            .cards
            .list_tag_practice_counts(deck_id, now)
            .await?;
        let out = counts
            .into_iter()
            .map(|item| TagPracticeStats {
                name: item.name,
                total: item.total,
                due: item.due,
                new: item.new,
            })
            .collect();
        Ok(out)
    }

    /// List cards for a deck with sorting and filtering.
    ///
    /// If `tag_names` is non-empty, only cards with at least one of the tags are returned.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn list_cards_filtered(
        &self,
        deck_id: DeckId,
        limit: u32,
        sort: CardListSort,
        filter: CardListFilter,
        tag_names: &[TagName],
    ) -> Result<Vec<Card>, CardServiceError> {
        let mut cards = if tag_names.is_empty() {
            self.cards.list_cards(deck_id, limit).await?
        } else {
            self.cards.list_cards_by_tags(deck_id, tag_names).await?
        };

        if matches!(filter, CardListFilter::DueSoon) {
            let now = self.clock.now();
            let cutoff = now + Duration::hours(24);
            cards.retain(|card| card.review_count() > 0 && card.next_review_at() <= cutoff);
        }

        match sort {
            CardListSort::Recent => {
                cards.sort_by(|a, b| {
                    b.created_at()
                        .cmp(&a.created_at())
                        .then_with(|| b.id().value().cmp(&a.id().value()))
                });
            }
            CardListSort::Created => {
                cards.sort_by(|a, b| {
                    a.created_at()
                        .cmp(&b.created_at())
                        .then_with(|| a.id().value().cmp(&b.id().value()))
                });
            }
            CardListSort::Alpha => {
                cards.sort_by(|left, right| {
                    let left_key = left.prompt().text().to_lowercase();
                    let right_key = right.prompt().text().to_lowercase();
                    left_key
                        .cmp(&right_key)
                        .then_with(|| left.id().value().cmp(&right.id().value()))
                });
            }
        }

        cards.truncate(limit as usize);
        Ok(cards)
    }

    /// List tags for a deck.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn list_tags_for_deck(&self, deck_id: DeckId) -> Result<Vec<Tag>, CardServiceError> {
        let tags = self.cards.list_tags_for_deck(deck_id).await?;
        Ok(tags)
    }

    /// List tags for a card.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn list_tags_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
    ) -> Result<Vec<Tag>, CardServiceError> {
        let tags = self.cards.list_tags_for_card(deck_id, card_id).await?;
        Ok(tags)
    }

    /// Replace tags for a card.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn set_tags_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        tag_names: &[TagName],
    ) -> Result<Vec<Tag>, CardServiceError> {
        let tags = dedup_tags(tag_names);
        let tags = self.cards.set_tags_for_card(deck_id, card_id, &tags).await?;
        Ok(tags)
    }

    /// Returns true if a card with the given prompt exists in the deck.
    ///
    /// Comparison is normalized (trimmed, case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if repository access fails.
    pub async fn prompt_exists(
        &self,
        deck_id: DeckId,
        prompt_text: &str,
        exclude: Option<CardId>,
    ) -> Result<bool, CardServiceError> {
        if prompt_text.trim().is_empty() {
            return Ok(false);
        }

        let exists = self
            .cards
            .prompt_exists(deck_id, prompt_text, exclude)
            .await?;
        Ok(exists)
    }

    /// Update a card's prompt/answer content while preserving scheduling state.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Card` for validation failures.
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn update_card_content(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        prompt: ContentDraft,
        answer: ContentDraft,
    ) -> Result<(), CardServiceError> {
        let now = self.clock.now();
        let prompt = prompt
            .validate(now, None, None)
            .map_err(CardError::InvalidPrompt)?;
        let answer = answer
            .validate(now, None, None)
            .map_err(CardError::InvalidAnswer)?;

        let mut cards = self.cards.get_cards(deck_id, &[card_id]).await?;
        let Some(card) = cards.pop() else {
            return Err(storage::repository::StorageError::NotFound.into());
        };

        let (stability, difficulty) = if card.review_count() == 0 {
            (0.0, 0.0)
        } else {
            let state = card
                .memory_state()
                .ok_or_else(|| CardError::InvalidPersistedState("missing memory state".into()))?;
            (state.stability, state.difficulty)
        };

        let updated = Card::from_persisted(
            card.id(),
            card.deck_id(),
            prompt,
            answer,
            card.created_at(),
            card.next_review_at(),
            card.last_review_at(),
            card.phase(),
            card.review_count(),
            stability,
            difficulty,
        )?;

        self.cards.upsert_card(&updated).await?;
        Ok(())
    }

    /// Update a card's prompt/answer content and tags while preserving scheduling state.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Card` for validation failures.
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn update_card_content_with_tags(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        prompt: ContentDraft,
        answer: ContentDraft,
        tag_names: &[TagName],
    ) -> Result<(), CardServiceError> {
        self.update_card_content(deck_id, card_id, prompt, answer)
            .await?;
        self.set_tags_for_card(deck_id, card_id, tag_names).await?;
        Ok(())
    }

    /// Delete a card and any associated persisted history.
    ///
    /// # Errors
    ///
    /// Returns `CardServiceError::Storage` if persistence fails.
    pub async fn delete_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
    ) -> Result<(), CardServiceError> {
        self.cards.delete_card(deck_id, card_id).await?;
        Ok(())
    }
}

fn day_bounds(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let date = now.date_naive();
    let start = DateTime::<Utc>::from_naive_utc_and_offset(
        date.and_hms_opt(0, 0, 0).unwrap(),
        Utc,
    );
    let end = start + Duration::days(1);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Duration;
    use learn_core::model::{Card, CardId, ContentDraft, DeckId, ReviewOutcome, TagName};
    use learn_core::time::fixed_now;
    use storage::repository::{CardRepository, InMemoryRepository};

    fn build_content(text: &str, now: chrono::DateTime<chrono::Utc>) -> learn_core::model::Content {
        ContentDraft::text_only(text)
            .validate(now, None, None)
            .expect("valid content")
    }

    fn build_card(id: u64, deck_id: DeckId, now: chrono::DateTime<chrono::Utc>) -> Card {
        let prompt = build_content("Q", now);
        let answer = build_content("A", now);
        Card::new(CardId::new(id), deck_id, prompt, answer, now, now).expect("card")
    }

    #[tokio::test]
    async fn deck_practice_stats_counts_due_new_total() {
        let repo = InMemoryRepository::new();
        let deck_id = DeckId::new(1);
        let now = fixed_now();

        let mut due_card = build_card(1, deck_id, now);
        due_card.apply_review(
            &ReviewOutcome::new(
                now - Duration::hours(2),
                1.0,
                1.0,
                1.0,
                1.0,
            ),
            now - Duration::days(1),
        );
        repo.upsert_card(&due_card).await.expect("due card");

        let mut future_card = build_card(2, deck_id, now);
        future_card.apply_review(
            &ReviewOutcome::new(
                now + Duration::hours(5),
                1.0,
                1.0,
                1.0,
                1.0,
            ),
            now - Duration::days(1),
        );
        repo.upsert_card(&future_card).await.expect("future card");

        let new_card = build_card(3, deck_id, now);
        repo.upsert_card(&new_card).await.expect("new card");

        let service = CardService::new(Clock::Fixed(now), Arc::new(repo));
        let stats = service
            .deck_practice_stats(deck_id)
            .await
            .expect("stats");

        assert_eq!(stats.total, 3);
        assert_eq!(stats.new, 1);
        assert_eq!(stats.due, 1);
    }

    #[tokio::test]
    async fn tag_practice_stats_counts_due_new_total() {
        let repo = InMemoryRepository::new();
        let deck_id = DeckId::new(1);
        let now = fixed_now();

        let mut due_card = build_card(1, deck_id, now);
        due_card.apply_review(
            &ReviewOutcome::new(
                now - Duration::hours(1),
                1.0,
                1.0,
                1.0,
                1.0,
            ),
            now - Duration::days(1),
        );
        repo.upsert_card(&due_card).await.expect("due card");

        let new_card = build_card(2, deck_id, now);
        repo.upsert_card(&new_card).await.expect("new card");

        let tag = TagName::new("Language").expect("tag");
        repo.set_tags_for_card(deck_id, due_card.id(), &[tag.clone()])
            .await
            .expect("tag due");
        repo.set_tags_for_card(deck_id, new_card.id(), &[tag.clone()])
            .await
            .expect("tag new");

        let service = CardService::new(Clock::Fixed(now), Arc::new(repo));
        let stats = service
            .list_tag_practice_stats(deck_id)
            .await
            .expect("tag stats");

        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].name, tag);
        assert_eq!(stats[0].total, 2);
        assert_eq!(stats[0].new, 1);
        assert_eq!(stats[0].due, 1);
    }

    #[tokio::test]
    async fn reset_deck_learning_clears_review_state() {
        let repo = InMemoryRepository::new();
        let deck_id = DeckId::new(1);
        let now = fixed_now();

        let mut reviewed = build_card(1, deck_id, now);
        reviewed.apply_review(
            &ReviewOutcome::new(
                now + Duration::hours(1),
                2.0,
                3.0,
                2.5,
                2.8,
            ),
            now - Duration::hours(2),
        );
        repo.upsert_card(&reviewed).await.expect("reviewed card");

        let service = CardService::new(Clock::Fixed(now), Arc::new(repo));
        service
            .reset_deck_learning(deck_id)
            .await
            .expect("reset");

        let cards = service
            .list_cards(deck_id, 10)
            .await
            .expect("list cards");
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!(card.phase(), learn_core::model::CardPhase::New);
        assert_eq!(card.review_count(), 0);
        assert_eq!(card.last_review_at(), None);
        assert_eq!(card.next_review_at(), now);
    }

    #[tokio::test]
    async fn new_cards_created_today_counts_only_today() {
        let repo = InMemoryRepository::new();
        let deck_id = DeckId::new(1);
        let now = fixed_now();

        let today_card = build_card(1, deck_id, now);
        repo.upsert_card(&today_card).await.expect("today card");

        let yesterday = now - Duration::days(1);
        let old_card = build_card(2, deck_id, yesterday);
        repo.upsert_card(&old_card).await.expect("old card");

        let service = CardService::new(Clock::Fixed(now), Arc::new(repo));
        let count = service
            .new_cards_created_today(deck_id)
            .await
            .expect("count");

        assert_eq!(count, 1);
    }
}

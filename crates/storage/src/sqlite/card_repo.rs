use std::collections::HashMap;

use learn_core::model::{Card, CardId, DeckId};

use super::{
    SqliteRepository,
    mapping::{card_id_from_i64, map_card_row, media_id_to_i64},
};
use crate::repository::{CardRepository, NewCardRecord, StorageError};

#[async_trait::async_trait]
impl CardRepository for SqliteRepository {
    async fn insert_new_card(&self, card: NewCardRecord) -> Result<CardId, StorageError> {
        let deck_id = i64::try_from(card.deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;
        let prompt_media_id = card
            .prompt_media_id
            .map(i64::try_from)
            .transpose()
            .map_err(|_| StorageError::Serialization("prompt_media_id overflow".into()))?;
        let answer_media_id = card
            .answer_media_id
            .map(i64::try_from)
            .transpose()
            .map_err(|_| StorageError::Serialization("answer_media_id overflow".into()))?;

        let result = sqlx::query(
            r"
            INSERT INTO cards (
                deck_id, prompt, prompt_media_id, answer, answer_media_id,
                phase, created_at, next_review_at, last_review_at, review_count,
                stability, difficulty
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ",
        )
        .bind(deck_id)
        .bind(card.prompt_text)
        .bind(prompt_media_id)
        .bind(card.answer_text)
        .bind(answer_media_id)
        .bind(card.phase.as_str())
        .bind(card.created_at)
        .bind(card.next_review_at)
        .bind(card.last_review_at)
        .bind(i64::from(card.review_count))
        .bind(card.stability)
        .bind(card.difficulty)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let id = result.last_insert_rowid();
        card_id_from_i64(id)
    }

    async fn upsert_card(&self, card: &Card) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO cards (
                id, deck_id, prompt, prompt_media_id, answer, answer_media_id,
                phase, created_at, next_review_at, last_review_at, review_count,
                stability, difficulty
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                -- keep created_at from the original insert; only update mutable fields
                prompt = excluded.prompt,
                prompt_media_id = excluded.prompt_media_id,
                answer = excluded.answer,
                answer_media_id = excluded.answer_media_id,
                phase = excluded.phase,
                next_review_at = excluded.next_review_at,
                last_review_at = excluded.last_review_at,
                review_count = excluded.review_count,
                stability = excluded.stability,
                difficulty = excluded.difficulty
            ",
        )
        .bind(
            i64::try_from(card.id().value())
                .map_err(|_| StorageError::Serialization("card_id overflow".into()))?,
        )
        .bind(
            i64::try_from(card.deck_id().value())
                .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?,
        )
        .bind(card.prompt().text().to_owned())
        .bind(media_id_to_i64(card.prompt().media_id())?)
        .bind(card.answer().text().to_owned())
        .bind(media_id_to_i64(card.answer().media_id())?)
        .bind(card.phase().as_str())
        .bind(card.created_at())
        .bind(card.next_review_at())
        .bind(card.last_review_at())
        .bind(i64::from(card.review_count()))
        .bind(card.memory_state().map(|m| m.stability))
        .bind(card.memory_state().map(|m| m.difficulty))
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(())
    }

    async fn delete_card(&self, deck_id: DeckId, card_id: CardId) -> Result<(), StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;
        let card = i64::try_from(card_id.value())
            .map_err(|_| StorageError::Serialization("card_id overflow".into()))?;

        let result = sqlx::query(
            r"
            DELETE FROM cards
            WHERE id = ?1 AND deck_id = ?2
            ",
        )
        .bind(card)
        .bind(deck)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }

        Ok(())
    }

    async fn get_cards(&self, deck_id: DeckId, ids: &[CardId]) -> Result<Vec<Card>, StorageError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut sql = String::from(
            r"
            SELECT
                id, deck_id, prompt, prompt_media_id, answer, answer_media_id, phase, created_at,
                next_review_at, last_review_at, review_count, stability, difficulty
            FROM cards
            WHERE deck_id = ?1 AND id IN (
            ",
        );

        for i in 0..ids.len() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push('?');
            sql.push_str(&(i + 2).to_string());
        }
        sql.push_str(")\n");

        let mut q = sqlx::query(&sql).bind(
            i64::try_from(deck_id.value())
                .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?,
        );

        for id in ids {
            q = q.bind(
                i64::try_from(id.value())
                    .map_err(|_| StorageError::Serialization("card_id overflow".into()))?,
            );
        }

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut by_id: HashMap<u64, Card> = HashMap::with_capacity(rows.len());

        for row in rows {
            let card = map_card_row(&row)?;
            by_id.insert(card.id().value(), card);
        }

        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            match by_id.remove(&id.value()) {
                Some(card) => out.push(card),
                None => return Err(StorageError::NotFound),
            }
        }

        Ok(out)
    }

    async fn due_cards(
        &self,
        deck_id: DeckId,
        now: chrono::DateTime<chrono::Utc>,
        limit: u32,
    ) -> Result<Vec<Card>, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;
        let lim = i64::from(limit);

        let rows = sqlx::query(
            r"
            SELECT
                id, deck_id, prompt, prompt_media_id, answer, answer_media_id, phase, created_at,
                next_review_at, last_review_at, review_count, stability, difficulty
            FROM cards
            WHERE deck_id = ?1
              AND review_count > 0
              AND next_review_at <= ?2
            ORDER BY next_review_at ASC, id ASC
            LIMIT ?3
            ",
        )
        .bind(deck)
        .bind(now)
        .bind(lim)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut cards = Vec::with_capacity(rows.len());
        for row in rows {
            cards.push(map_card_row(&row)?);
        }
        Ok(cards)
    }

    async fn new_cards(&self, deck_id: DeckId, limit: u32) -> Result<Vec<Card>, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;
        let lim = i64::from(limit);

        let rows = sqlx::query(
            r"
            SELECT
                id, deck_id, prompt, prompt_media_id, answer, answer_media_id, phase, created_at,
                next_review_at, last_review_at, review_count, stability, difficulty
            FROM cards
            WHERE deck_id = ?1
              AND review_count = 0
            ORDER BY created_at ASC, id ASC
            LIMIT ?2
            ",
        )
        .bind(deck)
        .bind(lim)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut cards = Vec::with_capacity(rows.len());
        for row in rows {
            cards.push(map_card_row(&row)?);
        }
        Ok(cards)
    }

    async fn list_cards(&self, deck_id: DeckId, limit: u32) -> Result<Vec<Card>, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;
        let lim = i64::from(limit);

        let rows = sqlx::query(
            r"
            SELECT
                id, deck_id, prompt, prompt_media_id, answer, answer_media_id, phase, created_at,
                next_review_at, last_review_at, review_count, stability, difficulty
            FROM cards
            WHERE deck_id = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT ?2
            ",
        )
        .bind(deck)
        .bind(lim)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut cards = Vec::with_capacity(rows.len());
        for row in rows {
            cards.push(map_card_row(&row)?);
        }
        Ok(cards)
    }

    async fn prompt_exists(
        &self,
        deck_id: DeckId,
        prompt_text: &str,
        exclude: Option<CardId>,
    ) -> Result<bool, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;

        let exists = if let Some(card_id) = exclude {
            let exclude_id = i64::try_from(card_id.value())
                .map_err(|_| StorageError::Serialization("card_id overflow".into()))?;
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT EXISTS(
                    SELECT 1
                    FROM cards
                    WHERE deck_id = ?1
                      AND LOWER(TRIM(prompt)) = LOWER(TRIM(?2))
                      AND id != ?3
                )
                ",
            )
            .bind(deck)
            .bind(prompt_text)
            .bind(exclude_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?
        } else {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT EXISTS(
                    SELECT 1
                    FROM cards
                    WHERE deck_id = ?1
                      AND LOWER(TRIM(prompt)) = LOWER(TRIM(?2))
                )
                ",
            )
            .bind(deck)
            .bind(prompt_text)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?
        };

        Ok(exists != 0)
    }
}

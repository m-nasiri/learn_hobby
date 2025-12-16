use std::collections::HashMap;

use learn_core::model::{Card, CardId, DeckId};

use super::{SqliteRepository, mapping::map_card_row};
use crate::repository::{CardRepository, StorageError};

#[async_trait::async_trait]
impl CardRepository for SqliteRepository {
    async fn upsert_card(&self, card: &Card) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO cards (
                id, deck_id, prompt, answer, phase, created_at,
                next_review_at, last_review_at, review_count, stability, difficulty
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id, deck_id) DO UPDATE SET
                -- keep created_at from the original insert; only update mutable fields
                prompt = excluded.prompt,
                answer = excluded.answer,
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
                .map_err(|_| StorageError::Serialization("card id overflow".into()))?,
        )
        .bind(
            i64::try_from(card.deck_id().value())
                .map_err(|_| StorageError::Serialization("deck id overflow".into()))?,
        )
        .bind(card.prompt().text().to_owned())
        .bind(card.answer().text().to_owned())
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

    async fn get_cards(&self, deck_id: DeckId, ids: &[CardId]) -> Result<Vec<Card>, StorageError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut sql = String::from(
            r"
            SELECT
                id, deck_id, prompt, answer, phase, created_at,
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
}

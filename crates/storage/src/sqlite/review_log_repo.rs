use learn_core::model::{Card, CardId, DeckId};

use super::{
    SqliteRepository,
    mapping::{grade_to_i64, map_review_log_row, media_id_to_i64},
};
use crate::repository::{ReviewLogRecord, ReviewLogRepository, ReviewPersistence, StorageError};

fn id_i64(field: &'static str, v: u64) -> Result<i64, StorageError> {
    i64::try_from(v).map_err(|_| StorageError::Serialization(format!("{field} overflow")))
}

#[async_trait::async_trait]
impl ReviewLogRepository for SqliteRepository {
    async fn append_log(&self, log: ReviewLogRecord) -> Result<i64, StorageError> {
        let card_id = id_i64("card_id", log.card_id.value())?;
        let deck_id = id_i64("deck_id", log.deck_id.value())?;

        let res = sqlx::query(
            r"
                INSERT INTO review_logs (
                    deck_id, card_id, grade, reviewed_at,
                    elapsed_days, scheduled_days, stability, difficulty, next_review_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ",
        )
        .bind(deck_id)
        .bind(card_id)
        .bind(grade_to_i64(log.grade))
        .bind(log.reviewed_at)
        .bind(log.elapsed_days)
        .bind(log.scheduled_days)
        .bind(log.stability)
        .bind(log.difficulty)
        .bind(log.next_review_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(res.last_insert_rowid())
    }

    async fn logs_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
    ) -> Result<Vec<ReviewLogRecord>, StorageError> {
        let deck = id_i64("deck_id", deck_id.value())?;
        let card = id_i64("card_id", card_id.value())?;

        let rows = sqlx::query(
            r"
                SELECT
                    id, deck_id, card_id, grade, reviewed_at,
                    elapsed_days, scheduled_days, stability, difficulty, next_review_at
                FROM review_logs
                WHERE deck_id = ?1 AND card_id = ?2
                ORDER BY reviewed_at ASC
            ",
        )
        .bind(deck)
        .bind(card)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(map_review_log_row(&row)?);
        }
        Ok(out)
    }
}

#[async_trait::async_trait]
impl ReviewPersistence for SqliteRepository {
    async fn apply_review(&self, card: &Card, log: ReviewLogRecord) -> Result<i64, StorageError> {
        if log.card_id != card.id() || log.deck_id != card.deck_id() {
            return Err(StorageError::Conflict);
        }

        let card_id = id_i64("card_id", card.id().value())?;
        let deck_id = id_i64("deck_id", card.deck_id().value())?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        sqlx::query(
            r"
            INSERT INTO cards (
                id, deck_id, prompt, prompt_media_id, answer, answer_media_id,
                phase, created_at, next_review_at, last_review_at, review_count,
                stability, difficulty
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id, deck_id) DO UPDATE SET
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
        .bind(card_id)
        .bind(deck_id)
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
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let res = sqlx::query(
            r"
                INSERT INTO review_logs (
                    deck_id, card_id, grade, reviewed_at,
                    elapsed_days, scheduled_days, stability, difficulty, next_review_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ",
        )
        .bind(deck_id)
        .bind(card_id)
        .bind(grade_to_i64(log.grade))
        .bind(log.reviewed_at)
        .bind(log.elapsed_days)
        .bind(log.scheduled_days)
        .bind(log.stability)
        .bind(log.difficulty)
        .bind(log.next_review_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(res.last_insert_rowid())
    }
}

use std::collections::HashMap;

use learn_core::model::{Card, CardId, DeckId, Tag, TagName};
use sqlx::Row;

use super::{
    SqliteRepository,
    mapping::{card_id_from_i64, map_card_row, map_tag_row, media_id_to_i64},
};
use crate::repository::{
    CardRepository, DeckPracticeCounts, NewCardRecord, StorageError, TagPracticeCounts,
};

fn u32_from_i64(field: &'static str, value: i64) -> Result<u32, StorageError> {
    u32::try_from(value)
        .map_err(|_| StorageError::Serialization(format!("invalid {field}: {value}")))
}

fn ser(error: &sqlx::Error) -> StorageError {
    StorageError::Serialization(error.to_string())
}

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

    async fn reset_deck_learning(
        &self,
        deck_id: DeckId,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;

        let result = sqlx::query(
            r"
            UPDATE cards
            SET
                phase = 'new',
                next_review_at = ?2,
                last_review_at = NULL,
                review_count = 0,
                stability = NULL,
                difficulty = NULL
            WHERE deck_id = ?1
            ",
        )
        .bind(deck)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(result.rows_affected())
    }

    async fn deck_practice_counts(
        &self,
        deck_id: DeckId,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<DeckPracticeCounts, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;

        let row = sqlx::query(
            r"
            SELECT
                COUNT(*) AS total,
                COALESCE(SUM(CASE WHEN review_count = 0 THEN 1 ELSE 0 END), 0) AS new_count,
                COALESCE(
                    SUM(CASE WHEN review_count > 0 AND next_review_at <= ?2 THEN 1 ELSE 0 END),
                    0
                ) AS due_count
            FROM cards
            WHERE deck_id = ?1
            ",
        )
        .bind(deck)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let total = u32_from_i64("total", row.try_get::<i64, _>("total").map_err(|e| ser(&e))?)?;
        let new = u32_from_i64(
            "new_count",
            row.try_get::<i64, _>("new_count").map_err(|e| ser(&e))?,
        )?;
        let due = u32_from_i64(
            "due_count",
            row.try_get::<i64, _>("due_count").map_err(|e| ser(&e))?,
        )?;

        Ok(DeckPracticeCounts { total, due, new })
    }

    async fn list_tag_practice_counts(
        &self,
        deck_id: DeckId,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<TagPracticeCounts>, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;

        let rows = sqlx::query(
            r"
            SELECT
                tags.name AS name,
                COALESCE(COUNT(cards.id), 0) AS total,
                COALESCE(SUM(CASE WHEN cards.review_count = 0 THEN 1 ELSE 0 END), 0) AS new_count,
                COALESCE(
                    SUM(
                        CASE
                            WHEN cards.review_count > 0 AND cards.next_review_at <= ?2 THEN 1
                            ELSE 0
                        END
                    ),
                    0
                ) AS due_count
            FROM tags
            LEFT JOIN card_tags ON card_tags.tag_id = tags.id
            LEFT JOIN cards ON cards.id = card_tags.card_id
            WHERE tags.deck_id = ?1
            GROUP BY tags.id
            ORDER BY tags.name ASC, tags.id ASC
            ",
        )
        .bind(deck)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let name_raw: String = row.try_get("name").map_err(|e| ser(&e))?;
            let name = TagName::new(name_raw)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            let total = u32_from_i64(
                "total",
                row.try_get::<i64, _>("total").map_err(|e| ser(&e))?,
            )?;
            let new = u32_from_i64(
                "new_count",
                row.try_get::<i64, _>("new_count").map_err(|e| ser(&e))?,
            )?;
            let due = u32_from_i64(
                "due_count",
                row.try_get::<i64, _>("due_count").map_err(|e| ser(&e))?,
            )?;

            out.push(TagPracticeCounts { name, total, due, new });
        }

        Ok(out)
    }

    async fn list_cards_by_tags(
        &self,
        deck_id: DeckId,
        tag_names: &[TagName],
    ) -> Result<Vec<Card>, StorageError> {
        if tag_names.is_empty() {
            return Ok(Vec::new());
        }

        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;

        let mut sql = String::from(
            r"
            SELECT DISTINCT
                cards.id, cards.deck_id, cards.prompt, cards.prompt_media_id, cards.answer,
                cards.answer_media_id, cards.phase, cards.created_at, cards.next_review_at,
                cards.last_review_at, cards.review_count, cards.stability, cards.difficulty
            FROM cards
            JOIN card_tags ON card_tags.card_id = cards.id
            JOIN tags ON tags.id = card_tags.tag_id
            WHERE cards.deck_id = ?1
              AND tags.name IN (
            ",
        );

        for i in 0..tag_names.len() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push('?');
            sql.push_str(&(i + 2).to_string());
        }
        sql.push_str(")\n");

        let mut q = sqlx::query(&sql).bind(deck);
        for name in tag_names {
            q = q.bind(name.as_str());
        }

        let rows = q
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

    async fn list_tags_for_deck(&self, deck_id: DeckId) -> Result<Vec<Tag>, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;

        let rows = sqlx::query(
            r"
            SELECT id, deck_id, name
            FROM tags
            WHERE deck_id = ?1
            ORDER BY name ASC, id ASC
            ",
        )
        .bind(deck)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut tags = Vec::with_capacity(rows.len());
        for row in rows {
            tags.push(map_tag_row(&row)?);
        }
        Ok(tags)
    }

    async fn list_tags_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
    ) -> Result<Vec<Tag>, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;
        let card = i64::try_from(card_id.value())
            .map_err(|_| StorageError::Serialization("card_id overflow".into()))?;

        let rows = sqlx::query(
            r"
            SELECT tags.id, tags.deck_id, tags.name
            FROM tags
            JOIN card_tags ON card_tags.tag_id = tags.id
            JOIN cards ON cards.id = card_tags.card_id
            WHERE cards.deck_id = ?1
              AND cards.id = ?2
            ORDER BY tags.name ASC, tags.id ASC
            ",
        )
        .bind(deck)
        .bind(card)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut tags = Vec::with_capacity(rows.len());
        for row in rows {
            tags.push(map_tag_row(&row)?);
        }
        Ok(tags)
    }

    async fn set_tags_for_card(
        &self,
        deck_id: DeckId,
        card_id: CardId,
        tag_names: &[TagName],
    ) -> Result<Vec<Tag>, StorageError> {
        let deck = i64::try_from(deck_id.value())
            .map_err(|_| StorageError::Serialization("deck_id overflow".into()))?;
        let card = i64::try_from(card_id.value())
            .map_err(|_| StorageError::Serialization("card_id overflow".into()))?;

        let mut tx = self.pool.begin().await.map_err(|e| StorageError::Connection(e.to_string()))?;

        sqlx::query(
            r"
            DELETE FROM card_tags
            WHERE card_id = ?1
            ",
        )
        .bind(card)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        for name in tag_names {
            sqlx::query(
                r"
                INSERT INTO tags (deck_id, name)
                VALUES (?1, ?2)
                ON CONFLICT(deck_id, name) DO NOTHING
                ",
            )
            .bind(deck)
            .bind(name.as_str())
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

            let tag_id: i64 = sqlx::query_scalar(
                r"
                SELECT id
                FROM tags
                WHERE deck_id = ?1 AND name = ?2
                ",
            )
            .bind(deck)
            .bind(name.as_str())
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

            sqlx::query(
                r"
                INSERT INTO card_tags (card_id, tag_id)
                VALUES (?1, ?2)
                ON CONFLICT(card_id, tag_id) DO NOTHING
                ",
            )
            .bind(card)
            .bind(tag_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;
        }

        let rows = sqlx::query(
            r"
            SELECT tags.id, tags.deck_id, tags.name
            FROM tags
            JOIN card_tags ON card_tags.tag_id = tags.id
            WHERE card_tags.card_id = ?1
            ORDER BY tags.name ASC, tags.id ASC
            ",
        )
        .bind(card)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut tags = Vec::with_capacity(rows.len());
        for row in rows {
            tags.push(map_tag_row(&row)?);
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(tags)
    }
}

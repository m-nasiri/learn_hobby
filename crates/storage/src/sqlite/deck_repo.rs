use learn_core::model::{Deck, DeckId};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use super::mapping::deck_id_from_i64;
use super::SqliteRepository;
use crate::repository::{DeckRepository, NewDeckRecord, StorageError};

fn ser<E: core::fmt::Display>(e: E) -> StorageError {
    StorageError::Serialization(e.to_string())
}

#[async_trait::async_trait]
impl DeckRepository for SqliteRepository {
    async fn insert_new_deck(&self, deck: NewDeckRecord) -> Result<DeckId, StorageError> {
        let description = deck.description;
        let created_at = deck.created_at;
        let new_cards = i64::from(deck.new_cards_per_day);
        let review_limit = i64::from(deck.review_limit_per_day);
        let micro = i64::from(deck.micro_session_size);
        let protect_overload = if deck.protect_overload { 1 } else { 0 };

        let res = sqlx::query(
            r"
            INSERT INTO decks (name, description, created_at, new_cards_per_day, review_limit_per_day, micro_session_size, protect_overload)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
        )
        .bind(deck.name)
        .bind(description)
        .bind(created_at)
        .bind(new_cards)
        .bind(review_limit)
        .bind(micro)
        .bind(protect_overload)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        deck_id_from_i64(res.last_insert_rowid())
    }

    async fn upsert_deck(&self, deck: &Deck) -> Result<(), StorageError> {
        let id = deck.id().value();
        let name = deck.name().to_string();
        let description = deck.description().map(ToString::to_string);
        let created_at = deck.created_at();
        let new_cards = i64::from(deck.settings().new_cards_per_day());
        let review_limit = i64::from(deck.settings().review_limit_per_day());
        let micro = i64::from(deck.settings().micro_session_size());
        let protect_overload = if deck.settings().protect_overload() { 1 } else { 0 };

        sqlx::query(
            r"
            INSERT INTO decks (id, name, description, created_at, new_cards_per_day, review_limit_per_day, micro_session_size, protect_overload)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                new_cards_per_day = excluded.new_cards_per_day,
                review_limit_per_day = excluded.review_limit_per_day,
                micro_session_size = excluded.micro_session_size,
                protect_overload = excluded.protect_overload
            ",
        )
        .bind(i64::try_from(id).map_err(|_| StorageError::Serialization("id overflow".into()))?)
        .bind(name)
        .bind(description)
        .bind(created_at)
        .bind(new_cards)
        .bind(review_limit)
        .bind(micro)
        .bind(protect_overload)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(())
    }

    async fn get_deck(&self, id: learn_core::model::DeckId) -> Result<Option<Deck>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT id, name, description, created_at, new_cards_per_day, review_limit_per_day, micro_session_size, protect_overload
            FROM decks WHERE id = ?1
            ",
        )
        .bind(
            i64::try_from(id.value()).map_err(|_| StorageError::Serialization("id overflow".into()))?,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        match row {
            Some(row) => deck_from_row(&row).map(Some),
            None => Ok(None),
        }
    }

    async fn list_decks(&self, limit: u32) -> Result<Vec<Deck>, StorageError> {
        let rows = sqlx::query(
            r"
            SELECT id, name, description, created_at, new_cards_per_day, review_limit_per_day, micro_session_size, protect_overload
            FROM decks
            ORDER BY id ASC
            LIMIT ?1
            ",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut decks = Vec::with_capacity(rows.len());
        for row in rows {
            decks.push(deck_from_row(&row)?);
        }
        Ok(decks)
    }
}

fn deck_from_row(row: &SqliteRow) -> Result<Deck, StorageError> {
    let settings = learn_core::model::DeckSettings::new(
        u32::try_from(row.try_get::<i64, _>("new_cards_per_day").map_err(ser)?)
            .map_err(|_| StorageError::Serialization("new_cards_per_day overflow".into()))?,
        u32::try_from(row.try_get::<i64, _>("review_limit_per_day").map_err(ser)?)
            .map_err(|_| StorageError::Serialization("review_limit_per_day overflow".into()))?,
        u32::try_from(row.try_get::<i64, _>("micro_session_size").map_err(ser)?)
            .map_err(|_| StorageError::Serialization("micro_session_size overflow".into()))?,
        row.try_get::<i64, _>("protect_overload").map_err(ser)? != 0,
    )
    .map_err(|e| StorageError::Serialization(e.to_string()))?;

    Deck::new(
        learn_core::model::DeckId::new(
            u64::try_from(row.try_get::<i64, _>("id").map_err(ser)?)
                .map_err(|_| StorageError::Serialization("id sign overflow".into()))?,
        ),
        row.try_get::<String, _>("name").map_err(ser)?,
        row.try_get::<Option<String>, _>("description").map_err(ser)?,
        settings,
        row.try_get("created_at").map_err(ser)?,
    )
    .map_err(|e| StorageError::Serialization(e.to_string()))
}

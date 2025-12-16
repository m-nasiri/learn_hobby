use learn_core::model::Deck;
use sqlx::Row;

use super::SqliteRepository;
use crate::repository::{DeckRepository, StorageError};

fn ser<E: core::fmt::Display>(e: E) -> StorageError {
    StorageError::Serialization(e.to_string())
}

#[async_trait::async_trait]
impl DeckRepository for SqliteRepository {
    async fn upsert_deck(&self, deck: &Deck) -> Result<(), StorageError> {
        let id = deck.id().value();
        let name = deck.name().to_string();
        let description = deck.description().map(ToString::to_string);
        let created_at = deck.created_at();
        let new_cards = i64::from(deck.settings().new_cards_per_day());
        let review_limit = i64::from(deck.settings().review_limit_per_day());
        let micro = i64::from(deck.settings().micro_session_size());

        sqlx::query(
            r"
            INSERT INTO decks (id, name, description, created_at, new_cards_per_day, review_limit_per_day, micro_session_size)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                new_cards_per_day = excluded.new_cards_per_day,
                review_limit_per_day = excluded.review_limit_per_day,
                micro_session_size = excluded.micro_session_size
            ",
        )
        .bind(i64::try_from(id).map_err(|_| StorageError::Serialization("id overflow".into()))?)
        .bind(name)
        .bind(description)
        .bind(created_at)
        .bind(new_cards)
        .bind(review_limit)
        .bind(micro)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(())
    }

    async fn get_deck(&self, id: learn_core::model::DeckId) -> Result<Deck, StorageError> {
        let row = sqlx::query(
            r"
            SELECT id, name, description, created_at, new_cards_per_day, review_limit_per_day, micro_session_size
            FROM decks WHERE id = ?1
            ",
        )
        .bind(
            i64::try_from(id.value()).map_err(|_| StorageError::Serialization("id overflow".into()))?,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?
        .ok_or(StorageError::NotFound)?;

        let settings = learn_core::model::DeckSettings::new(
            u32::try_from(row.try_get::<i64, _>("new_cards_per_day").map_err(ser)?)
                .map_err(|_| StorageError::Serialization("new_cards_per_day overflow".into()))?,
            u32::try_from(row.try_get::<i64, _>("review_limit_per_day").map_err(ser)?)
                .map_err(|_| StorageError::Serialization("review_limit_per_day overflow".into()))?,
            u32::try_from(row.try_get::<i64, _>("micro_session_size").map_err(ser)?)
                .map_err(|_| StorageError::Serialization("micro_session_size overflow".into()))?,
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
}

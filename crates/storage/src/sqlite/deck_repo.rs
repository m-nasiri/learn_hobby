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
        let protect_overload = i64::from(i32::from(deck.protect_overload));
        let preserve_stability_on_lapse = i64::from(i32::from(deck.preserve_stability_on_lapse));
        let lapse_min_interval_secs = i64::from(deck.lapse_min_interval_secs);
        let show_timer = i64::from(i32::from(deck.show_timer));
        let soft_time_reminder = i64::from(i32::from(deck.soft_time_reminder));
        let auto_advance_cards = i64::from(i32::from(deck.auto_advance_cards));
        let soft_time_reminder_secs = i64::from(deck.soft_time_reminder_secs);
        let auto_reveal_secs = i64::from(deck.auto_reveal_secs);
        let fsrs_target_retention = f64::from(deck.fsrs_target_retention);
        let fsrs_optimize_enabled = i64::from(i32::from(deck.fsrs_optimize_enabled));
        let fsrs_optimize_after = i64::from(deck.fsrs_optimize_after);

        let res = sqlx::query(
            r"
            INSERT INTO decks (
                name, description, created_at, new_cards_per_day, review_limit_per_day,
                micro_session_size, protect_overload, preserve_stability_on_lapse,
                lapse_min_interval_secs, show_timer, soft_time_reminder, auto_advance_cards,
                soft_time_reminder_secs, auto_reveal_secs, fsrs_target_retention,
                fsrs_optimize_enabled, fsrs_optimize_after
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ",
        )
        .bind(deck.name)
        .bind(description)
        .bind(created_at)
        .bind(new_cards)
        .bind(review_limit)
        .bind(micro)
        .bind(protect_overload)
        .bind(preserve_stability_on_lapse)
        .bind(lapse_min_interval_secs)
        .bind(show_timer)
        .bind(soft_time_reminder)
        .bind(auto_advance_cards)
        .bind(soft_time_reminder_secs)
        .bind(auto_reveal_secs)
        .bind(fsrs_target_retention)
        .bind(fsrs_optimize_enabled)
        .bind(fsrs_optimize_after)
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
        let protect_overload = i64::from(i32::from(deck.settings().protect_overload()));
        let preserve_stability_on_lapse =
            i64::from(i32::from(deck.settings().preserve_stability_on_lapse()));
        let lapse_min_interval_secs = i64::from(deck.settings().lapse_min_interval_secs());
        let show_timer = i64::from(i32::from(deck.settings().show_timer()));
        let soft_time_reminder = i64::from(i32::from(deck.settings().soft_time_reminder()));
        let auto_advance_cards = i64::from(i32::from(deck.settings().auto_advance_cards()));
        let soft_time_reminder_secs = i64::from(deck.settings().soft_time_reminder_secs());
        let auto_reveal_secs = i64::from(deck.settings().auto_reveal_secs());
        let fsrs_target_retention = f64::from(deck.settings().fsrs_target_retention());
        let fsrs_optimize_enabled = i64::from(i32::from(deck.settings().fsrs_optimize_enabled()));
        let fsrs_optimize_after = i64::from(deck.settings().fsrs_optimize_after());

        sqlx::query(
            r"
            INSERT INTO decks (
                id, name, description, created_at, new_cards_per_day, review_limit_per_day,
                micro_session_size, protect_overload, preserve_stability_on_lapse,
                lapse_min_interval_secs, show_timer, soft_time_reminder, auto_advance_cards,
                soft_time_reminder_secs, auto_reveal_secs, fsrs_target_retention,
                fsrs_optimize_enabled, fsrs_optimize_after
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                new_cards_per_day = excluded.new_cards_per_day,
                review_limit_per_day = excluded.review_limit_per_day,
                micro_session_size = excluded.micro_session_size,
                protect_overload = excluded.protect_overload,
                preserve_stability_on_lapse = excluded.preserve_stability_on_lapse,
                lapse_min_interval_secs = excluded.lapse_min_interval_secs,
                show_timer = excluded.show_timer,
                soft_time_reminder = excluded.soft_time_reminder,
                auto_advance_cards = excluded.auto_advance_cards,
                soft_time_reminder_secs = excluded.soft_time_reminder_secs,
                auto_reveal_secs = excluded.auto_reveal_secs,
                fsrs_target_retention = excluded.fsrs_target_retention,
                fsrs_optimize_enabled = excluded.fsrs_optimize_enabled,
                fsrs_optimize_after = excluded.fsrs_optimize_after
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
        .bind(preserve_stability_on_lapse)
        .bind(lapse_min_interval_secs)
        .bind(show_timer)
        .bind(soft_time_reminder)
        .bind(auto_advance_cards)
        .bind(soft_time_reminder_secs)
        .bind(auto_reveal_secs)
        .bind(fsrs_target_retention)
        .bind(fsrs_optimize_enabled)
        .bind(fsrs_optimize_after)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(())
    }

    async fn get_deck(&self, id: learn_core::model::DeckId) -> Result<Option<Deck>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT id, name, description, created_at, new_cards_per_day, review_limit_per_day,
                   micro_session_size, protect_overload, preserve_stability_on_lapse,
                   lapse_min_interval_secs, show_timer, soft_time_reminder, auto_advance_cards,
                   soft_time_reminder_secs, auto_reveal_secs, fsrs_target_retention,
                   fsrs_optimize_enabled, fsrs_optimize_after
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
            SELECT id, name, description, created_at, new_cards_per_day, review_limit_per_day,
                   micro_session_size, protect_overload, preserve_stability_on_lapse,
                   lapse_min_interval_secs, show_timer, soft_time_reminder, auto_advance_cards,
                   soft_time_reminder_secs, auto_reveal_secs, fsrs_target_retention,
                   fsrs_optimize_enabled, fsrs_optimize_after
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
        row.try_get::<i64, _>("preserve_stability_on_lapse")
            .map_err(ser)?
            != 0,
        u32::try_from(row.try_get::<i64, _>("lapse_min_interval_secs").map_err(ser)?)
            .map_err(|_| StorageError::Serialization("lapse_min_interval_secs overflow".into()))?,
        row.try_get::<i64, _>("show_timer").map_err(ser)? != 0,
        row.try_get::<i64, _>("soft_time_reminder").map_err(ser)? != 0,
        row.try_get::<i64, _>("auto_advance_cards").map_err(ser)? != 0,
        u32::try_from(row.try_get::<i64, _>("soft_time_reminder_secs").map_err(ser)?)
            .map_err(|_| StorageError::Serialization("soft_time_reminder_secs overflow".into()))?,
        u32::try_from(row.try_get::<i64, _>("auto_reveal_secs").map_err(ser)?)
            .map_err(|_| StorageError::Serialization("auto_reveal_secs overflow".into()))?,
        {
            let retention = row.try_get::<f32, _>("fsrs_target_retention").map_err(ser)?;
            if !retention.is_finite() || retention <= 0.0 || retention > 1.0 {
                return Err(StorageError::Serialization(
                    "fsrs_target_retention invalid".into(),
                ));
            }
            retention
        },
        row.try_get::<i64, _>("fsrs_optimize_enabled").map_err(ser)? != 0,
        u32::try_from(row.try_get::<i64, _>("fsrs_optimize_after").map_err(ser)?)
            .map_err(|_| StorageError::Serialization("fsrs_optimize_after overflow".into()))?,
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

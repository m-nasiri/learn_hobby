use learn_core::model::SessionSummary;
use sqlx::Row;

use super::{SqliteRepository, mapping::deck_id_from_i64};
use crate::repository::{SessionSummaryRepository, StorageError};

fn id_i64(field: &'static str, v: u64) -> Result<i64, StorageError> {
    i64::try_from(v).map_err(|_| StorageError::Serialization(format!("{field} overflow")))
}

fn ser<E: core::fmt::Display>(e: E) -> StorageError {
    StorageError::Serialization(e.to_string())
}

fn u32_from_i64(field: &'static str, v: i64) -> Result<u32, StorageError> {
    u32::try_from(v).map_err(|_| StorageError::Serialization(format!("invalid {field}: {v}")))
}

#[async_trait::async_trait]
impl SessionSummaryRepository for SqliteRepository {
    async fn append_summary(&self, summary: &SessionSummary) -> Result<i64, StorageError> {
        let deck_id = id_i64("deck_id", summary.deck_id().value())?;

        let res = sqlx::query(
            r"
                INSERT INTO session_summaries (
                    deck_id, started_at, completed_at, total_reviews,
                    again, hard, good, easy
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
        )
        .bind(deck_id)
        .bind(summary.started_at())
        .bind(summary.completed_at())
        .bind(i64::from(summary.total_reviews()))
        .bind(i64::from(summary.again()))
        .bind(i64::from(summary.hard()))
        .bind(i64::from(summary.good()))
        .bind(i64::from(summary.easy()))
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;

        Ok(res.last_insert_rowid())
    }

    async fn get_summary(&self, id: i64) -> Result<SessionSummary, StorageError> {
        let row = sqlx::query(
            r"
                SELECT
                    deck_id, started_at, completed_at, total_reviews,
                    again, hard, good, easy
                FROM session_summaries
                WHERE id = ?1
            ",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?
        .ok_or(StorageError::NotFound)?;

        let deck_id = deck_id_from_i64(row.try_get::<i64, _>("deck_id").map_err(ser)?)?;
        let started_at = row.try_get("started_at").map_err(ser)?;
        let completed_at = row.try_get("completed_at").map_err(ser)?;
        let total_reviews = u32_from_i64(
            "total_reviews",
            row.try_get::<i64, _>("total_reviews").map_err(ser)?,
        )?;
        let again = u32_from_i64("again", row.try_get::<i64, _>("again").map_err(ser)?)?;
        let hard = u32_from_i64("hard", row.try_get::<i64, _>("hard").map_err(ser)?)?;
        let good = u32_from_i64("good", row.try_get::<i64, _>("good").map_err(ser)?)?;
        let easy = u32_from_i64("easy", row.try_get::<i64, _>("easy").map_err(ser)?)?;

        SessionSummary::from_persisted(
            deck_id,
            started_at,
            completed_at,
            total_reviews,
            again,
            hard,
            good,
            easy,
        )
        .map_err(ser)
    }
}

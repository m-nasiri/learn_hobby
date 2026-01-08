use learn_core::model::{DeckId, SessionSummary};
use sqlx::Row;
use std::collections::HashSet;

use super::{SqliteRepository, mapping::deck_id_from_i64};
use crate::repository::{SessionSummaryRepository, SessionSummaryRow, StorageError};

fn id_i64(field: &'static str, v: u64) -> Result<i64, StorageError> {
    i64::try_from(v).map_err(|_| StorageError::Serialization(format!("{field} overflow")))
}

fn ser<E: core::fmt::Display>(e: E) -> StorageError {
    StorageError::Serialization(e.to_string())
}

fn u32_from_i64(field: &'static str, v: i64) -> Result<u32, StorageError> {
    u32::try_from(v).map_err(|_| StorageError::Serialization(format!("invalid {field}: {v}")))
}

fn map_summary_row(row: &sqlx::sqlite::SqliteRow) -> Result<SessionSummary, StorageError> {
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

fn map_summary_row_with_id(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<SessionSummaryRow, StorageError> {
    let id: i64 = row.try_get("id").map_err(ser)?;
    let summary = map_summary_row(row)?;
    Ok(SessionSummaryRow::new(id, summary))
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

        map_summary_row(&row)
    }

    async fn list_summaries(
        &self,
        deck_id: DeckId,
        completed_from: Option<chrono::DateTime<chrono::Utc>>,
        completed_until: Option<chrono::DateTime<chrono::Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummary>, StorageError> {
        let rows = self
            .list_summary_rows(deck_id, completed_from, completed_until, limit)
            .await?;
        Ok(rows.into_iter().map(|r| r.summary).collect())
    }

    async fn list_summary_rows(
        &self,
        deck_id: DeckId,
        completed_from: Option<chrono::DateTime<chrono::Utc>>,
        completed_until: Option<chrono::DateTime<chrono::Utc>>,
        limit: u32,
    ) -> Result<Vec<SessionSummaryRow>, StorageError> {
        let mut sql = String::from(
            r"
                SELECT
                    id, deck_id, started_at, completed_at, total_reviews,
                    again, hard, good, easy
                FROM session_summaries
                WHERE deck_id = ?1
            ",
        );

        let mut bind_index = 2;
        if completed_from.is_some() {
            sql.push_str(" AND completed_at >= ?");
            sql.push_str(&bind_index.to_string());
            bind_index += 1;
        }
        if completed_until.is_some() {
            sql.push_str(" AND completed_at <= ?");
            sql.push_str(&bind_index.to_string());
            bind_index += 1;
        }
        sql.push_str(" ORDER BY completed_at DESC, id DESC");
        sql.push_str(" LIMIT ?");
        sql.push_str(&bind_index.to_string());

        let mut query = sqlx::query(&sql).bind(id_i64("deck_id", deck_id.value())?);
        if let Some(from) = completed_from {
            query = query.bind(from);
        }
        if let Some(until) = completed_until {
            query = query.bind(until);
        }
        query = query.bind(i64::from(limit));

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(map_summary_row_with_id(&row)?);
        }

        Ok(out)
    }

    async fn list_latest_summary_rows(
        &self,
        deck_ids: &[DeckId],
    ) -> Result<Vec<SessionSummaryRow>, StorageError> {
        if deck_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut sql = String::from(
            r"
                SELECT
                    id, deck_id, started_at, completed_at, total_reviews,
                    again, hard, good, easy
                FROM session_summaries
                WHERE deck_id IN (
            ",
        );

        for i in 0..deck_ids.len() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push('?');
            sql.push_str(&(i + 1).to_string());
        }
        sql.push_str(")\n ORDER BY deck_id ASC, completed_at DESC, id DESC");

        let mut query = sqlx::query(&sql);
        for deck_id in deck_ids {
            let deck = id_i64("deck_id", deck_id.value())?;
            query = query.bind(deck);
        }

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for row in rows {
            let deck_id = deck_id_from_i64(row.try_get::<i64, _>("deck_id").map_err(ser)?)?;
            if !seen.insert(deck_id) {
                continue;
            }
            out.push(map_summary_row_with_id(&row)?);
        }

        Ok(out)
    }
}

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::repository::{
    AiUsageCompletion, AiUsageRepository, AiUsageStatus, NewAiUsageRecord, StorageError,
};

use super::SqliteRepository;

#[async_trait]
impl AiUsageRepository for SqliteRepository {
    async fn insert_started(&self, record: NewAiUsageRecord) -> Result<i64, StorageError> {
        let result = sqlx::query(
            r"
            INSERT INTO ai_usage (provider, model, created_at, status)
            VALUES (?1, ?2, ?3, ?4)
            ",
        )
        .bind(record.provider)
        .bind(record.model)
        .bind(record.created_at)
        .bind(status_to_str(AiUsageStatus::Started))
        .execute(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        Ok(result.last_insert_rowid())
    }

    async fn update_completion(
        &self,
        id: i64,
        completion: AiUsageCompletion,
    ) -> Result<(), StorageError> {
        let result = sqlx::query(
            r"
            UPDATE ai_usage
            SET status = ?1,
                prompt_tokens = ?2,
                completion_tokens = ?3,
                total_tokens = ?4,
                cost_micro_usd = ?5
            WHERE id = ?6
            ",
        )
        .bind(status_to_str(completion.status))
        .bind(completion.prompt_tokens.map(i64::from))
        .bind(completion.completion_tokens.map(i64::from))
        .bind(completion.total_tokens.map(i64::from))
        .bind(completion.cost_micro_usd.map(|val| {
            i64::try_from(val).unwrap_or(i64::MAX)
        }))
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }

        Ok(())
    }

    async fn count_since(&self, since: DateTime<Utc>) -> Result<u32, StorageError> {
        let row = sqlx::query(
            r"
            SELECT COUNT(*) as count
            FROM ai_usage
            WHERE created_at >= ?1
            ",
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        let count: i64 = row
            .try_get("count")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
    }

    async fn last_request_at(&self) -> Result<Option<DateTime<Utc>>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT MAX(created_at) as last_request_at
            FROM ai_usage
            ",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        let last: Option<DateTime<Utc>> = row
            .try_get("last_request_at")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        Ok(last)
    }

    async fn sum_cost_since(&self, since: DateTime<Utc>) -> Result<u64, StorageError> {
        let row = sqlx::query(
            r"
            SELECT COALESCE(SUM(cost_micro_usd), 0) as total_cost
            FROM ai_usage
            WHERE created_at >= ?1
            ",
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        let total: i64 = row
            .try_get("total_cost")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        Ok(u64::try_from(total).unwrap_or(u64::MAX))
    }
}

fn status_to_str(status: AiUsageStatus) -> &'static str {
    match status {
        AiUsageStatus::Started => "started",
        AiUsageStatus::Succeeded => "succeeded",
        AiUsageStatus::Failed => "failed",
    }
}

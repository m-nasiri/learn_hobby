use async_trait::async_trait;
use sqlx::Row;

use crate::repository::{AiPriceBookEntry, AiPriceBookRepository, StorageError};

use super::SqliteRepository;

#[async_trait]
impl AiPriceBookRepository for SqliteRepository {
    async fn get_entry(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<Option<AiPriceBookEntry>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT provider, model, input_micro_usd_per_million, output_micro_usd_per_million, deprecated
            FROM ai_price_book
            WHERE provider = ?1 AND model = ?2
            ",
        )
        .bind(provider)
        .bind(model)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let input_micro_usd_per_million: i64 = row
            .try_get("input_micro_usd_per_million")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let output_micro_usd_per_million: i64 = row
            .try_get("output_micro_usd_per_million")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let deprecated: i64 = row
            .try_get("deprecated")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;

        Ok(Some(AiPriceBookEntry {
            provider: row
                .try_get("provider")
                .map_err(|err| StorageError::Serialization(err.to_string()))?,
            model: row
                .try_get("model")
                .map_err(|err| StorageError::Serialization(err.to_string()))?,
            input_micro_usd_per_million: input_micro_usd_per_million
                .try_into()
                .map_err(|_| StorageError::Serialization("invalid input price".into()))?,
            output_micro_usd_per_million: output_micro_usd_per_million
                .try_into()
                .map_err(|_| StorageError::Serialization("invalid output price".into()))?,
            deprecated: deprecated != 0,
        }))
    }

    async fn list_entries(&self) -> Result<Vec<AiPriceBookEntry>, StorageError> {
        let rows = sqlx::query(
            r"
            SELECT provider, model, input_micro_usd_per_million, output_micro_usd_per_million, deprecated
            FROM ai_price_book
            ORDER BY provider, model
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let input_micro_usd_per_million: i64 = row
                .try_get("input_micro_usd_per_million")
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
            let output_micro_usd_per_million: i64 = row
                .try_get("output_micro_usd_per_million")
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
            let deprecated: i64 = row
                .try_get("deprecated")
                .map_err(|err| StorageError::Serialization(err.to_string()))?;

            entries.push(AiPriceBookEntry {
                provider: row
                    .try_get("provider")
                    .map_err(|err| StorageError::Serialization(err.to_string()))?,
                model: row
                    .try_get("model")
                    .map_err(|err| StorageError::Serialization(err.to_string()))?,
                input_micro_usd_per_million: input_micro_usd_per_million
                    .try_into()
                    .map_err(|_| StorageError::Serialization("invalid input price".into()))?,
                output_micro_usd_per_million: output_micro_usd_per_million
                    .try_into()
                    .map_err(|_| StorageError::Serialization("invalid output price".into()))?,
                deprecated: deprecated != 0,
            });
        }

        Ok(entries)
    }

    async fn upsert_entry(&self, entry: &AiPriceBookEntry) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO ai_price_book (
                provider, model, input_micro_usd_per_million, output_micro_usd_per_million, deprecated
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(provider, model) DO UPDATE SET
                input_micro_usd_per_million = excluded.input_micro_usd_per_million,
                output_micro_usd_per_million = excluded.output_micro_usd_per_million,
                deprecated = excluded.deprecated
            ",
        )
        .bind(&entry.provider)
        .bind(&entry.model)
        .bind(i64::try_from(entry.input_micro_usd_per_million).map_err(|_| {
            StorageError::Serialization("invalid input price".into())
        })?)
        .bind(i64::try_from(entry.output_micro_usd_per_million).map_err(|_| {
            StorageError::Serialization("invalid output price".into())
        })?)
        .bind(i64::from(entry.deprecated))
        .execute(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        Ok(())
    }
}

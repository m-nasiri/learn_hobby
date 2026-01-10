use async_trait::async_trait;
use sqlx::Row;

use crate::repository::{AppSettingsRepository, StorageError};
use learn_core::model::AppSettings;

use super::SqliteRepository;

#[async_trait]
impl AppSettingsRepository for SqliteRepository {
    async fn get_settings(&self) -> Result<Option<AppSettings>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT api_key, api_model, api_base_url, ai_system_prompt
            FROM app_settings
            WHERE id = 1
            ",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let api_key: Option<String> = row.try_get("api_key").map_err(|err| {
            StorageError::Serialization(err.to_string())
        })?;
        let api_model: Option<String> = row.try_get("api_model").map_err(|err| {
            StorageError::Serialization(err.to_string())
        })?;
        let api_base_url: Option<String> = row.try_get("api_base_url").map_err(|err| {
            StorageError::Serialization(err.to_string())
        })?;
        let ai_system_prompt: Option<String> = row
            .try_get("ai_system_prompt")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;

        AppSettings::from_persisted(api_key, api_model, api_base_url, ai_system_prompt)
            .map(Some)
            .map_err(|err| StorageError::Serialization(err.to_string()))
    }

    async fn save_settings(&self, settings: &AppSettings) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO app_settings (
                id, api_key, api_model, api_base_url, ai_system_prompt
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                api_key = excluded.api_key,
                api_model = excluded.api_model,
                api_base_url = excluded.api_base_url,
                ai_system_prompt = excluded.ai_system_prompt
            ",
        )
        .bind(1_i64)
        .bind(settings.api_key())
        .bind(settings.api_model())
        .bind(settings.api_base_url())
        .bind(settings.ai_system_prompt())
        .execute(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        Ok(())
    }
}

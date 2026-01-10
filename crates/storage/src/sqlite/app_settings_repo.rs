use async_trait::async_trait;
use sqlx::Row;

use crate::repository::{AppSettingsRepository, StorageError};
use learn_core::model::{AppSettings, AppSettingsDraft};

use super::SqliteRepository;

#[async_trait]
impl AppSettingsRepository for SqliteRepository {
    async fn get_settings(&self) -> Result<Option<AppSettings>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT
                api_key,
                api_model,
                api_fallback_model,
                api_base_url,
                ai_system_prompt,
                ai_daily_request_cap,
                ai_cooldown_secs,
                ai_monthly_budget_cents,
                ai_warn_50_pct,
                ai_warn_80_pct,
                ai_warn_100_pct
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
        let api_fallback_model: Option<String> =
            row.try_get("api_fallback_model")
                .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let api_base_url: Option<String> = row.try_get("api_base_url").map_err(|err| {
            StorageError::Serialization(err.to_string())
        })?;
        let ai_system_prompt: Option<String> = row
            .try_get("ai_system_prompt")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let ai_daily_request_cap: Option<i64> = row
            .try_get("ai_daily_request_cap")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let ai_cooldown_secs: Option<i64> = row
            .try_get("ai_cooldown_secs")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let ai_monthly_budget_cents: Option<i64> = row
            .try_get("ai_monthly_budget_cents")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let ai_warn_50_pct: Option<i64> = row
            .try_get("ai_warn_50_pct")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let ai_warn_80_pct: Option<i64> = row
            .try_get("ai_warn_80_pct")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;
        let ai_warn_100_pct: Option<i64> = row
            .try_get("ai_warn_100_pct")
            .map_err(|err| StorageError::Serialization(err.to_string()))?;

        AppSettings::from_persisted(AppSettingsDraft {
            api_key,
            api_model,
            api_fallback_model,
            api_base_url,
            ai_system_prompt,
            ai_daily_request_cap: ai_daily_request_cap.and_then(|val| u32::try_from(val).ok()),
            ai_cooldown_secs: ai_cooldown_secs.and_then(|val| u32::try_from(val).ok()),
            ai_monthly_budget_cents: ai_monthly_budget_cents
                .and_then(|val| u32::try_from(val).ok()),
            ai_warn_50_pct: ai_warn_50_pct.and_then(|val| u8::try_from(val).ok()),
            ai_warn_80_pct: ai_warn_80_pct.and_then(|val| u8::try_from(val).ok()),
            ai_warn_100_pct: ai_warn_100_pct.and_then(|val| u8::try_from(val).ok()),
        })
        .map(Some)
        .map_err(|err| StorageError::Serialization(err.to_string()))
    }

    async fn save_settings(&self, settings: &AppSettings) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO app_settings (
                id,
                api_key,
                api_model,
                api_fallback_model,
                api_base_url,
                ai_system_prompt,
                ai_daily_request_cap,
                ai_cooldown_secs,
                ai_monthly_budget_cents,
                ai_warn_50_pct,
                ai_warn_80_pct,
                ai_warn_100_pct
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
                api_key = excluded.api_key,
                api_model = excluded.api_model,
                api_fallback_model = excluded.api_fallback_model,
                api_base_url = excluded.api_base_url,
                ai_system_prompt = excluded.ai_system_prompt,
                ai_daily_request_cap = excluded.ai_daily_request_cap,
                ai_cooldown_secs = excluded.ai_cooldown_secs,
                ai_monthly_budget_cents = excluded.ai_monthly_budget_cents,
                ai_warn_50_pct = excluded.ai_warn_50_pct,
                ai_warn_80_pct = excluded.ai_warn_80_pct,
                ai_warn_100_pct = excluded.ai_warn_100_pct
            ",
        )
        .bind(1_i64)
        .bind(settings.api_key())
        .bind(settings.api_model())
        .bind(settings.api_fallback_model())
        .bind(settings.api_base_url())
        .bind(settings.ai_system_prompt())
        .bind(i64::from(settings.ai_daily_request_cap()))
        .bind(i64::from(settings.ai_cooldown_secs()))
        .bind(i64::from(settings.ai_monthly_budget_cents()))
        .bind(i64::from(settings.ai_warn_50_pct()))
        .bind(i64::from(settings.ai_warn_80_pct()))
        .bind(i64::from(settings.ai_warn_100_pct()))
        .execute(&self.pool)
        .await
        .map_err(|err| StorageError::Connection(err.to_string()))?;

        Ok(())
    }
}

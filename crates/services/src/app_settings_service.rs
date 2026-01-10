use std::sync::Arc;

use learn_core::model::{AppSettings, AppSettingsDraft};
use storage::repository::AppSettingsRepository;

use crate::error::AppSettingsServiceError;

#[derive(Clone)]
pub struct AppSettingsService {
    repo: Arc<dyn AppSettingsRepository>,
}

impl AppSettingsService {
    #[must_use]
    pub fn new(repo: Arc<dyn AppSettingsRepository>) -> Self {
        Self { repo }
    }

    /// Load persisted settings (or defaults if missing).
    ///
    /// # Errors
    ///
    /// Returns `AppSettingsServiceError` on storage failures.
    pub async fn load(&self) -> Result<AppSettings, AppSettingsServiceError> {
        let settings = self.repo.get_settings().await?;
        Ok(settings.unwrap_or_default())
    }

    /// Validate and persist new settings.
    ///
    /// # Errors
    ///
    /// Returns `AppSettingsServiceError` if validation fails or persistence fails.
    pub async fn save(
        &self,
        draft: AppSettingsDraft,
    ) -> Result<AppSettings, AppSettingsServiceError> {
        let settings = draft.validate()?;
        self.repo.save_settings(&settings).await?;
        Ok(settings)
    }
}

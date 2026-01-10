use thiserror::Error;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppSettings {
    api_key: Option<String>,
    api_model: Option<String>,
    api_base_url: Option<String>,
    ai_system_prompt: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AppSettingsDraft {
    pub api_key: Option<String>,
    pub api_model: Option<String>,
    pub api_base_url: Option<String>,
    pub ai_system_prompt: Option<String>,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppSettingsError {
    #[error("invalid base URL")]
    InvalidBaseUrl,
}

impl AppSettingsDraft {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate and normalize the draft into persisted settings.
    ///
    /// # Errors
    ///
    /// Returns `AppSettingsError` if the base URL is present but invalid.
    pub fn validate(self) -> Result<AppSettings, AppSettingsError> {
        let api_key = normalize_optional(self.api_key);
        let api_model = normalize_optional(self.api_model);
        let api_base_url = normalize_optional(self.api_base_url);
        let ai_system_prompt = normalize_optional(self.ai_system_prompt);

        if let Some(url) = api_base_url.as_ref() {
            if Url::parse(url).is_err() {
                return Err(AppSettingsError::InvalidBaseUrl);
            }
        }

        Ok(AppSettings {
            api_key,
            api_model,
            api_base_url,
            ai_system_prompt,
        })
    }
}

impl AppSettings {
    #[must_use]
    pub fn from_persisted(
        api_key: Option<String>,
        api_model: Option<String>,
        api_base_url: Option<String>,
        ai_system_prompt: Option<String>,
    ) -> Result<Self, AppSettingsError> {
        AppSettingsDraft {
            api_key,
            api_model,
            api_base_url,
            ai_system_prompt,
        }
        .validate()
    }

    #[must_use]
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    #[must_use]
    pub fn api_model(&self) -> Option<&str> {
        self.api_model.as_deref()
    }

    #[must_use]
    pub fn api_base_url(&self) -> Option<&str> {
        self.api_base_url.as_deref()
    }

    #[must_use]
    pub fn ai_system_prompt(&self) -> Option<&str> {
        self.ai_system_prompt.as_deref()
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            api_model: None,
            api_base_url: None,
            ai_system_prompt: None,
        }
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|val| val.trim().to_string())
        .filter(|val| !val.is_empty())
}

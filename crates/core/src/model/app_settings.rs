use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppSettings {
    api_key: Option<String>,
    api_model: Option<String>,
    api_fallback_model: Option<String>,
    ai_system_prompt: Option<String>,
    ai_daily_request_cap: u32,
    ai_cooldown_secs: u32,
}

#[derive(Clone, Debug, Default)]
pub struct AppSettingsDraft {
    pub api_key: Option<String>,
    pub api_model: Option<String>,
    pub api_fallback_model: Option<String>,
    pub ai_system_prompt: Option<String>,
    pub ai_daily_request_cap: Option<u32>,
    pub ai_cooldown_secs: Option<u32>,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppSettingsError {
    #[error("daily request cap must be greater than zero")]
    InvalidDailyRequestCap,
    #[error("cooldown seconds must be greater than zero")]
    InvalidCooldownSeconds,
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
        let api_fallback_model = normalize_optional(self.api_fallback_model);
        let ai_system_prompt = normalize_optional(self.ai_system_prompt);

        let ai_daily_request_cap =
            self.ai_daily_request_cap.unwrap_or(DEFAULT_AI_DAILY_REQUEST_CAP);
        if ai_daily_request_cap == 0 {
            return Err(AppSettingsError::InvalidDailyRequestCap);
        }

        let ai_cooldown_secs = self.ai_cooldown_secs.unwrap_or(DEFAULT_AI_COOLDOWN_SECS);
        if ai_cooldown_secs == 0 {
            return Err(AppSettingsError::InvalidCooldownSeconds);
        }

        Ok(AppSettings {
            api_key,
            api_model,
            api_fallback_model,
            ai_system_prompt,
            ai_daily_request_cap,
            ai_cooldown_secs,
        })
    }
}

impl AppSettings {
    /// Build settings from persisted draft values.
    ///
    /// # Errors
    ///
    /// Returns `AppSettingsError` if validation fails.
    pub fn from_persisted(draft: AppSettingsDraft) -> Result<Self, AppSettingsError> {
        draft.validate()
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
    pub fn ai_system_prompt(&self) -> Option<&str> {
        self.ai_system_prompt.as_deref()
    }

    #[must_use]
    pub fn api_fallback_model(&self) -> Option<&str> {
        self.api_fallback_model.as_deref()
    }

    #[must_use]
    pub fn ai_daily_request_cap(&self) -> u32 {
        self.ai_daily_request_cap
    }

    #[must_use]
    pub fn ai_cooldown_secs(&self) -> u32 {
        self.ai_cooldown_secs
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            api_model: Some(DEFAULT_AI_PREFERRED_MODEL.to_string()),
            api_fallback_model: Some(DEFAULT_AI_FALLBACK_MODEL.to_string()),
            ai_system_prompt: None,
            ai_daily_request_cap: DEFAULT_AI_DAILY_REQUEST_CAP,
            ai_cooldown_secs: DEFAULT_AI_COOLDOWN_SECS,
        }
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|val| val.trim().to_string())
        .filter(|val| !val.is_empty())
}

const DEFAULT_AI_DAILY_REQUEST_CAP: u32 = 100;
const DEFAULT_AI_COOLDOWN_SECS: u32 = 5;
const DEFAULT_AI_PREFERRED_MODEL: &str = "gpt-4.1-mini";
const DEFAULT_AI_FALLBACK_MODEL: &str = "gpt-4o-mini";

use thiserror::Error;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppSettings {
    api_key: Option<String>,
    api_model: Option<String>,
    api_fallback_model: Option<String>,
    api_base_url: Option<String>,
    ai_system_prompt: Option<String>,
    ai_daily_request_cap: u32,
    ai_cooldown_secs: u32,
    ai_monthly_budget_cents: u32,
    ai_warn_50_pct: u8,
    ai_warn_80_pct: u8,
    ai_warn_100_pct: u8,
}

#[derive(Clone, Debug, Default)]
pub struct AppSettingsDraft {
    pub api_key: Option<String>,
    pub api_model: Option<String>,
    pub api_fallback_model: Option<String>,
    pub api_base_url: Option<String>,
    pub ai_system_prompt: Option<String>,
    pub ai_daily_request_cap: Option<u32>,
    pub ai_cooldown_secs: Option<u32>,
    pub ai_monthly_budget_cents: Option<u32>,
    pub ai_warn_50_pct: Option<u8>,
    pub ai_warn_80_pct: Option<u8>,
    pub ai_warn_100_pct: Option<u8>,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppSettingsError {
    #[error("invalid base URL")]
    InvalidBaseUrl,
    #[error("daily request cap must be greater than zero")]
    InvalidDailyRequestCap,
    #[error("cooldown seconds must be greater than zero")]
    InvalidCooldownSeconds,
    #[error("monthly budget must be greater than zero")]
    InvalidMonthlyBudget,
    #[error("invalid budget warning thresholds")]
    InvalidBudgetThresholds,
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
        let api_base_url = normalize_optional(self.api_base_url);
        let ai_system_prompt = normalize_optional(self.ai_system_prompt);

        if let Some(url) = api_base_url.as_ref()
            && Url::parse(url).is_err()
        {
            return Err(AppSettingsError::InvalidBaseUrl);
        }

        let ai_daily_request_cap =
            self.ai_daily_request_cap.unwrap_or(DEFAULT_AI_DAILY_REQUEST_CAP);
        if ai_daily_request_cap == 0 {
            return Err(AppSettingsError::InvalidDailyRequestCap);
        }

        let ai_cooldown_secs = self.ai_cooldown_secs.unwrap_or(DEFAULT_AI_COOLDOWN_SECS);
        if ai_cooldown_secs == 0 {
            return Err(AppSettingsError::InvalidCooldownSeconds);
        }

        let ai_monthly_budget_cents =
            self.ai_monthly_budget_cents.unwrap_or(DEFAULT_AI_MONTHLY_BUDGET_CENTS);
        if ai_monthly_budget_cents == 0 {
            return Err(AppSettingsError::InvalidMonthlyBudget);
        }

        let ai_warn_50_pct = self.ai_warn_50_pct.unwrap_or(DEFAULT_AI_WARN_50_PCT);
        let ai_warn_80_pct = self.ai_warn_80_pct.unwrap_or(DEFAULT_AI_WARN_80_PCT);
        let ai_warn_100_pct = self.ai_warn_100_pct.unwrap_or(DEFAULT_AI_WARN_100_PCT);
        validate_thresholds(ai_warn_50_pct, ai_warn_80_pct, ai_warn_100_pct)?;

        Ok(AppSettings {
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
            ai_warn_100_pct,
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
    pub fn api_base_url(&self) -> Option<&str> {
        self.api_base_url.as_deref()
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

    #[must_use]
    pub fn ai_monthly_budget_cents(&self) -> u32 {
        self.ai_monthly_budget_cents
    }

    #[must_use]
    pub fn ai_warn_50_pct(&self) -> u8 {
        self.ai_warn_50_pct
    }

    #[must_use]
    pub fn ai_warn_80_pct(&self) -> u8 {
        self.ai_warn_80_pct
    }

    #[must_use]
    pub fn ai_warn_100_pct(&self) -> u8 {
        self.ai_warn_100_pct
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            api_model: Some(DEFAULT_AI_PREFERRED_MODEL.to_string()),
            api_fallback_model: Some(DEFAULT_AI_FALLBACK_MODEL.to_string()),
            api_base_url: None,
            ai_system_prompt: None,
            ai_daily_request_cap: DEFAULT_AI_DAILY_REQUEST_CAP,
            ai_cooldown_secs: DEFAULT_AI_COOLDOWN_SECS,
            ai_monthly_budget_cents: DEFAULT_AI_MONTHLY_BUDGET_CENTS,
            ai_warn_50_pct: DEFAULT_AI_WARN_50_PCT,
            ai_warn_80_pct: DEFAULT_AI_WARN_80_PCT,
            ai_warn_100_pct: DEFAULT_AI_WARN_100_PCT,
        }
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|val| val.trim().to_string())
        .filter(|val| !val.is_empty())
}

fn validate_thresholds(
    warn_50_pct: u8,
    warn_80_pct: u8,
    warn_100_pct: u8,
) -> Result<(), AppSettingsError> {
    if warn_50_pct == 0 || warn_80_pct == 0 || warn_100_pct == 0 {
        return Err(AppSettingsError::InvalidBudgetThresholds);
    }
    if warn_50_pct >= warn_80_pct || warn_80_pct >= warn_100_pct {
        return Err(AppSettingsError::InvalidBudgetThresholds);
    }
    if warn_100_pct > 100 {
        return Err(AppSettingsError::InvalidBudgetThresholds);
    }
    Ok(())
}

const DEFAULT_AI_DAILY_REQUEST_CAP: u32 = 100;
const DEFAULT_AI_COOLDOWN_SECS: u32 = 5;
const DEFAULT_AI_MONTHLY_BUDGET_CENTS: u32 = 500;
const DEFAULT_AI_WARN_50_PCT: u8 = 50;
const DEFAULT_AI_WARN_80_PCT: u8 = 80;
const DEFAULT_AI_WARN_100_PCT: u8 = 100;
const DEFAULT_AI_PREFERRED_MODEL: &str = "gpt-4.1-mini";
const DEFAULT_AI_FALLBACK_MODEL: &str = "gpt-4o-mini";

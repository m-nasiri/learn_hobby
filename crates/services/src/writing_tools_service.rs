use std::env;
use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::WritingToolsError;
use learn_core::model::AppSettings;
use storage::repository::AppSettingsRepository;

#[derive(Clone, Debug)]
pub struct WritingToolsConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub system_prompt: Option<String>,
}

impl WritingToolsConfig {
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let api_key = env::var("LEARN_AI_API_KEY")
            .or_else(|_| env::var("LEARN_API_KEY"))
            .ok()?;
        if api_key.trim().is_empty() {
            return None;
        }
        let base_url =
            env::var("LEARN_AI_BASE_URL").unwrap_or_else(|_| default_base_url().into());
        let model = env::var("LEARN_AI_MODEL").unwrap_or_else(|_| default_model().into());
        Some(Self {
            base_url,
            api_key,
            model,
            system_prompt: env::var("LEARN_AI_SYSTEM_PROMPT").ok(),
        })
    }

    #[must_use]
    pub fn from_settings(settings: &AppSettings, fallback: Option<&Self>) -> Option<Self> {
        let api_key = settings
            .api_key()
            .map(str::to_string)
            .or_else(|| fallback.map(|config| config.api_key.clone()))?;
        let base_url = settings
            .api_base_url()
            .map(str::to_string)
            .or_else(|| fallback.map(|config| config.base_url.clone()))
            .unwrap_or_else(|| default_base_url().into());
        let model = settings
            .api_model()
            .map(str::to_string)
            .or_else(|| fallback.map(|config| config.model.clone()))
            .unwrap_or_else(|| default_model().into());
        let system_prompt = settings
            .ai_system_prompt()
            .map(str::to_string)
            .or_else(|| fallback.and_then(|config| config.system_prompt.clone()));
        Some(Self {
            base_url,
            api_key,
            model,
            system_prompt,
        })
    }
}

#[derive(Clone)]
pub struct WritingToolsService {
    client: Client,
    settings_repo: Arc<dyn AppSettingsRepository>,
    env_config: Option<WritingToolsConfig>,
}

impl WritingToolsService {
    #[must_use]
    pub fn from_env(settings_repo: Arc<dyn AppSettingsRepository>) -> Self {
        Self::new(settings_repo, WritingToolsConfig::from_env())
    }

    #[must_use]
    pub fn new(
        settings_repo: Arc<dyn AppSettingsRepository>,
        env_config: Option<WritingToolsConfig>,
    ) -> Self {
        Self {
            client: Client::new(),
            settings_repo,
            env_config,
        }
    }

    /// Generate text from a prompt.
    ///
    /// # Errors
    ///
    /// Returns `WritingToolsError` when the service is disabled, the request fails,
    /// or the response is empty.
    pub async fn generate(&self, prompt: &str) -> Result<String, WritingToolsError> {
        let config = self.resolve_config().await?;

        let url = format!(
            "{}/chat/completions",
            config.base_url.trim_end_matches('/')
        );
        let mut messages = Vec::new();
        if let Some(system_prompt) = config.system_prompt.as_ref() {
            messages.push(ChatMessage {
                role: "system",
                content: system_prompt.clone(),
            });
        }
        messages.push(ChatMessage {
            role: "user",
            content: prompt.to_string(),
        });
        let payload = ChatRequest {
            model: config.model.clone(),
            messages,
            temperature: 0.2,
        };

        let response = self
            .client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(WritingToolsError::HttpStatus(response.status()));
        }

        let body: ChatResponse = response.json().await?;
        let content = body
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .ok_or(WritingToolsError::EmptyResponse)?;

        Ok(content.trim().to_string())
    }

    async fn resolve_config(&self) -> Result<WritingToolsConfig, WritingToolsError> {
        let settings = self.settings_repo.get_settings().await?;
        let settings = settings.unwrap_or_default();
        WritingToolsConfig::from_settings(&settings, self.env_config.as_ref())
            .ok_or(WritingToolsError::Disabled)
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

fn default_base_url() -> &'static str {
    "https://api.openai.com/v1"
}

fn default_model() -> &'static str {
    "gpt-4o-mini"
}

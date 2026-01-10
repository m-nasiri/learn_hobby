use std::env;
use std::sync::Arc;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::ai::AiUsageService;
use crate::error::WritingToolsError;
use learn_core::model::AppSettings;
use storage::repository::AppSettingsRepository;

#[derive(Clone, Debug)]
pub struct WritingToolsConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub preferred_model: String,
    pub fallback_model: Option<String>,
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
        let preferred_model =
            env::var("LEARN_AI_MODEL").unwrap_or_else(|_| default_model().into());
        let fallback_model = env::var("LEARN_AI_FALLBACK_MODEL")
            .ok()
            .filter(|model| !model.trim().is_empty())
            .or_else(|| Some(default_fallback_model().into()));
        Some(Self {
            provider: default_provider().to_string(),
            base_url,
            api_key,
            preferred_model,
            fallback_model,
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
        let preferred_model = settings
            .api_model()
            .map(str::to_string)
            .or_else(|| fallback.map(|config| config.preferred_model.clone()))
            .unwrap_or_else(|| default_model().into());
        let fallback_model = settings
            .api_fallback_model()
            .map(str::to_string)
            .or_else(|| fallback.and_then(|config| config.fallback_model.clone()))
            .or_else(|| Some(default_fallback_model().into()));
        let system_prompt = settings
            .ai_system_prompt()
            .map(str::to_string)
            .or_else(|| fallback.and_then(|config| config.system_prompt.clone()));
        Some(Self {
            provider: default_provider().to_string(),
            base_url,
            api_key,
            preferred_model,
            fallback_model,
            system_prompt,
        })
    }
}

#[derive(Clone)]
pub struct WritingToolsService {
    client: Client,
    settings_repo: Arc<dyn AppSettingsRepository>,
    env_config: Option<WritingToolsConfig>,
    usage: Arc<AiUsageService>,
}

impl WritingToolsService {
    #[must_use]
    pub fn from_env(
        settings_repo: Arc<dyn AppSettingsRepository>,
        usage: Arc<AiUsageService>,
    ) -> Self {
        Self::new(settings_repo, WritingToolsConfig::from_env(), usage)
    }

    #[must_use]
    pub fn new(
        settings_repo: Arc<dyn AppSettingsRepository>,
        env_config: Option<WritingToolsConfig>,
        usage: Arc<AiUsageService>,
    ) -> Self {
        Self {
            client: Client::new(),
            settings_repo,
            env_config,
            usage,
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
        let preferred = config.preferred_model.clone();
        let fallback = config.fallback_model.clone();

        match self.generate_with_model(prompt, &config, &preferred).await {
            Ok(result) => Ok(result),
            Err(WritingToolsError::HttpStatus(status))
                if status == StatusCode::TOO_MANY_REQUESTS =>
            {
                let Some(fallback_model) = fallback.filter(|model| model != &preferred) else {
                    return Err(WritingToolsError::HttpStatus(status));
                };
                self.generate_with_model(prompt, &config, &fallback_model)
                    .await
            }
            Err(err) => Err(err),
        }
    }

    async fn resolve_config(&self) -> Result<WritingToolsConfig, WritingToolsError> {
        let settings = self.settings_repo.get_settings().await?;
        let settings = settings.unwrap_or_default();
        WritingToolsConfig::from_settings(&settings, self.env_config.as_ref())
            .ok_or(WritingToolsError::Disabled)
    }

    async fn generate_with_model(
        &self,
        prompt: &str,
        config: &WritingToolsConfig,
        model: &str,
    ) -> Result<String, WritingToolsError> {
        let usage_handle = self
            .usage
            .start_request(&config.provider, model)
            .await?;
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
            model: model.to_string(),
            messages,
            temperature: 0.2,
        };

        let response = match self
            .client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                self.usage.finish_failure(&usage_handle).await?;
                return Err(WritingToolsError::Http(err));
            }
        };

        if !response.status().is_success() {
            self.usage.finish_failure(&usage_handle).await?;
            return Err(WritingToolsError::HttpStatus(response.status()));
        }

        let body: ChatResponse = match response.json().await {
            Ok(body) => body,
            Err(err) => {
                self.usage.finish_failure(&usage_handle).await?;
                return Err(WritingToolsError::Http(err));
            }
        };
        let Some(content) = body
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
        else {
            self.usage.finish_failure(&usage_handle).await?;
            return Err(WritingToolsError::EmptyResponse);
        };
        let Some(usage) = body.usage else {
            self.usage.finish_failure(&usage_handle).await?;
            return Err(WritingToolsError::MissingUsage);
        };

        self.usage
            .finish_success(
                &usage_handle,
                usage.prompt,
                usage.completion,
                usage.total,
            )
            .await?;

        Ok(content.trim().to_string())
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
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    #[serde(rename = "prompt_tokens")]
    prompt: u32,
    #[serde(rename = "completion_tokens")]
    completion: u32,
    #[serde(rename = "total_tokens")]
    total: u32,
}

fn default_base_url() -> &'static str {
    "https://api.openai.com/v1"
}

fn default_model() -> &'static str {
    "gpt-4.1-mini"
}

fn default_fallback_model() -> &'static str {
    "gpt-4o-mini"
}

fn default_provider() -> &'static str {
    "openai"
}

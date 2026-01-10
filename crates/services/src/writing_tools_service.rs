use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::WritingToolsError;

#[derive(Clone, Debug)]
pub struct WritingToolsConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl WritingToolsConfig {
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let api_key = env::var("LEARN_AI_API_KEY").ok()?;
        if api_key.trim().is_empty() {
            return None;
        }
        let base_url =
            env::var("LEARN_AI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".into());
        let model = env::var("LEARN_AI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
        Some(Self {
            base_url,
            api_key,
            model,
        })
    }
}

#[derive(Clone)]
pub struct WritingToolsService {
    client: Client,
    config: Option<WritingToolsConfig>,
}

impl WritingToolsService {
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(WritingToolsConfig::from_env())
    }

    #[must_use]
    pub fn new(config: Option<WritingToolsConfig>) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    #[must_use]
    pub fn enabled(&self) -> bool {
        self.config.is_some()
    }

    /// Generate text from a prompt.
    ///
    /// # Errors
    ///
    /// Returns `WritingToolsError` when the service is disabled, the request fails,
    /// or the response is empty.
    pub async fn generate(&self, prompt: &str) -> Result<String, WritingToolsError> {
        let config = self
            .config
            .as_ref()
            .ok_or(WritingToolsError::Disabled)?;

        let url = format!(
            "{}/chat/completions",
            config.base_url.trim_end_matches('/')
        );
        let payload = ChatRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage {
                role: "user",
                content: prompt.to_string(),
            }],
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

use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};

use crate::ai::pricing::PriceBook;
use crate::error::AiUsageError;
use crate::Clock;
use storage::repository::{
    AiPriceBookEntry, AiPriceBookRepository, AiUsageCompletion, AiUsageRepository, AiUsageStatus,
    AppSettingsRepository, NewAiUsageRecord,
};

#[derive(Clone, Debug)]
pub struct AiUsageHandle {
    pub id: i64,
    pub provider: String,
    pub model: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AiUsageService {
    clock: Clock,
    settings_repo: Arc<dyn AppSettingsRepository>,
    usage_repo: Arc<dyn AiUsageRepository>,
    price_book: PriceBook,
}

impl AiUsageService {
    #[must_use]
    pub fn new(
        clock: Clock,
        settings_repo: Arc<dyn AppSettingsRepository>,
        usage_repo: Arc<dyn AiUsageRepository>,
        price_repo: Arc<dyn AiPriceBookRepository>,
    ) -> Self {
        Self {
            clock,
            settings_repo,
            usage_repo,
            price_book: PriceBook::new(price_repo),
        }
    }

    /// Start a request after enforcing cooldown, daily cap, and monthly budget.
    ///
    /// # Errors
    ///
    /// Returns `AiUsageError` if limits are exceeded or persistence fails.
    pub async fn start_request(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<AiUsageHandle, AiUsageError> {
        let settings = self
            .settings_repo
            .get_settings()
            .await?
            .unwrap_or_default();
        let now = self.clock.now();

        let daily_start = start_of_day(now);
        let requests_today = self.usage_repo.count_since(daily_start).await?;
        if requests_today >= settings.ai_daily_request_cap() {
            return Err(AiUsageError::DailyCapReached {
                cap: settings.ai_daily_request_cap(),
            });
        }

        if let Some(last_request_at) = self.usage_repo.last_request_at().await? {
            let cooldown = Duration::seconds(i64::from(settings.ai_cooldown_secs()));
            if last_request_at + cooldown > now {
                let remaining_secs = (last_request_at + cooldown - now).num_seconds().max(0);
                let remaining = u32::try_from(remaining_secs).unwrap_or(u32::MAX);
                return Err(AiUsageError::CooldownActive { remaining_secs: remaining });
            }
        }

        let id = self
            .usage_repo
            .insert_started(NewAiUsageRecord {
                provider: provider.to_string(),
                model: model.to_string(),
                created_at: now,
            })
            .await?;

        Ok(AiUsageHandle {
            id,
            provider: provider.to_string(),
            model: model.to_string(),
            started_at: now,
        })
    }

    /// Record a successful request and compute the cost.
    ///
    /// # Errors
    ///
    /// Returns `AiUsageError` if pricing metadata is missing or persistence fails.
    pub async fn finish_success(
        &self,
        handle: &AiUsageHandle,
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    ) -> Result<u64, AiUsageError> {
        let Some(cost) = self
            .price_book
            .estimate_cost_micro_usd(
                &handle.provider,
                &handle.model,
                prompt_tokens,
                completion_tokens,
            )
            .await?
        else {
            self.usage_repo
                .update_completion(
                    handle.id,
                    AiUsageCompletion {
                        status: AiUsageStatus::Failed,
                        prompt_tokens: None,
                        completion_tokens: None,
                        total_tokens: None,
                        cost_micro_usd: None,
                    },
                )
                .await?;
            return Err(AiUsageError::MissingPriceEntry {
                provider: handle.provider.clone(),
                model: handle.model.clone(),
            });
        };

        self.usage_repo
            .update_completion(
                handle.id,
                AiUsageCompletion {
                    status: AiUsageStatus::Succeeded,
                    prompt_tokens: Some(prompt_tokens),
                    completion_tokens: Some(completion_tokens),
                    total_tokens: Some(total_tokens),
                    cost_micro_usd: Some(cost),
                },
            )
            .await?;

        Ok(cost)
    }

    /// Record a failed request.
    ///
    /// # Errors
    ///
    /// Returns `AiUsageError` if persistence fails.
    pub async fn finish_failure(&self, handle: &AiUsageHandle) -> Result<(), AiUsageError> {
        self.usage_repo
            .update_completion(
                handle.id,
                AiUsageCompletion {
                    status: AiUsageStatus::Failed,
                    prompt_tokens: None,
                    completion_tokens: None,
                    total_tokens: None,
                    cost_micro_usd: None,
                },
            )
            .await?;
        Ok(())
    }

    /// List the current price book entries.
    ///
    /// # Errors
    ///
    /// Returns `AiUsageError` on persistence failures.
    pub async fn price_entries(&self) -> Result<Vec<AiPriceBookEntry>, AiUsageError> {
        Ok(self.price_book.list_entries().await?)
    }
}

fn start_of_day(now: DateTime<Utc>) -> DateTime<Utc> {
    let date = now.date_naive();
    Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .single()
        .unwrap_or(now)
}

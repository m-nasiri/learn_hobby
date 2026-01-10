use std::sync::Arc;

use storage::repository::{AiPriceBookEntry, AiPriceBookRepository, StorageError};

#[derive(Clone)]
pub struct PriceBook {
    repo: Arc<dyn AiPriceBookRepository>,
}

impl PriceBook {
    #[must_use]
    pub fn new(repo: Arc<dyn AiPriceBookRepository>) -> Self {
        Self { repo }
    }

    /// Fetch a pricing entry for the given provider/model.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on persistence failures.
    pub async fn get_entry(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<Option<AiPriceBookEntry>, StorageError> {
        self.repo.get_entry(provider, model).await
    }

    /// List all pricing entries.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on persistence failures.
    pub async fn list_entries(&self) -> Result<Vec<AiPriceBookEntry>, StorageError> {
        self.repo.list_entries().await
    }

    /// Estimate the cost in micro-USD for the given token usage.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` on persistence failures.
    pub async fn estimate_cost_micro_usd(
        &self,
        provider: &str,
        model: &str,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> Result<Option<u64>, StorageError> {
        let Some(entry) = self.repo.get_entry(provider, model).await? else {
            return Ok(None);
        };

        Ok(Some(estimate_cost_micro_usd(
            entry.input_micro_usd_per_million,
            entry.output_micro_usd_per_million,
            prompt_tokens,
            completion_tokens,
        )))
    }
}

fn estimate_cost_micro_usd(
    input_micro_usd_per_million: u64,
    output_micro_usd_per_million: u64,
    prompt_tokens: u32,
    completion_tokens: u32,
) -> u64 {
    let prompt_cost = u64::from(prompt_tokens)
        .saturating_mul(input_micro_usd_per_million)
        / 1_000_000;
    let completion_cost = u64::from(completion_tokens)
        .saturating_mul(output_micro_usd_per_million)
        / 1_000_000;
    prompt_cost.saturating_add(completion_cost)
}

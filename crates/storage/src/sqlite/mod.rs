use std::sync::Arc;
use std::time::Duration;

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use thiserror::Error;

use crate::repository::{
    AiPriceBookRepository, AiUsageRepository, AppSettingsRepository, CardRepository, DeckRepository,
    ReviewLogRepository, ReviewPersistence, SessionSummaryRepository, Storage,
};

mod ai_price_book_repo;
mod ai_usage_repo;
mod app_settings_repo;
mod card_repo;
mod deck_repo;
mod mapping;
mod migrate;
mod review_log_repo;
mod session_summary_repo;

#[derive(Clone)]
pub struct SqliteRepository {
    pool: SqlitePool,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SqliteInitError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl SqliteRepository {
    /// Connect to `SQLite` using the given URL.
    ///
    /// # Errors
    ///
    /// Returns `SqliteInitError` if the connection cannot be established or if
    /// enforcing foreign key constraints fails during setup.
    pub async fn connect(database_url: &str) -> Result<Self, SqliteInitError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    sqlx::query("PRAGMA foreign_keys = ON;")
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("PRAGMA journal_mode = WAL;")
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("PRAGMA busy_timeout = 5000;")
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Create tables if they do not exist.
    ///
    /// # Errors
    ///
    /// Returns `SqliteInitError` if migration queries fail.
    pub async fn migrate(&self) -> Result<(), SqliteInitError> {
        migrate::run_migrations(&self.pool).await
    }
}

impl Storage {
    /// Build a `Storage` backed by `SQLite`.
    ///
    /// # Errors
    ///
    /// Returns `SqliteInitError` if connection or migrations cannot be
    /// completed.
    pub async fn sqlite(database_url: &str) -> Result<Self, SqliteInitError> {
        let repo = SqliteRepository::connect(database_url).await?;
        repo.migrate().await?;

        let deck_repo: Arc<dyn DeckRepository> = Arc::new(repo.clone());
        let card_repo: Arc<dyn CardRepository> = Arc::new(repo.clone());
        let log_repo: Arc<dyn ReviewLogRepository> = Arc::new(repo.clone());
        let review_repo: Arc<dyn ReviewPersistence> = Arc::new(repo.clone());
        let summary_repo: Arc<dyn SessionSummaryRepository> = Arc::new(repo.clone());
        let app_settings_repo: Arc<dyn AppSettingsRepository> = Arc::new(repo.clone());
        let ai_price_book_repo: Arc<dyn AiPriceBookRepository> = Arc::new(repo.clone());
        let ai_usage_repo: Arc<dyn AiUsageRepository> = Arc::new(repo);
        Ok(Self {
            decks: deck_repo,
            cards: card_repo,
            review_logs: log_repo,
            reviews: review_repo,
            session_summaries: summary_repo,
            app_settings: app_settings_repo,
            ai_price_book: ai_price_book_repo,
            ai_usage: ai_usage_repo,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SqliteRepository>();
    }
}

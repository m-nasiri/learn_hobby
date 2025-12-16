use std::sync::Arc;
use std::time::Duration;

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use thiserror::Error;

use crate::repository::{CardRepository, DeckRepository, ReviewLogRepository, Storage};

mod card_repo;
mod deck_repo;
mod mapping;
mod migrate;
mod review_log_repo;

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
        let log_repo: Arc<dyn ReviewLogRepository> = Arc::new(repo);
        Ok(Self {
            decks: deck_repo,
            cards: card_repo,
            review_logs: log_repo,
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

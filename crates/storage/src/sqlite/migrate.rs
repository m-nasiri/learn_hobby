use chrono::Utc;
use sqlx::SqlitePool;

use super::SqliteInitError;

/// Runs a single, consolidated migration for the current schema.
///
/// creates the full schema (decks, cards with media, review logs, and indexes).
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), SqliteInitError> {
    async fn is_applied(pool: &SqlitePool, version: i64) -> Result<bool, sqlx::Error> {
        let row = sqlx::query("SELECT 1 FROM schema_migrations WHERE version = ?1")
            .bind(version)
            .fetch_optional(pool)
            .await?;
        Ok(row.is_some())
    }

    sqlx::query(
        r"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );
            ",
    )
    .execute(pool)
    .await?;

    // Version 1: full schema.
    if !is_applied(pool, 1).await? {
        let mut tx = pool.begin().await?;

        sqlx::query(
            r"
                CREATE TABLE IF NOT EXISTS decks (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    created_at TEXT NOT NULL,
                    new_cards_per_day INTEGER NOT NULL,
                    review_limit_per_day INTEGER NOT NULL,
                    micro_session_size INTEGER NOT NULL
                );
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE TABLE IF NOT EXISTS cards (
                    id INTEGER NOT NULL,
                    deck_id INTEGER NOT NULL,
                    prompt TEXT NOT NULL,
                    prompt_media_id INTEGER,
                    answer TEXT NOT NULL,
                    answer_media_id INTEGER,
                    phase TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    next_review_at TEXT NOT NULL,
                    last_review_at TEXT,
                    review_count INTEGER NOT NULL,
                    stability REAL,
                    difficulty REAL,
                    PRIMARY KEY (id, deck_id),
                    FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE
                );
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE TABLE IF NOT EXISTS review_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    deck_id INTEGER NOT NULL,
                    card_id INTEGER NOT NULL,
                    grade INTEGER NOT NULL,
                    reviewed_at TEXT NOT NULL,
                    elapsed_days REAL NOT NULL,
                    scheduled_days REAL NOT NULL,
                    stability REAL NOT NULL,
                    difficulty REAL NOT NULL,
                    next_review_at TEXT NOT NULL,
                    FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE,
                    FOREIGN KEY (card_id, deck_id) REFERENCES cards(id, deck_id) ON DELETE CASCADE
                );
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE INDEX IF NOT EXISTS idx_cards_deck_next_review
                    ON cards(deck_id, next_review_at);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE INDEX IF NOT EXISTS idx_cards_deck_reviewcount_created
                    ON cards(deck_id, review_count, created_at, id);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE INDEX IF NOT EXISTS review_logs_deck_card_idx
                    ON review_logs (deck_id, card_id, reviewed_at);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                INSERT INTO schema_migrations (version, applied_at)
                VALUES (?1, ?2)
                ON CONFLICT(version) DO NOTHING
            ",
        )
        .bind(1_i64)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    Ok(())
}

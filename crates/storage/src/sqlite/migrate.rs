use chrono::Utc;
use sqlx::SqlitePool;

use super::SqliteInitError;

/// Runs a single, consolidated migration for the current schema.
///
/// Creates the full schema (decks, cards with media, review logs, session summaries, and indexes).
#[allow(clippy::too_many_lines)]
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
                    new_cards_per_day INTEGER NOT NULL CHECK (new_cards_per_day >= 0),
                    review_limit_per_day INTEGER NOT NULL CHECK (review_limit_per_day >= 0),
                    micro_session_size INTEGER NOT NULL CHECK (micro_session_size >= 0)
                );
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE TABLE IF NOT EXISTS cards (
                    id INTEGER PRIMARY KEY,
                    deck_id INTEGER NOT NULL,
                    prompt TEXT NOT NULL,
                    prompt_media_id INTEGER,
                    answer TEXT NOT NULL,
                    answer_media_id INTEGER,
                    phase TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    next_review_at TEXT NOT NULL,
                    last_review_at TEXT,
                    review_count INTEGER NOT NULL CHECK (review_count >= 0),
                    stability REAL,
                    difficulty REAL,
                    FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE
                );
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE TABLE IF NOT EXISTS tags (
                    id INTEGER PRIMARY KEY,
                    deck_id INTEGER NOT NULL,
                    name TEXT NOT NULL,
                    FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE,
                    UNIQUE(deck_id, name)
                );
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE TABLE IF NOT EXISTS card_tags (
                    card_id INTEGER NOT NULL,
                    tag_id INTEGER NOT NULL,
                    PRIMARY KEY (card_id, tag_id),
                    FOREIGN KEY (card_id) REFERENCES cards(id) ON DELETE CASCADE,
                    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
                );
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE TABLE IF NOT EXISTS review_logs (
                    id INTEGER PRIMARY KEY,
                    deck_id INTEGER NOT NULL,
                    card_id INTEGER NOT NULL,
                    grade INTEGER NOT NULL CHECK (grade BETWEEN 0 AND 3),
                    reviewed_at TEXT NOT NULL,
                    elapsed_days REAL NOT NULL,
                    scheduled_days REAL NOT NULL,
                    stability REAL NOT NULL,
                    difficulty REAL NOT NULL,
                    next_review_at TEXT NOT NULL,
                    FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE,
                    FOREIGN KEY (card_id) REFERENCES cards(id) ON DELETE CASCADE
                );
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE TABLE IF NOT EXISTS session_summaries (
                    id INTEGER PRIMARY KEY,
                    deck_id INTEGER NOT NULL,
                    started_at TEXT NOT NULL,
                    completed_at TEXT NOT NULL,
                    total_reviews INTEGER NOT NULL CHECK (total_reviews >= 0),
                    again INTEGER NOT NULL CHECK (again >= 0),
                    hard INTEGER NOT NULL CHECK (hard >= 0),
                    good INTEGER NOT NULL CHECK (good >= 0),
                    easy INTEGER NOT NULL CHECK (easy >= 0),
                    FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE
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
                CREATE INDEX IF NOT EXISTS idx_tags_deck_name
                    ON tags(deck_id, name);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE INDEX IF NOT EXISTS idx_card_tags_card
                    ON card_tags(card_id, tag_id);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE INDEX IF NOT EXISTS idx_card_tags_tag
                    ON card_tags(tag_id, card_id);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE INDEX IF NOT EXISTS idx_review_logs_deck_card_reviewed_at
                    ON review_logs (deck_id, card_id, reviewed_at);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                CREATE INDEX IF NOT EXISTS idx_session_summaries_deck_completed
                    ON session_summaries (deck_id, completed_at);
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

    if !is_applied(pool, 2).await? {
        let mut tx = pool.begin().await?;
        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN protect_overload INTEGER NOT NULL DEFAULT 1 CHECK (protect_overload IN (0, 1));
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
        .bind(2_i64)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    if !is_applied(pool, 3).await? {
        let mut tx = pool.begin().await?;
        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN preserve_stability_on_lapse INTEGER NOT NULL DEFAULT 1
                    CHECK (preserve_stability_on_lapse IN (0, 1));
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN lapse_min_interval_days INTEGER NOT NULL DEFAULT 1
                    CHECK (lapse_min_interval_days >= 1);
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
        .bind(3_i64)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    if !is_applied(pool, 4).await? {
        let mut tx = pool.begin().await?;
        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN lapse_min_interval_secs INTEGER NOT NULL DEFAULT 86400
                    CHECK (lapse_min_interval_secs >= 1);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                UPDATE decks
                SET lapse_min_interval_secs = lapse_min_interval_days * 86400
                WHERE lapse_min_interval_secs = 86400;
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
        .bind(4_i64)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    if !is_applied(pool, 5).await? {
        let mut tx = pool.begin().await?;
        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN fsrs_target_retention REAL NOT NULL DEFAULT 0.85
                    CHECK (fsrs_target_retention > 0 AND fsrs_target_retention <= 1);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN fsrs_optimize_enabled INTEGER NOT NULL DEFAULT 1
                    CHECK (fsrs_optimize_enabled IN (0, 1));
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN fsrs_optimize_after INTEGER NOT NULL DEFAULT 100
                    CHECK (fsrs_optimize_after >= 1);
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
        .bind(5_i64)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    if !is_applied(pool, 7).await? {
        let mut tx = pool.begin().await?;
        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN show_timer INTEGER NOT NULL DEFAULT 0
                    CHECK (show_timer IN (0, 1));
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN soft_time_reminder INTEGER NOT NULL DEFAULT 0
                    CHECK (soft_time_reminder IN (0, 1));
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN auto_advance_cards INTEGER NOT NULL DEFAULT 0
                    CHECK (auto_advance_cards IN (0, 1));
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
        .bind(7_i64)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    if !is_applied(pool, 8).await? {
        let mut tx = pool.begin().await?;
        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN soft_time_reminder_secs INTEGER NOT NULL DEFAULT 25
                    CHECK (soft_time_reminder_secs BETWEEN 5 AND 600);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN auto_reveal_secs INTEGER NOT NULL DEFAULT 20
                    CHECK (auto_reveal_secs BETWEEN 5 AND 600);
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
        .bind(8_i64)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    if !is_applied(pool, 9).await? {
        let mut tx = pool.begin().await?;
        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN easy_days_enabled INTEGER NOT NULL DEFAULT 1
                    CHECK (easy_days_enabled IN (0, 1));
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN easy_day_load_factor REAL NOT NULL DEFAULT 0.5
                    CHECK (easy_day_load_factor > 0 AND easy_day_load_factor <= 1);
            ",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
                ALTER TABLE decks
                ADD COLUMN easy_days_mask INTEGER NOT NULL DEFAULT 96
                    CHECK (easy_days_mask BETWEEN 0 AND 127);
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
        .bind(9_i64)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    Ok(())
}

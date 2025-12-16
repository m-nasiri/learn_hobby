use chrono::Utc;
use sqlx::{Row, SqlitePool};

use super::SqliteInitError;

#[allow(clippy::too_many_lines)]
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), SqliteInitError> {
    async fn is_applied(pool: &SqlitePool, version: i64) -> Result<bool, sqlx::Error> {
        let row = sqlx::query("SELECT 1 FROM schema_migrations WHERE version = ?1")
            .bind(version)
            .fetch_optional(pool)
            .await?;
        Ok(row.is_some())
    }

    // Ensure we can store applied migration versions.
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

    // ─── Migration V1: initial tables ─────────────────────────────────────
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
                    answer TEXT NOT NULL,
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

    // ─── Migration V2: ensure stability/difficulty are nullable ───────────
    if !is_applied(pool, 2).await? {
        let mut tx = pool.begin().await?;

        let cols = sqlx::query(r"PRAGMA table_info(cards);")
            .fetch_all(&mut *tx)
            .await?;

        let mut stability_notnull = false;
        let mut difficulty_notnull = false;

        for row in cols {
            let name: String = row.try_get("name")?;
            let notnull: i64 = row.try_get("notnull")?;
            if name == "stability" {
                stability_notnull = notnull == 1;
            }
            if name == "difficulty" {
                difficulty_notnull = notnull == 1;
            }
        }

        if stability_notnull || difficulty_notnull {
            // already in transaction

            sqlx::query("PRAGMA foreign_keys=OFF;")
                .execute(&mut *tx)
                .await?;

            sqlx::query(
                r"
                    CREATE TABLE IF NOT EXISTS cards_new (
                        id INTEGER NOT NULL,
                        deck_id INTEGER NOT NULL,
                        prompt TEXT NOT NULL,
                        answer TEXT NOT NULL,
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
                    INSERT INTO cards_new (
                        id, deck_id, prompt, answer, phase, created_at,
                        next_review_at, last_review_at, review_count, stability, difficulty
                    )
                    SELECT
                        id, deck_id, prompt, answer, phase, created_at,
                        next_review_at, last_review_at, review_count, stability, difficulty
                    FROM cards;
                    ",
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query("DROP TABLE cards;").execute(&mut *tx).await?;

            sqlx::query("ALTER TABLE cards_new RENAME TO cards;")
                .execute(&mut *tx)
                .await?;

            sqlx::query("PRAGMA foreign_keys=ON;")
                .execute(&mut *tx)
                .await?;
        }

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

    Ok(())
}

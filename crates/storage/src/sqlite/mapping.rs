use learn_core::model::{Card, CardId, CardPhase, DeckId, content::ContentDraft};
use sqlx::Row;

use crate::repository::StorageError;

fn ser<E: core::fmt::Display>(e: E) -> StorageError {
    StorageError::Serialization(e.to_string())
}

fn i64_to_u64(field: &'static str, v: i64) -> Result<u64, StorageError> {
    u64::try_from(v).map_err(|_| StorageError::Serialization(format!("{field} sign overflow")))
}

fn deck_id_from_i64(v: i64) -> Result<DeckId, StorageError> {
    Ok(DeckId::new(i64_to_u64("deck_id", v)?))
}

fn card_id_from_i64(v: i64) -> Result<CardId, StorageError> {
    Ok(CardId::new(i64_to_u64("id", v)?))
}

pub(crate) fn parse_card_phase(s: &str) -> Result<CardPhase, StorageError> {
    match s {
        "new" => Ok(CardPhase::New),
        "learning" => Ok(CardPhase::Learning),
        "reviewing" => Ok(CardPhase::Reviewing),
        "relearning" => Ok(CardPhase::Relearning),
        _ => Err(StorageError::Serialization(format!("invalid phase: {s}"))),
    }
}

pub(crate) fn map_card_row(row: &sqlx::sqlite::SqliteRow) -> Result<Card, StorageError> {
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").map_err(ser)?;

    let prompt = ContentDraft::text_only(row.try_get::<String, _>("prompt").map_err(ser)?)
        .validate(created_at, None, None)
        .map_err(ser)?;

    let answer = ContentDraft::text_only(row.try_get::<String, _>("answer").map_err(ser)?)
        .validate(created_at, None, None)
        .map_err(ser)?;

    let phase_str: String = row.try_get("phase").map_err(ser)?;
    let phase = parse_card_phase(phase_str.as_str())?;

    let review_count_i64: i64 = row.try_get("review_count").map_err(ser)?;
    let review_count: u32 = u32::try_from(review_count_i64)
        .map_err(|_| StorageError::Serialization("review_count overflow".into()))?;

    let stability: f64 = if review_count == 0 {
        0.0
    } else {
        row.try_get::<Option<f64>, _>("stability")
            .map_err(ser)?
            .ok_or_else(|| StorageError::Serialization("missing stability".into()))?
    };

    let difficulty: f64 = if review_count == 0 {
        0.0
    } else {
        row.try_get::<Option<f64>, _>("difficulty")
            .map_err(ser)?
            .ok_or_else(|| StorageError::Serialization("missing difficulty".into()))?
    };

    Card::from_persisted(
        card_id_from_i64(row.try_get::<i64, _>("id").map_err(ser)?)?,
        deck_id_from_i64(row.try_get::<i64, _>("deck_id").map_err(ser)?)?,
        prompt,
        answer,
        created_at,
        row.try_get("next_review_at").map_err(ser)?,
        row.try_get("last_review_at").map_err(ser)?,
        phase,
        review_count,
        stability,
        difficulty,
    )
    .map_err(ser)
}

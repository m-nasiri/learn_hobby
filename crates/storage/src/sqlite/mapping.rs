use learn_core::model::{Card, CardId, CardPhase, DeckId, ReviewGrade, content::Content};
use sqlx::Row;

use crate::repository::StorageError;

fn ser<E: core::fmt::Display>(e: E) -> StorageError {
    StorageError::Serialization(e.to_string())
}

fn i64_to_u64(field: &'static str, v: i64) -> Result<u64, StorageError> {
    u64::try_from(v).map_err(|_| StorageError::Serialization(format!("{field} sign overflow")))
}

pub(crate) fn deck_id_from_i64(v: i64) -> Result<DeckId, StorageError> {
    Ok(DeckId::new(i64_to_u64("deck_id", v)?))
}

pub(crate) fn card_id_from_i64(v: i64) -> Result<CardId, StorageError> {
    Ok(CardId::new(i64_to_u64("card_id", v)?))
}

pub(crate) fn media_id_from_i64(v: i64) -> Result<learn_core::model::MediaId, StorageError> {
    Ok(learn_core::model::MediaId::new(i64_to_u64("media_id", v)?))
}

pub(crate) fn media_id_to_i64(
    mid: Option<learn_core::model::MediaId>,
) -> Result<Option<i64>, StorageError> {
    mid.map(|m| {
        i64::try_from(m.value())
            .map_err(|_| StorageError::Serialization("media_id overflow".into()))
    })
    .transpose()
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

    let prompt = Content::from_persisted(
        row.try_get::<String, _>("prompt").map_err(ser)?,
        row.try_get::<Option<i64>, _>("prompt_media_id")
            .map_err(ser)?
            .map(media_id_from_i64)
            .transpose()?,
    )
    .map_err(ser)?;

    let answer = Content::from_persisted(
        row.try_get::<String, _>("answer").map_err(ser)?,
        row.try_get::<Option<i64>, _>("answer_media_id")
            .map_err(ser)?
            .map(media_id_from_i64)
            .transpose()?,
    )
    .map_err(ser)?;

    let phase_str: String = row.try_get("phase").map_err(ser)?;
    let phase = parse_card_phase(phase_str.as_str())?;

    let review_count_i64: i64 = row.try_get("review_count").map_err(ser)?;
    let review_count: u32 = u32::try_from(review_count_i64).map_err(|_| {
        StorageError::Serialization(format!("invalid review_count: {review_count_i64}"))
    })?;

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

/// Converts a `ReviewGrade` to its storage representation.
/// Storage encoding uses 0..=3:
/// Again=0, Hard=1, Good=2, Easy=3.
/// (FSRS ratings use 1..=4 and are handled separately.)
pub(crate) fn grade_to_i64(grade: ReviewGrade) -> i64 {
    match grade {
        ReviewGrade::Again => 0,
        ReviewGrade::Hard => 1,
        ReviewGrade::Good => 2,
        ReviewGrade::Easy => 3,
    }
}

/// Converts a stored integer grade (0..=3) back into `ReviewGrade`.
/// This must stay consistent with `grade_to_i64`.
pub(crate) fn grade_from_i64(value: i64) -> Result<ReviewGrade, StorageError> {
    match value {
        0 => Ok(ReviewGrade::Again),
        1 => Ok(ReviewGrade::Hard),
        2 => Ok(ReviewGrade::Good),
        3 => Ok(ReviewGrade::Easy),
        other => Err(StorageError::Serialization(format!(
            "invalid grade: {other}"
        ))),
    }
}

pub(crate) fn map_review_log_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<crate::repository::ReviewLogRecord, StorageError> {
    use crate::repository::ReviewLogRecord;

    let reviewed_at = row.try_get("reviewed_at").map_err(ser)?;
    Ok(ReviewLogRecord {
        id: Some(row.try_get("id").map_err(ser)?),
        deck_id: deck_id_from_i64(row.try_get::<i64, _>("deck_id").map_err(ser)?)?,
        card_id: card_id_from_i64(row.try_get::<i64, _>("card_id").map_err(ser)?)?,
        grade: grade_from_i64(row.try_get::<i64, _>("grade").map_err(ser)?)?,
        reviewed_at,
        elapsed_days: row.try_get("elapsed_days").map_err(ser)?,
        scheduled_days: row.try_get("scheduled_days").map_err(ser)?,
        stability: row.try_get("stability").map_err(ser)?,
        difficulty: row.try_get("difficulty").map_err(ser)?,
        next_review_at: row.try_get("next_review_at").map_err(ser)?,
    })
}

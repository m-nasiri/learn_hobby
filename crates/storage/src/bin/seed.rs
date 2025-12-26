use std::fmt;

use chrono::{DateTime, Duration, Utc};
use learn_core::model::{Card, CardId, ContentDraft, Deck, DeckId, DeckSettings, SessionSummary};
use storage::repository::{NewDeckRecord, Storage};

#[derive(Debug, Clone)]
struct Args {
    db_url: String,
    deck_id: DeckId,
    deck_name: String,
    deck_desc: Option<String>,
    summaries: u32,
    cards: u32,
    now: Option<DateTime<Utc>>,
}

#[derive(Debug)]
enum ArgsError {
    MissingValue { flag: &'static str },
    UnknownArg(String),
    InvalidDeckId { raw: String },
    InvalidSummaries { raw: String },
    InvalidDbUrl { raw: String },
    InvalidNow { raw: String },
    InvalidCards { raw: String },
}

impl fmt::Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgsError::MissingValue { flag } => write!(f, "{flag} requires a value"),
            ArgsError::UnknownArg(arg) => write!(f, "unknown argument: {arg}"),
            ArgsError::InvalidDeckId { raw } => write!(f, "invalid --deck-id value: {raw}"),
            ArgsError::InvalidSummaries { raw } => write!(f, "invalid --summaries value: {raw}"),
            ArgsError::InvalidDbUrl { raw } => write!(f, "invalid --db value: {raw}"),
            ArgsError::InvalidNow { raw } => {
                write!(f, "invalid --now value (expected RFC3339): {raw}")
            }
            ArgsError::InvalidCards { raw } => write!(f, "invalid --cards value: {raw}"),
        }
    }
}

impl std::error::Error for ArgsError {}

fn require_value(
    args: &mut impl Iterator<Item = String>,
    flag: &'static str,
) -> Result<String, ArgsError> {
    args.next().ok_or(ArgsError::MissingValue { flag })
}

impl Args {
    fn parse() -> Result<Self, ArgsError> {
        let mut db_url =
            std::env::var("LEARN_DB_URL").unwrap_or_else(|_| "sqlite:dev.sqlite3".into());
        let mut deck_id = std::env::var("LEARN_DECK_ID")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map_or_else(|| DeckId::new(1), DeckId::new);
        let mut deck_name = std::env::var("LEARN_DECK_NAME").unwrap_or_else(|_| "Japanese".into());
        let mut deck_desc = std::env::var("LEARN_DECK_DESC").ok();
        let mut summaries = std::env::var("LEARN_SUMMARIES")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(3);
        let mut cards = std::env::var("LEARN_CARDS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(5);
        let mut now: Option<DateTime<Utc>> = None;

        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--db" => {
                    let value = require_value(&mut args, "--db")?;
                    if value.trim().is_empty() {
                        return Err(ArgsError::InvalidDbUrl { raw: value });
                    }
                    db_url = value;
                }
                "--deck-id" => {
                    let value = require_value(&mut args, "--deck-id")?;
                    let parsed: u64 = value
                        .parse()
                        .map_err(|_| ArgsError::InvalidDeckId { raw: value.clone() })?;
                    deck_id = DeckId::new(parsed);
                }
                "--deck-name" => {
                    let value = require_value(&mut args, "--deck-name")?;
                    deck_name = value;
                }
                "--deck-desc" => {
                    let value = require_value(&mut args, "--deck-desc")?;
                    deck_desc = Some(value);
                }
                "--summaries" => {
                    let value = require_value(&mut args, "--summaries")?;
                    summaries = value
                        .parse::<u32>()
                        .map_err(|_| ArgsError::InvalidSummaries { raw: value.clone() })?;
                }
                "--cards" => {
                    let value = require_value(&mut args, "--cards")?;
                    cards = value
                        .parse::<u32>()
                        .map_err(|_| ArgsError::InvalidCards { raw: value.clone() })?;
                }
                "--now" => {
                    let value = require_value(&mut args, "--now")?;
                    let parsed = DateTime::parse_from_rfc3339(&value)
                        .map_err(|_| ArgsError::InvalidNow { raw: value.clone() })?
                        .with_timezone(&Utc);
                    now = Some(parsed);
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                _ => return Err(ArgsError::UnknownArg(arg)),
            }
        }

        Ok(Self {
            db_url,
            deck_id,
            deck_name,
            deck_desc,
            summaries,
            cards,
            now,
        })
    }
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  cargo run -p storage --bin seed -- [options]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --db <sqlite_url>         SQLite URL (default: sqlite:dev.sqlite3)");
    eprintln!("  --deck-id <id>            Deck id to upsert (default: 1)");
    eprintln!("  --deck-name <name>        Deck name (default: Japanese)");
    eprintln!("  --deck-desc <text>        Optional deck description");
    eprintln!("  --summaries <n>           Number of session summaries to append (default: 3)");
    eprintln!("  --cards <n>               Number of sample cards to upsert (default: 5)");
    eprintln!("  --now <rfc3339>           Fixed current time for deterministic seeding");
    eprintln!("  -h, --help                Show this help");
    eprintln!();
    eprintln!("Environment (same as flags):");
    eprintln!(
        "  LEARN_DB_URL, LEARN_DECK_ID, LEARN_DECK_NAME, LEARN_DECK_DESC, LEARN_SUMMARIES, LEARN_CARDS"
    );
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse().map_err(|e| {
        eprintln!("{e}");
        print_usage();
        e
    })?;

    let storage = Storage::sqlite(&args.db_url).await?;
    let now = args.now.unwrap_or_else(Utc::now);

    let deck_id = match storage.decks.get_deck(args.deck_id).await? {
        Some(deck) => {
            let updated = Deck::new(
                deck.id(),
                args.deck_name.clone(),
                args.deck_desc.clone(),
                DeckSettings::default_for_adhd(),
                now,
            )?;
            storage.decks.upsert_deck(&updated).await?;
            deck.id()
        }
        None => {
            let draft = Deck::new(
                DeckId::new(1),
                args.deck_name.clone(),
                args.deck_desc.clone(),
                DeckSettings::default_for_adhd(),
                now,
            )?;
            storage
                .decks
                .insert_new_deck(NewDeckRecord::from_deck(&draft))
                .await?
        }
    };

    let samples = [
        ("Hallo", "Hello"),
        ("Danke", "Thank you"),
        ("Bitte", "Please / You are welcome"),
        ("Tschuss", "Bye"),
        ("Guten Morgen", "Good morning"),
    ];
    for i in 0..args.cards {
        let idx = (i as usize) % samples.len();
        let (prompt_text, answer_text) = samples[idx];
        let prompt = ContentDraft::text_only(prompt_text)
            .validate(now, None, None)?;
        let answer = ContentDraft::text_only(answer_text)
            .validate(now, None, None)?;
        let card = Card::new(
            CardId::new(u64::from(i + 1)),
            deck_id,
            prompt,
            answer,
            now,
            now,
        )?;
        storage.cards.upsert_card(&card).await?;
    }

    for i in 0..args.summaries {
        let days_ago = i64::from(i) * 2;
        let started_at = now - Duration::days(days_ago) - Duration::minutes(10);
        let completed_at = started_at + Duration::minutes(5);

        let summary =
            SessionSummary::from_persisted(deck_id, started_at, completed_at, 5, 1, 1, 2, 1)?;

        let _ = storage.session_summaries.append_summary(&summary).await?;
    }

    println!(
        "Seeded deck {} with {} cards and {} session summaries into {}",
        deck_id.value(),
        args.cards,
        args.summaries,
        args.db_url
    );

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

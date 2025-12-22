use std::fmt;
use std::sync::Arc;

use dioxus::LaunchBuilder;
use learn_core::model::DeckId;
use services::Clock;
use services::session_view::SessionSummaryService;
use storage::repository::Storage;
use ui::{App, UiApp, build_app_context};

#[derive(Debug)]
enum ArgsError {
    MissingValue { flag: &'static str },
    UnknownArg(String),
    InvalidDeckId { raw: String },
    InvalidDbUrl { raw: String },
}

impl fmt::Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgsError::MissingValue { flag } => write!(f, "{flag} requires a value"),
            ArgsError::UnknownArg(arg) => write!(f, "unknown argument: {arg}"),
            ArgsError::InvalidDeckId { raw } => write!(f, "invalid --deck-id value: {raw}"),
            ArgsError::InvalidDbUrl { raw } => write!(f, "invalid --db value: {raw}"),
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

struct DesktopApp {
    deck_id: DeckId,
    session_summaries: Arc<SessionSummaryService>,
}

impl UiApp for DesktopApp {
    fn current_deck_id(&self) -> DeckId {
        self.deck_id
    }

    fn session_summaries(&self) -> Arc<SessionSummaryService> {
        Arc::clone(&self.session_summaries)
    }
}

struct Args {
    db_url: String,
    deck_id: DeckId,
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  cargo run -p app -- ui   [--db <sqlite_url>] [--deck-id <id>]");
    eprintln!("  cargo run -p app -- seed [--db <sqlite_url>] [--deck-id <id>]  # placeholder");
    eprintln!();
    eprintln!("Defaults for ui:");
    eprintln!("  --db sqlite:dev.sqlite3");
    eprintln!("  --deck-id 1");
    eprintln!();
    eprintln!("Environment:");
    eprintln!("  LEARN_DB_URL, LEARN_DECK_ID");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Ui,
    Seed,
}

impl Command {
    fn from_arg(arg: &str) -> Option<Self> {
        match arg {
            "ui" => Some(Self::Ui),
            "seed" => Some(Self::Seed),
            _ => None,
        }
    }
}

impl Args {
    fn parse_ui(args: &mut impl Iterator<Item = String>) -> Result<Self, ArgsError> {
        let mut db_url = std::env::var("LEARN_DB_URL")
            .ok()
            .map(normalize_sqlite_url)
            .unwrap_or_else(|| "sqlite://dev.sqlite3".into());
        let mut deck_id = std::env::var("LEARN_DECK_ID")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(DeckId::new)
            .unwrap_or_else(|| DeckId::new(1));

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--db" => {
                    let value = require_value(args, "--db")?;
                    if value.trim().is_empty() {
                        return Err(ArgsError::InvalidDbUrl { raw: value });
                    }
                    db_url = normalize_sqlite_url(value);
                }
                "--deck-id" => {
                    let value = require_value(args, "--deck-id")?;
                    let parsed: u64 = value
                        .parse()
                        .map_err(|_| ArgsError::InvalidDeckId { raw: value.clone() })?;
                    deck_id = DeckId::new(parsed);
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                _ => return Err(ArgsError::UnknownArg(arg)),
            }
        }

        Ok(Self { db_url, deck_id })
    }

    fn parse_seed(args: &mut impl Iterator<Item = String>) -> Result<Self, ArgsError> {
        // For now, seed uses the same db/deck targeting knobs.
        // This keeps the command useful even before we add concrete seed operations.
        Self::parse_ui(args)
    }
}

fn normalize_sqlite_url(raw: String) -> String {
    if raw == "sqlite::memory:" || raw.starts_with("sqlite://") {
        return raw;
    }

    let trimmed = raw.trim().to_string();
    let path_str = trimmed
        .strip_prefix("sqlite:")
        .unwrap_or(trimmed.as_str())
        .to_string();
    let path = std::path::Path::new(&path_str);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(path)
    };
    format!("sqlite://{}", absolute.display())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();

    // Default behavior: launching UI when no subcommand is provided.
    let cmd = match argv.first().map(String::as_str) {
        None => Command::Ui,
        Some("--help") | Some("-h") => {
            print_usage();
            return Ok(());
        }
        Some(first) if first.starts_with("--") => Command::Ui,
        Some(first) => Command::from_arg(first).ok_or_else(|| {
            eprintln!("unknown subcommand: {first}");
            print_usage();
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "unknown subcommand")
        })?,
    };

    if matches!(cmd, Command::Ui | Command::Seed)
        && argv.first().is_some()
        && !argv[0].starts_with("--")
    {
        argv.remove(0);
    }

    let mut iter = argv.into_iter();
    let args = match cmd {
        Command::Ui => Args::parse_ui(&mut iter),
        Command::Seed => Args::parse_seed(&mut iter),
    }
    .map_err(|e| {
        eprintln!("{e}");
        print_usage();
        e
    })?;

    // Open + migrate SQLite at startup. Keep this in the binary glue so core/services stay pure.
    prepare_sqlite_file(&args.db_url)?;
    let storage = Storage::sqlite(&args.db_url).await?;

    match cmd {
        Command::Ui => {
            let summaries = Arc::new(SessionSummaryService::new(
                Clock::default(),
                storage.session_summaries,
            ));

            let app = DesktopApp {
                deck_id: args.deck_id,
                session_summaries: summaries,
            };

            let context = build_app_context(Arc::new(app));
            LaunchBuilder::desktop().with_context(context).launch(App);
            Ok(())
        }
        Command::Seed => {
            // Placeholder: we intentionally keep seed separate from the UI launch.
            // Next step will add concrete operations (create deck, add cards) once the
            // storage/services API is finalized.
            eprintln!(
                "seed: not implemented yet (db={}, deck_id={}).",
                args.db_url, args.deck_id
            );
            drop(storage);
            Ok(())
        }
    }
}

fn prepare_sqlite_file(db_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if db_url == "sqlite::memory:" {
        return Ok(());
    }

    let path = db_url
        .strip_prefix("sqlite://")
        .ok_or_else(|| ArgsError::InvalidDbUrl {
            raw: db_url.to_string(),
        })?;
    let path = path.split('?').next().unwrap_or(path);
    if path.is_empty() {
        return Err(ArgsError::InvalidDbUrl {
            raw: db_url.to_string(),
        }
        .into());
    }

    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if !path.exists() {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        // At this layer (binary glue), printing once is fine.
        eprintln!("{err}");
        std::process::exit(2);
    }
}

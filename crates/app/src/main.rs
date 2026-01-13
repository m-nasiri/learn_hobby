use std::fmt;
use std::sync::Arc;

use dioxus::desktop::{Config as DesktopConfig, LogicalSize, WindowBuilder};
use dioxus::LaunchBuilder;
use learn_core::model::DeckId;
use services::{
    AppServices, AppSettingsService, CardService, Clock, DeckService, SessionLoopService,
    SessionSummaryService, WritingToolsService,
};
use ui::{App, UiApp, UiLinkOpener, build_app_context};
use ui::platform::DesktopLinkOpener;

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
    session_loop: Arc<SessionLoopService>,
    card_service: Arc<CardService>,
    deck_service: Arc<DeckService>,
    app_settings: Arc<AppSettingsService>,
    writing_tools: Arc<WritingToolsService>,
    open_editor_on_launch: bool,
    link_opener: Arc<dyn UiLinkOpener>,
}

impl UiApp for DesktopApp {
    fn current_deck_id(&self) -> DeckId {
        self.deck_id
    }

    fn session_summaries(&self) -> Arc<SessionSummaryService> {
        Arc::clone(&self.session_summaries)
    }

    fn session_loop(&self) -> Arc<SessionLoopService> {
        Arc::clone(&self.session_loop)
    }

    fn card_service(&self) -> Arc<CardService> {
        Arc::clone(&self.card_service)
    }

    fn deck_service(&self) -> Arc<DeckService> {
        Arc::clone(&self.deck_service)
    }

    fn app_settings(&self) -> Arc<AppSettingsService> {
        Arc::clone(&self.app_settings)
    }

    fn writing_tools(&self) -> Arc<WritingToolsService> {
        Arc::clone(&self.writing_tools)
    }

    fn open_editor_on_launch(&self) -> bool {
        self.open_editor_on_launch
    }

    fn link_opener(&self) -> Arc<dyn UiLinkOpener> {
        Arc::clone(&self.link_opener)
    }
}

struct Args {
    db_url: String,
    deck_id: DeckId,
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  cargo run -p app -- ui [--db <sqlite_url>] [--deck-id <id>]");
    eprintln!("  cargo run -p app -- seed [--db <sqlite_url>] [--deck-id <id>]");
    eprintln!();
    eprintln!("Defaults for ui:");
    eprintln!("  --db sqlite://dev.sqlite3");
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
            .map_or_else(|| "sqlite://dev.sqlite3".into(), |value| normalize_sqlite_url(&value));
        let mut deck_id = std::env::var("LEARN_DECK_ID")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map_or_else(|| DeckId::new(1), DeckId::new);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--db" => {
                    let value = require_value(args, "--db")?;
                    if value.trim().is_empty() {
                        return Err(ArgsError::InvalidDbUrl { raw: value });
                    }
                    db_url = normalize_sqlite_url(&value);
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

fn normalize_sqlite_url(raw: &str) -> String {
    let trimmed = raw.trim();

    // Preserve in-memory.
    if trimmed == "sqlite::memory:" {
        return trimmed.to_string();
    }

    // Already a full sqlite URL.
    if trimmed.starts_with("sqlite://") {
        return trimmed.to_string();
    }

    // Accept `sqlite:relative/path.db` and plain paths.
    let path_str = trimmed.strip_prefix("sqlite:").unwrap_or(trimmed);

    // Handle accidental empty input early.
    if path_str.trim().is_empty() {
        return trimmed.to_string();
    }

    let path = std::path::Path::new(path_str);
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
        Some("--help" | "-h") => {
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

    if matches!(cmd, Command::Ui | Command::Seed) && !argv.is_empty() && !argv[0].starts_with("--")
    {
        argv.remove(0);
    }

    let mut iter = argv.into_iter();
    let parsed = match cmd {
        Command::Ui => Args::parse_ui(&mut iter),
        Command::Seed => Args::parse_seed(&mut iter),
    }
    .map_err(|e| {
        eprintln!("{e}");
        print_usage();
        e
    })?;

    // Open + migrate SQLite at startup. Keep this in the binary glue so core/services stay pure.
    prepare_sqlite_file(&parsed.db_url)?;

    match cmd {
        Command::Ui => {
            let clock = Clock::default_clock();
            let services =
                AppServices::new_sqlite(&parsed.db_url, clock, parsed.deck_id).await?;

            let app = DesktopApp {
                deck_id: services.deck_id(),
                session_summaries: services.session_summaries(),
                session_loop: services.session_loop(),
                card_service: services.card_service(),
                deck_service: services.deck_service(),
                app_settings: services.app_settings(),
                writing_tools: services.writing_tools(),
                open_editor_on_launch: services.open_editor_on_launch(),
                link_opener: Arc::new(DesktopLinkOpener),
            };

            let app: Arc<dyn UiApp> = Arc::new(app);
            let context = build_app_context(&app);

            // On macOS, Dioxus/tao can default to an always-on-top window in some dev setups.
            // Explicitly disable it so the app doesn't behave like a modal window.
            let desktop_cfg = DesktopConfig::new().with_window(
                WindowBuilder::new()
                    .with_title("Learn")
                    .with_always_on_top(false)
                    .with_min_inner_size(LogicalSize::new(980.0, 720.0)),
            );

            LaunchBuilder::desktop()
                .with_cfg(desktop_cfg)
                .with_context(context)
                .launch(App);
            Ok(())
        }
        Command::Seed => {
            // Placeholder: we intentionally keep seed separate from the UI launch.
            // Next step will add concrete operations (create deck, add cards) once the
            // storage/services API is finalized.
            eprintln!(
                "seed: not implemented yet (db={}, deck_id={}).",
                parsed.db_url, parsed.deck_id
            );
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
        .or_else(|| db_url.strip_prefix("sqlite:"))
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
            .truncate(false)
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

# Learn Hobby

## Build

```
cargo build
```

## Seed SQLite data

```
cargo run -p storage --bin seed -- --db sqlite:dev.sqlite3 --deck-id 1 --summaries 3
```

Optional flags:
- `--deck-name "Japanese"`
- `--deck-desc "Core deck"`
- `--now 2025-01-01T10:00:00Z`

Environment variables (same as flags):
- `LEARN_DB_URL`
- `LEARN_DECK_ID`
- `LEARN_DECK_NAME`
- `LEARN_DECK_DESC`
- `LEARN_SUMMARIES`

## Run (desktop)

```
cargo run -p app -- --db sqlite:dev.sqlite3 --deck-id 1
```

## Serve (Dioxus dev server)

```
LEARN_DB_URL=sqlite:dev.sqlite3 LEARN_DECK_ID=1 dx serve -p app
```

Notes:
- CLI flags override env vars.
- `dx serve` does not forward app args like `--db`; use env vars.

## Practice options (UI)

- Practice Due Cards: default session (due + new, micro-session size).
- Practice All Cards: full-deck session (ignores micro-session size).
- Re-practice Mistakes: cards currently in `Relearning` phase (typically Again grades).
- Reset Learning Progress: resets scheduling for all cards in a deck.

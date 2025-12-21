# Guideline

This document is the **authoritative architectural contract** for this repository.
Code **must** follow it. The guideline changes only when the architecture intentionally evolves.

---

## Project Overview

This is a **language learning application** built with **Rust** and **Dioxus**, designed to help people with **ADHD and anxiety** who struggle with memorization.

The app uses the **FSRS (Free Spaced Repetition Scheduler)** algorithm via the `fsrs` crate for scientifically optimized review scheduling.

### Product goals
- **ADHD-friendly**: micro-sessions (default 5), low cognitive load, fast “small wins”.
- **Anxiety-aware**: no punishment mechanics, encouraging feedback, pause/resume anytime.
- **Type-safe**: leverage Rust types to prevent invalid states.
- **Best-practice Rust**: panic-free public APIs, explicit errors, future-proofing.

---

## Workspace Structure

This is a Cargo workspace with **strict layering / DDD boundaries**:

- **`crates/core`**: domain logic only (models + invariants + pure scheduler)
- **`crates/storage`**: persistence only (SQLite + migrations + mapping)
- **`crates/services`**: orchestration only (workflows; owns time via `Clock`)
- **`crates/ui`**: Dioxus UI only (render state + dispatch intents)

### Non‑negotiable architecture rules
- **No SQL outside `crates/storage`**.
- **No clocks outside `crates/services`** (never call `Utc::now()` directly).
- **No I/O in `crates/core`**.
- **No business logic in `crates/ui`**.
- **No stringly-typed APIs** where enums/newtypes apply.
- **No silent data corruption**: invalid persisted state must surface as explicit errors.

---

## Rust API Design Sources

This project follows:

1. Official Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
2. Elegant APIs in Rust: https://ruststack.org/elegant-apis-in-rust/
3. Type-Driven API Design Patterns: https://willcrichton.net/rust-api-type-patterns/

### Canonical rules
- Public APIs are panic-free; fallible operations return `Result` and document `# Errors`.
- Mark evolving public enums/structs as `#[non_exhaustive]`.
- Prefer types over strings; prefer compile-time guarantees when they pay for themselves.
- Keep invariants inside types; make invalid states unrepresentable where reasonable.

---

## Determinism and Time

All time-dependent logic goes through a **Clock abstraction** in `crates/services`.

```rust
pub enum Clock {
    System,
    Fixed(DateTime<Utc>),
}
```

Rules:
- `crates/core`: **no clocks**.
- `crates/storage`: **no clocks**.
- `crates/services`: owns/receives a `Clock` and passes timestamps to domain logic.
- Tests use `Clock::Fixed(...)`.

---

## Core Design Patterns

### Newtypes for IDs and domain primitives
Use newtypes to prevent accidental misuse and make signatures self-documenting.

```rust
pub struct CardId(u64);
pub struct DeckId(u64);
pub struct MediaId(u64);
```

### Draft → Validated pattern
Use drafts for potentially invalid inputs; validate into immutable “valid” types.

```rust
ContentDraft::new(...)
    .validate(...)
    -> Result<Content, ContentValidationError>
```

Rules:
- Drafts accept broad input.
- Validation enforces invariants.
- IDs are assigned during persistence (not by UI).

### Typestate pattern
Use typestate to prevent invalid transitions (only where it improves correctness without overcomplexity).

Good candidates:
- Session lifecycle (`NotStarted → InProgress → Completed`)
- Card lifecycle (draft/active/suspended) if it reduces bug surface

---

## Error Handling

- Use `thiserror` in library crates.
- No `anyhow` in library crates (allowed only in app/binary glue).
- Domain errors are precise and composable.
- Storage maps DB/driver errors inside `storage` and returns crate-local error types.
- Service errors compose domain + storage errors (no panics, no silent fallback).

### UX-facing error policy
- UI must translate technical errors into calm, actionable messages.
- Prefer “We couldn’t save that yet — try again” over stack-trace-y wording.
- Provide a single recovery action: Retry / Back / Report.

---

## Testing Strategy

### Where tests live
- **Domain unit tests**: `crates/core` (`mod tests` in-file for small modules; `tests/` for larger contracts).
- **Storage integration tests**: `crates/storage/tests/` (SQLite + migrations + real queries).
- **Service tests**: `crates/services` with in-memory repos + `Clock::Fixed`.
- **UI tests**: view-model/state-machine tests (avoid snapshot-heavy tests).

### Test rules
- Tests must be deterministic (no system time).
- Prefer explicit fixtures and small helpers over large shared state.
- Ensure **in-memory ordering matches SQLite ordering**.

---

## UI/UX Principles (ADHD + Anxiety)

### UX rules
- Default micro-session size is **5**.
- “Pause/Resume” is always available; no penalties.
- Avoid punishment language (prefer “Try again” over “Failed”).
- Show progress simply (“3 of 5”).
- Primary action is always clear and single.

### Accessibility rules
- Keyboard-first on desktop: `1/2/3/4` for grading, `Space` to reveal, `Esc` to pause/close.
- Predictable focus order; visible focus ring.
- Minimum target size 44×44 (prepares for mobile).

### Calm UX defaults
- Reduce surprise: avoid sudden navigation; use gentle transitions.
- Avoid modal spam: at most one modal at a time; prefer inline toasts.
- Respect “quiet hours” for nudges.

---

## UI Architecture (Dioxus)

### Core rule
UI is **dumb**: render state + dispatch intents. **All domain work happens in services**.

### Required UI data flow (MVI-style)
Each interactive screen must follow:

`UI → Intent → ViewModel → Service call → New UI State`

- **Intent**: enum describing user actions.
- **State**: enum describing screen state (avoid boolean soup).

Example (Session screen):
- `Intent::RevealAnswer`
- `Intent::Grade(ReviewGrade)`

State:
- `State::Loading`
- `State::ShowingPrompt { .. }`
- `State::ShowingAnswer { .. }`
- `State::Completed { summary_id }`

### Dependency injection (AppContext)
UI must consume a single context that exposes **services only**. Repositories/storage types remain private.

- ✅ UI sees: `DeckService`, `CardService`, `SessionService`, `ReviewService`, `MediaService`
- ❌ UI does not see: SQLx types, repository traits, DB pools

### Async / concurrency rules
- Use `use_resource` / `use_future` for async work.
- Use explicit state: `Idle → Loading → Ready/Error`.
- **Never block the UI thread** (no sync DB calls, no heavy decoding on main).
- Cancel stale async work when navigating away (guard with request ids / generation counters).

### UI state ownership rules
- Screen state lives in the screen/view-model.
- Reusable components are pure: props in, events out.
- No global mutable singletons in UI; use context + immutable state updates.

### Dioxus-specific recommendations
- Prefer **stable keys** for lists (`key: card_id`) to avoid rerender glitches.
- Keep components small; lift state only when needed.
- Avoid cloning large data into props; pass IDs and fetch via VM when appropriate.
- Use `Signal`/state hooks consistently; avoid mixing patterns within the same screen.

### Styling rules
- Start with a small, consistent design system: spacing scale, typography, button variants.
- Avoid heavy theming early; ship clarity first.
- Ensure high contrast readiness; don’t encode meaning by color alone.

---

## Platform Features

Platform-specific capabilities must be behind traits in `crates/ui/src/platform/`:
- `UiNotifier` (notifications)
- `UiFilePicker` (import/export)
- `UiTray` (menu bar/system tray)

`platform/macos.rs` implements these traits.

Rule: no macOS-only types leak into `core` or `services`.

---

## Suggested `crates/ui` Structure

```text
crates/ui/src/
├── app.rs
├── context.rs
├── routes.rs
├── views/
│   ├── home.rs
│   ├── session.rs
│   ├── editor.rs
│   ├── history.rs
│   ├── summary.rs
│   └── settings.rs
├── vm/
│   ├── home_vm.rs
│   ├── session_vm.rs
│   ├── editor_vm.rs
│   ├── history_vm.rs
│   └── summary_vm.rs
├── components/
│   ├── buttons.rs
│   ├── content_block.rs
│   ├── grade_bar.rs
│   └── summary_card.rs
└── platform/
    ├── mod.rs
    └── macos.rs
```

---

## UI Implementation Roadmap (macOS-first)

Build **vertical slices** (thin end-to-end): UI → VM → service → repo.

### Step 1 — UI scaffold
- Router + layout
- Sidebar: Decks / Practice / History / Settings
- Minimal design system (buttons/cards/badges)

Exit: app runs on macOS; navigation works with stub data.

### Step 2 — Service wiring
- `AppContext` exposes services only
- Smoke call from UI to a service and render result

Exit: UI can call real services without accessing storage types.

### Step 3 — Home (Practice launcher)
- “Practice now”, “Continue”, “Quick add”
- “Practice now” calls `SessionService` to build/start a session

Exit: one click starts a session.

### Step 4 — Session player (review loop)
- Prompt → reveal → grade
- Keyboard shortcuts
- Persist review through `ReviewService` atomic boundary
- Persist session summary and navigate to summary page

Exit: end-to-end persisted practice works.

### Step 5 — Card editor (create/edit)
- Prompt content: text + optional image
- Answer content: text + optional image
- Save + save-and-practice

Exit: add/edit cards and practice immediately.

### Step 6 — History
- List session summaries with stable ordering
- Open details

Exit: user can review past sessions.

### Step 7 — macOS menu bar + nudges
- Menu bar popover (Practice/Continue/Quick add)
- Notifications based on due count + quiet hours

Exit: lightweight macOS entry is available.

---

## Final Checklist

- No SQL outside `crates/storage`
- No clocks outside `crates/services`
- No I/O in `crates/core`
- UI depends on services only
- No stringly-typed APIs
- Invalid persisted state errors are explicit
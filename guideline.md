# Guideline

This file provides guidance when working with code in this repository.

## Project Overview

This is a **language learning application** built with Rust and the Dioxus framework, specifically designed to help people with **ADHD and anxiety** who struggle with memorization. The application uses the **FSRS (Free Spaced Repetition Scheduler)** algorithm via the `fsrs` crate for scientifically-optimized card scheduling.

### Key Goals
- **ADHD-friendly design**: Micro-sessions (5 cards default), minimal cognitive load, bite-sized learning chunks
- **Anxiety management**: Progressive difficulty, encouraging feedback, no pressure mechanics
- **Type-safe architecture**: Leverage Rust's type system to prevent invalid states at compile time
- **Best-practice Rust**: Follow official Rust API guidelines, elegant API design patterns, and modern type-driven development

## Workspace Structure

The project is organized as a Cargo workspace with four crates following domain-driven design:

- **`crates/core`**: Domain logic, models, and business rules (platform-agnostic)
- **`crates/storage`**: Persistence layer (database, file I/O)
- **`crates/services`**: Application services (orchestration, business workflows)
- **`crates/ui`**: Dioxus-based user interface (web/desktop/mobile)

---

## Rust API Design Principles

This project follows three authoritative sources for Rust API design:

### 1. Official Rust API Guidelines
- **Source**: https://rust-lang.github.io/api-guidelines/
- **Focus**: Naming conventions, interoperability, documentation, type safety, future-proofing

### 2. Elegant APIs in Rust
- **Source**: https://ruststack.org/elegant-apis-in-rust/
- **Focus**: Builder patterns, session types, extension traits, lazy evaluation

### 3. Type-Driven Design Patterns
- **Source**: https://willcrichton.net/rust-api-type-patterns/
- **Focus**: State machines, witnesses, guards, typestate pattern

### Canonical API Expectations (from the above sources)
- Prefer panic-free public APIs; return `Result` and document `# Errors` / `# Panics` explicitly.
- Future-proof with `#[non_exhaustive]` on public enums/structs intended to evolve.
- Follow Rust naming patterns (`iter`/`iter_mut`/`into_iter`, `as_` vs `to_` vs `into_`; avoid `get_` unless needed).
- Accept flexible inputs via standard traits (`IntoIterator`, `AsRef`, `Into`) and let callers choose ownership/cloning.
- Keep builders and validation separate; validated types must uphold invariants.

---

## Core Design Patterns

### Pattern 1: Typestate Pattern (Session Types)
Encode state transitions as different types to prevent invalid operations at compile time.

**Current Implementation**: `Text<T>` with `Front` and `Back` phantom type markers
```rust
pub struct Text<T>(String, std::marker::PhantomData<T>);
pub struct Front;
pub struct Back;
pub type FrontText = Text<Front>;
pub type BackText = Text<Back>;
```

**Benefits**:
- Compile-time prevention of mixing front/back text
- Zero runtime overhead
- Self-documenting API

**Future Applications**:
- Card lifecycle states: `Draft`, `Active`, `Suspended`, `Buried`
- Review states: `New`, `Learning`, `Reviewing`, `Relearning`
- Session states: `NotStarted`, `InProgress`, `Completed`

### Pattern 2: Builder Pattern with Validation
Separate draft entities (builders) from validated entities.

**Current Implementation**:
```rust
ContentDraft::new(text, media)
    .validate(timestamp, metadata, checksum)
    .map(|content| content.with_id(id))
```

**Principles**:
- Draft types accept any input (ergonomic construction) while keeping fields private to enforce invariants
- Validation returns `Result<ValidatedType, Error>`
- Validated types are immutable and guarantee invariants
- IDs assigned only after validation (prevents invalid references)

### Pattern 3: Newtype Pattern for Domain Primitives
Wrap primitives in domain-specific types to prevent misuse.

**Current Implementation**:
```rust
pub struct CardId(u64);
pub struct DeckId(u64);
pub struct MediaId(u64);
```

**Benefits**:
- Cannot accidentally pass `CardId` where `DeckId` is expected
- Can implement domain-specific methods
- Self-documenting function signatures

### Pattern 4: Enum Over Strings ("Stringly-typed" → Type-safe)
Replace string parameters with enums when variants are known.

**Current Implementation**:
```rust
pub enum MediaKind { Image }  // Will extend: Audio, Video
pub enum ReviewGrade { Again, Hard, Good, Easy }
```

**Anti-pattern to avoid**:
```rust
// BAD: Stringly-typed
fn set_difficulty(level: &str) { /* "easy", "hard", "impossible" */ }

// GOOD: Type-safe
fn set_difficulty(level: Difficulty) { /* enum Difficulty */ }
```

### Pattern 5: Extension Traits
Add domain-specific methods to external types without breaking encapsulation.

**Convention**: Suffix with `Ext`
```rust
pub trait DurationExt {
    fn to_days(&self) -> f64;
}
impl DurationExt for chrono::Duration {
    fn to_days(&self) -> f64 { self.num_seconds() as f64 / 86400.0 }
}
```

---

## Naming Conventions (RFC 430 + API Guidelines)

### Type-level Constructs (UpperCamelCase)
- Types, traits, enum variants: `Card`, `ReviewGrade`, `Easy`
- Acronyms count as one word: `Uuid` (not `UUID`), `Url` (not `URL`)

### Value-level Constructs (snake_case)
- Functions, methods, variables: `new_cards_per_day`, `validate()`
- Acronyms lowercase: `is_xid_start`, `parse_url()`

### Method Naming Patterns
- **Constructors**: `new()`, `default()`, `with_capacity()`
- **Conversions**:
  - `from_*()`: Constructing conversion (e.g., `from_file()`)
  - `as_*()`: Cheap reference-to-reference (e.g., `as_str()`)
  - `to_*()`: Expensive conversion (e.g., `to_owned()`)
  - `into_*()`: Consuming conversion (e.g., `into_inner()`)
- **Setters**: `set_*()`
- **Builders**: `with_*()` (chainable, returns Self)
- **Predicates**: `is_*()`, `has_*()` (returns bool)

### Getter Conventions
**Do NOT use `get_` prefix** unless there's computation or ambiguity.
```rust
// GOOD
card.id()
card.prompt()

// BAD (unless getters are complex)
card.get_id()
card.get_prompt()
```

### Crate Naming
**Never use `-rs` or `-rust` suffix/prefix**. Every crate is Rust!
```
✓ learn-app
✗ learn-app-rs
✗ rust-learn-app
```

---

## Error Handling Best Practices

### Use `thiserror` for Library Errors
```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CardError {
    #[error("invalid prompt content: {0}")]
    InvalidPrompt(#[source] ContentValidationError),

    #[error("invalid answer content: {0}")]
    InvalidAnswer(#[source] ContentValidationError),
}
```

### Error Design Principles
1. **Specific errors**: Each error variant should represent one failure mode
2. **Transparent wrapping**: Use `#[error(transparent)]` for nested errors
3. **Context-rich messages**: Include what failed and why
4. **Derive Clone, PartialEq**: Enable testing and error comparison
5. **No `anyhow` in libraries**: Use `anyhow` only in application code

---

## Type Safety Guidelines

### Make Invalid States Unrepresentable
```rust
// BAD: Can create invalid ImageMeta
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
}

// GOOD: Constructor enforces invariants
pub struct ImageMeta {
    width: u32,
    height: u32,
}

impl ImageMeta {
    pub fn new(width: u32, height: u32) -> Result<Self, Error> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidDimensions);
        }
        Ok(Self { width, height })
    }
}
```

### Use Marker Types for Compile-Time Guarantees
```rust
// State machine with phantom types
pub struct ReviewSession<State> {
    cards: Vec<Card>,
    _state: PhantomData<State>,
}

pub struct NotStarted;
pub struct InProgress;
pub struct Completed;

impl ReviewSession<NotStarted> {
    pub fn start(self) -> ReviewSession<InProgress> { /* ... */ }
}

impl ReviewSession<InProgress> {
    pub fn answer(&mut self, grade: ReviewGrade) { /* ... */ }
    pub fn finish(self) -> ReviewSession<Completed> { /* ... */ }
}

impl ReviewSession<Completed> {
    pub fn results(&self) -> SessionResults { /* ... */ }
}
```

### Leverage Conversion Traits
Accept flexible inputs using standard traits:
```rust
// GOOD: Accepts &str, String, Cow<str>, etc.
pub fn from_file(path: impl Into<String>) -> Result<MediaUri, Error> {
    let path = path.into();
    // ...
}

// Also consider AsRef for paths and strings
pub fn load_deck(path: impl AsRef<Path>) -> Result<Deck, Error> {
    let path = path.as_ref();
    // ...
}
```

---

## Documentation Best Practices

### Write Executable Examples
```rust
/// Creates a new deck with ADHD-friendly defaults.
///
/// # Examples
///
/// ```
/// use core::model::DeckSettings;
///
/// let settings = DeckSettings::default_for_adhd();
/// assert_eq!(settings.micro_session_size, 5);
/// assert_eq!(settings.new_cards_per_day, 5);
/// ```
pub fn default_for_adhd() -> Self { /* ... */ }
```

### Document Panics, Errors, and Safety
```rust
/// # Errors
///
/// Returns `CardError::InvalidPrompt` if prompt text is empty.
/// Returns `CardError::InvalidAnswer` if answer text is empty.
pub fn new(...) -> Result<Self, CardError> { /* ... */ }
```

---

## ADHD-Focused Design Principles

### Cognitive Load Reduction
- **Micro-sessions**: Default 5 cards per session (configurable)
- **Single focus**: One card at a time, no distractions
- **Quick wins**: Immediate feedback, progress indicators
- **Flexible pacing**: Pause/resume anytime, no penalties

### Anxiety Management
- **No punishment**: Failed cards don't reset to zero
- **Gradual difficulty**: FSRS algorithm naturally adapts
- **Encouraging language**: Positive reinforcement in UI
- **Low commitment**: "Just 5 cards" feels manageable

---

## Core Architecture

### Domain Model (`crates/core/src/model/`)

The core domain follows a validation-focused design pattern with separation between draft and validated entities:

#### Content System (Multimodal)
- **`Content`**: Validated content with text (required) and optional media reference (MediaId)
- **`ContentDraft`**: Unvalidated content builder with text and optional MediaDraft
- **Pattern**: Drafts are validated via `validate()` method that requires metadata (timestamps, image dimensions, checksums)

#### Media System
- **`MediaUri`**: Enum wrapping either `FilePath(PathBuf)` or `Url(Url)`
- **`MediaDraft`**: Unvalidated media with URI, kind, and optional alt text
- **`MediaItem`**: Validated media with ID, metadata (ImageMeta), checksum, and timestamps
- **`ImageMeta`**: Width/height validation (both must be > 0)
- **Pattern**: Media drafts are validated and receive IDs during persistence

#### Text System
- **`Text<T>`**: Generic wrapper around String with phantom type marker (Front/Back)
- **`FrontText`** and **`BackText`**: Type aliases using Front/Back markers
- **Validation**: Rejects empty or whitespace-only strings

#### Card System
- **`Card`**: Flashcard with prompt (Content) and answer (Content)
- **Validation**: Both prompt and answer text cannot be empty
- Each card belongs to a `DeckId` and has timestamps

#### Deck System
- **`Deck`**: Collection of cards with name, description, and settings
- **`DeckSettings`**: Configurable parameters:
  - `new_cards_per_day`: Daily limit for new cards
  - `review_limit_per_day`: Daily limit for reviews
  - `micro_session_size`: Cards per micro-session (ADHD-friendly feature)
- **`default_for_adhd()`**: Pre-configured with 5 new cards/day, 30 reviews/day, 5 cards per session

#### Review System
- **`ReviewGrade`**: Four-level grading (Again, Hard, Good, Easy)
- **`ReviewLog`**: Records card review events with grade and timestamp
- **`ReviewOutcome`**: FSRS algorithm output with next review time and metrics (stability, difficulty, elapsed/scheduled days)

### ID Types (`crates/core/src/model/ids.rs`)
- Newtype wrappers: `CardId(u64)`, `DeckId(u64)`, `MediaId(u64)`
- All derive Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize

### Scheduler (`crates/core/src/scheduler.rs`)
- Integration point for FSRS algorithm (using `fsrs` crate v5.x)
- Encapsulates card scheduling logic and FSRS parameter optimization
- Constructors (`Scheduler::new`, `with_retention`) are fallible; callers must handle `Result`
- Public errors are `#[non_exhaustive]` to allow future extension without breaking dependents

### Error Handling (`crates/core/src/error.rs`)
- All errors use `thiserror::Error` for standardized error types
- Domain-specific errors: `CardError`, `DeckError`, `ReviewError`, `ContentValidationError`, `MediaValidationError`, `TextError`
- Public error enums are marked `#[non_exhaustive]` to future-proof API evolution

---

## Dependencies

### Core Dependencies
- `serde` (1.x): Serialization with derive macros
- `chrono` (0.4): DateTime handling (always use `Utc`)
- `thiserror` (1.x): Error type derivation
- `uuid` (1.x): UUID generation with v4 and serde features
- `url` (2.5): URL parsing and validation

### FSRS Integration
- `fsrs` (5.x): Full FSRS implementation with optimizer and scheduler
  - Use for parameter optimization and card scheduling
  - Provides `ReviewOutcome` with stability/difficulty metrics
  - Latest: v5.2.0 (2024 edition)

### UI Framework (Future)
- `dioxus` (0.6.x, target 0.7 in March 2025): Cross-platform UI framework
- `dioxus-router`: Navigation and routing
- `tailwind-css`: Styling (built-in Dioxus support)

### Storage (Future)
- `sqlx` or `diesel`: SQL database integration
- `serde_json`: JSON serialization for media metadata

---

## Code Style

### General Conventions
- **Edition**: 2024
- **License**: Apache-2.0
- **Use section comments** with box drawing characters: `// ─── SECTION ───`
- **Derive order**: Debug, Error (for errors), Clone, Serialize, Deserialize, PartialEq, Eq
- **Use `impl Into<String>`** for string parameters that accept both String and &str
- **Use `impl AsRef<Path>`** for path parameters

### Formatting
- Run `cargo fmt` before every commit
- Use rustfmt defaults (no custom configuration)
- Max line length: 100 characters (rustfmt default)

### Imports
- Group imports: std → external crates → local crates → super/self
- Use explicit imports, avoid glob imports (`use foo::*;`)

### Comments
- Use `//` for inline comments
- Use `///` for doc comments on public items
- Use `//!` for module-level documentation

---

## Implementation Roadmap

### Roadmap Principles
- ADHD/anxiety-friendly first: optimize for short sessions, low cognitive load, and reassuring feedback.
- Rust API quality: follow Rust API Guidelines (naming, docs, non-exhaustive errors, panic-free APIs).
- Layered separation: core/domain stays UI/storage-agnostic; services orchestrate; UI consumes services.
- Test-as-contract: each milestone includes unit + integration coverage where applicable.

### Status Snapshot
- ✅ Core domain models, validation patterns, FSRS scheduler (fallible), non-exhaustive errors, unit tests.
- ⏳ Card lifecycle typestate not started; storage design not started; UI scaffold not started.

### Phase 1 (Core Domain) — done except lifecycle
- [done] Domain models (Card, Deck, Content, Media, Review) with validation.
- [done] FSRS scheduling via `fsrs` crate; fallible constructors; non-exhaustive errors.
- [todo] Card lifecycle typestate (`New`, `Learning`, `Reviewing`, `Relearning`) with compile-time transitions.
  - Exit: state type prevents invalid ops; tests cover transitions and persistence of state enum.

### Phase 2 (Storage) — design + integration tests
- Choose SQLite + `sqlx` to start; define schema for decks/cards/media/review logs.
- Repository traits for domain entities; implement SQLite adapters.
- Migrations + smoke tests; integration tests cover CRUD and constraint errors.
- Exit: repositories used in services without leaking SQL types; migrations automated in CI.

#### Storage Recommendation (offline + cloud)
- Default to SQLite for local/offline and cross-platform support (desktop, mobile, WASM).
- Target hosted SQLite-compatible services for cloud sharing without dialect drift:
  - **libSQL/Turso** (edge-friendly, SQLite API compatible)
  - **Cloudflare D1** (SQLite-based, serverless)
- Keep one SQL dialect; avoid vendor-specific features so the same schema works locally and in the cloud.

### Phase 3 (Services) — orchestration on top of repos
- Deck management service (create/update decks + settings).
- Review session service persisted (load/save sessions, append logs).
- Media handling service (store URIs + metadata; checksum optional).
- Exit: services tested with mock repos; end-to-end happy paths with SQLite adapters.

### Phase 4 (UI) — Dioxus scaffold, ADHD-friendly UX
- Create Dioxus app shell; wiring to services via async calls.
- Components: Deck list/detail, Micro-session player, Review grading UI.
- Styling: Tailwind; ensure mobile-first sizing and focusable controls.
- Exit: can run a micro-session against local services with fake data; keyboard navigation + basic a11y checks.

### Phase 5 (Integration & Polish)
- Wire UI to real storage/services; add offline-first plan (cache + sync strategy).
- Observability (logging/metrics) for review outcomes.
- Performance passes; user testing with ADHD/anxiety focus group feedback loop.

---

## Prescriptive Architecture Checklist

- Persistence: default to SQLite with `sqlx`; keep schema portable for libSQL/Turso/D1.
- Media: store URIs + metadata; choose filesystem/cloud later, but keep the domain API storage-agnostic.
- Sync: design for offline-first; add sync later (WebSocket/REST/CRDT) without breaking local mode.
- Platforms: target web/desktop/mobile with the same domain/services; avoid platform-specific types in core.
- Accessibility: ensure keyboard navigation, focus order, and high-contrast readiness in UI components.

use chrono::{DateTime, Duration, Utc};

/// A simple clock abstraction for deterministic time in services and tests.
#[derive(Debug, Clone, Copy, Default)]
pub enum Clock {
    #[default]
    Default,
    Fixed(DateTime<Utc>),
}

impl Clock {
    /// Returns a clock that uses the current system time.
    #[must_use]
    pub fn default_clock() -> Self {
        Self::Default
    }

    /// Returns a clock fixed at the given timestamp.
    #[must_use]
    pub fn fixed(at: DateTime<Utc>) -> Self {
        Self::Fixed(at)
    }

    /// Returns the current time according to the clock.
    #[must_use]
    pub fn now(&self) -> DateTime<Utc> {
        match self {
            Clock::Default => Utc::now(),
            Clock::Fixed(t) => *t,
        }
    }

    /// If this is a fixed clock, advance it by the given duration.
    ///
    /// Has no effect on `Clock::Default`.
    pub fn advance(&mut self, delta: Duration) {
        if let Clock::Fixed(t) = self {
            *t += delta;
        }
    }

    /// Returns true if this clock represents real time.
    #[must_use]
    pub fn is_default(&self) -> bool {
        matches!(self, Clock::Default)
    }

    /// Returns true if this clock is fixed.
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        matches!(self, Clock::Fixed(_))
    }
}

/// Deterministic timestamp for tests and examples (2023-11-14T22:13:20Z).
pub const FIXED_TEST_TIMESTAMP: i64 = 1_700_000_000;

/// Returns a deterministic `DateTime<Utc>` for tests and doc examples.
///
/// # Panics
///
/// Panics if the fixed timestamp cannot be represented.
#[must_use]
pub fn fixed_now() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(FIXED_TEST_TIMESTAMP, 0)
        .expect("fixed timestamp should be valid")
}

/// Returns a `Clock` fixed at the deterministic test timestamp.
#[must_use]
pub fn fixed_clock() -> Clock {
    Clock::fixed(fixed_now())
}

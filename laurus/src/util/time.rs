//! Cross-platform time utilities.
//!
//! Provides [`now_secs`] and [`now_millis`] which return the current time
//! since the Unix epoch, working on both native and `wasm32-unknown-unknown`
//! targets.
//!
//! On native targets, this uses [`std::time::SystemTime`]. On WASM, it
//! falls back to a constant value of `0` since `SystemTime::now()` is
//! not available in `wasm32-unknown-unknown`.

/// Return the current time as seconds since the Unix epoch.
///
/// On `wasm32-unknown-unknown`, returns `0` because `SystemTime::now()`
/// is not available. This is acceptable because timestamps in WASM are
/// used only for metadata (not for correctness).
///
/// # Returns
///
/// Seconds since 1970-01-01T00:00:00Z.
pub fn now_secs() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

/// Return the current time as milliseconds since the Unix epoch.
///
/// On `wasm32-unknown-unknown`, returns `0` because `SystemTime::now()`
/// is not available. This is acceptable because timestamps in WASM are
/// used only for metadata (not for correctness).
///
/// # Returns
///
/// Milliseconds since 1970-01-01T00:00:00Z.
pub fn now_millis() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

/// A cross-platform monotonic timer for measuring elapsed durations.
///
/// On native targets, wraps [`std::time::Instant`]. On WASM, elapsed
/// time is always reported as zero since `Instant::now()` is not
/// available in `wasm32-unknown-unknown`.
///
/// # Example
///
/// ```
/// use laurus::util::time::Timer;
///
/// let timer = Timer::now();
/// // ... do work ...
/// let elapsed = timer.elapsed_ms();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Timer {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}

impl Timer {
    /// Start a new timer.
    pub fn now() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
        }
    }

    /// Return elapsed time in milliseconds since this timer was created.
    ///
    /// Returns `0` on WASM targets.
    pub fn elapsed_ms(&self) -> u64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed().as_millis() as u64
        }
        #[cfg(target_arch = "wasm32")]
        {
            0
        }
    }

    /// Return elapsed time as a [`std::time::Duration`].
    ///
    /// Returns `Duration::ZERO` on WASM targets.
    pub fn elapsed(&self) -> std::time::Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            std::time::Duration::ZERO
        }
    }
}

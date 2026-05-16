//! Helpers for deterministic virtual time under Tokio paused runtimes (`SC-010`).
//!
//! Integration tests annotate `#[tokio::test(start_paused = true)]` then drive timeouts,
//! backoff, and shutdown grace timers with [`advance_test_clock`] instead of relying on wall
//! clock `sleep`.

use std::future::Future;
use std::time::Duration;

/// Quantum used by [`with_auto_clock_drive`] to march the mocked clock forward.
pub const AUTO_CLOCK_TICK: Duration = Duration::from_millis(1);

/// Advances the paused Tokio timer wheel by `duration`.
///
/// # Arguments
///
/// - `duration`: Virtual elapsed time applied to timers and `tokio::time::Instant::now()` in this
///   runtime.
///
/// # Returns
///
/// This asynchronous function resolves after the mocked clock jumps forward.
///
/// # Preconditions
///
/// The enclosing `#[tokio::test(start_paused = true)]` (or equivalent main) must configure a
/// paused runtime; otherwise callers should not use this helper.
///
/// # Examples
///
/// ```ignore
/// #[tokio::test(start_paused = true)]
/// async fn example() {
///     rust_supervisor::test_support::test_time::advance_test_clock(Duration::from_millis(30))
///         .await;
/// }
/// ```
pub async fn advance_test_clock(duration: Duration) {
    tokio::time::advance(duration).await;
}

/// Runs [`advance_test_clock`] in a tight loop forever until the returned join handle is aborted.
///
/// Intended for spawning alongside [`tokio::time::timeout`] wrappers in paused tests when the wrapped
/// work needs continuous virtual clock progress.
///
/// # Returns
///
/// Returns [`tokio::task::JoinHandle`] for the spawned driver task.
pub fn spawn_auto_clock_drive() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            advance_test_clock(AUTO_CLOCK_TICK).await;
        }
    })
}

/// Runs `body` while concurrently stepping the mocked clock so timer-dependent work completes.
///
/// # Type parameters
///
/// - `F`: Future produced by async work under test.
///
/// # Arguments
///
/// - `body`: Async block that awaits supervisor behavior relying on Tokio timers.
///
/// # Returns
///
/// Returns the output resolved by `body`.
pub async fn with_auto_clock_drive<F, T>(body: F) -> T
where
    F: Future<Output = T>,
{
    let driver = spawn_auto_clock_drive();
    let output = body.await;
    driver.abort();
    let _join = driver.await;
    output
}

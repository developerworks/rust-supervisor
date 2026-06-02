//! Controlled child spawner fixture.
//!
//! Provides `FixtureChildSpawner` for injecting controlled failure
//! patterns (panic, block, cancel-ignore) into chaos scenarios.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

/// Configuration for a controlled child task.
#[derive(Debug, Clone)]
pub enum ChildBehavior {
    /// Task panics after the given delay.
    PanicAfter(Duration),
    /// Task blocks forever without responding to cancellation.
    BlockForever,
    /// Task ignores the cancellation token and runs forever.
    IgnoreCancel,
    /// Task runs normally and succeeds.
    Normal,
}

/// A fixture that spawns a controlled child task.
#[derive(Debug)]
pub struct FixtureChildSpawner {
    /// Behavior of the spawned child.
    pub behavior: ChildBehavior,
}

impl Default for FixtureChildSpawner {
    fn default() -> Self {
        Self {
            behavior: ChildBehavior::Normal,
        }
    }
}

impl FixtureChildSpawner {
    /// Creates a new spawner with the given behavior.
    pub fn new(behavior: ChildBehavior) -> Self {
        Self { behavior }
    }

    /// Sets panic delay.
    pub fn with_panic_delay(delay: Duration) -> Self {
        Self::new(ChildBehavior::PanicAfter(delay))
    }

    /// Sets block-forever behavior.
    pub fn with_block_forever() -> Self {
        Self::new(ChildBehavior::BlockForever)
    }

    /// Sets ignore-cancel behavior.
    pub fn with_ignore_cancel() -> Self {
        Self::new(ChildBehavior::IgnoreCancel)
    }

    /// Spawns a controlled child task and returns its cancellation token.
    ///
    /// The spawned task runs according to the configured `behavior`.
    pub fn spawn(&self) -> CancellationToken {
        let cancel = CancellationToken::new();
        let child_cancel = cancel.clone();
        let behavior = self.behavior.clone();

        tokio::spawn(async move {
            match behavior {
                ChildBehavior::PanicAfter(delay) => {
                    tokio::time::sleep(delay).await;
                    panic!("injected panic after {delay:?}");
                }
                ChildBehavior::BlockForever => {
                    // Busy-loop that never checks cancellation.
                    loop {
                        std::hint::spin_loop();
                    }
                }
                ChildBehavior::IgnoreCancel => loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    // Intentionally never checks child_cancel.
                    let _ = &child_cancel;
                },
                ChildBehavior::Normal => {
                    // Run until cancelled.
                    child_cancel.cancelled().await;
                }
            }
        });

        cancel
    }
}

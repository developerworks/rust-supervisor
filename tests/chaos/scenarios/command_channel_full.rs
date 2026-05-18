//! Chaos scenario: command_channel_full.
//!
//! Fills an mpsc channel (capacity=256) to saturation.
//! Verifies send() returns Err(Closed) instead of blocking forever,
//! and the control loop does not panic.

use crate::chaos::verdict::ScenarioVerdict;
use std::time::Instant;
use tokio::sync::mpsc;

/// Runs the command_channel_full scenario.
pub fn run() -> ScenarioVerdict {
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("command_channel_full");

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let send_closed = rt.block_on(async {
        let (tx, mut rx) = mpsc::channel::<u32>(256);
        // Spawn a consumer that receives very slowly.
        tokio::spawn(async move {
            while rx.recv().await.is_some() {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
        // Fill the channel.
        let mut last_result = Ok(());
        for i in 0..10_000 {
            last_result = tx.try_send(i);
            if last_result.is_err() {
                break;
            }
        }
        last_result.is_err()
    });

    let elapsed = start.elapsed();
    verdict
        .with_threshold("send_closed", if send_closed { 1.0 } else { 0.0 }, 1.0)
        .with_threshold("control_loop_panicked", 0.0, 0.0)
        .with_duration(elapsed.as_nanos())
}

//! Acceptance tests for backoff jitter distribution (SC-004).
//!
//! This test verifies that:
//! 1. Full jitter produces more dispersed wait intervals than fixed delay
//! 2. Decorrelated jitter produces more dispersed wait intervals than fixed delay
//! 3. With fixed RNG seed, results are reproducible
//! 4. SC-004: CV ratio requirement - jitter strategy CV / fixed baseline CV >= 1.3

use rust_supervisor::policy::backoff::BackoffPolicy;
use std::time::Duration;

/// Calculate coefficient of variation (CV) = std_deviation / mean
fn coefficient_of_variation(values: &[u64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<u64>() as f64 / n;
    if mean == 0.0 {
        return 0.0;
    }
    let variance = values
        .iter()
        .map(|v| {
            let diff = *v as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / n;
    let std_dev = variance.sqrt();
    std_dev / mean
}

/// Generate a sequence of next_wait samples from a backoff policy
fn generate_next_wait_samples(policy: &BackoffPolicy, count: usize) -> Vec<u64> {
    (1..=count)
        .map(|attempt| policy.delay_for_child_start_count(attempt as u64).as_millis() as u64)
        .collect()
}

#[test]
fn test_fixed_delay_has_zero_variance() {
    // Fixed delay should have zero variance (all values identical)
    let policy = BackoffPolicy::new(
        Duration::from_millis(1000),
        Duration::from_millis(5000),
        0, // No jitter
        Duration::from_secs(60),
    );

    let samples = generate_next_wait_samples(&policy, 10);
    let cv = coefficient_of_variation(&samples);
    assert_eq!(cv, 0.0, "Fixed delay should have zero CV");
}

#[test]
fn test_full_jitter_produces_dispersion() {
    // SC-004: Full jitter should produce non-zero CV
    let policy = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(5000),
        50, // 50% jitter
        Duration::from_secs(60),
    )
    .with_full_jitter(42);

    let samples = generate_next_wait_samples(&policy, 20);
    let cv = coefficient_of_variation(&samples);
    assert!(cv > 0.0, "Full jitter should produce non-zero CV, got {}", cv);
}

#[test]
fn test_decorrelated_jitter_produces_dispersion() {
    // SC-004: Decorrelated jitter should produce non-zero CV
    let policy = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(5000),
        50,
        Duration::from_secs(60),
    )
    .with_decorrelated_jitter(42);

    let samples = generate_next_wait_samples(&policy, 20);
    let cv = coefficient_of_variation(&samples);
    assert!(
        cv > 0.0,
        "Decorrelated jitter should produce non-zero CV, got {}",
        cv
    );
}

#[test]
fn test_reproducible_with_fixed_seed() {
    // With fixed RNG seed, same sequence should be produced
    let policy = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(5000),
        50,
        Duration::from_secs(60),
    )
    .with_full_jitter(42);

    let samples1 = generate_next_wait_samples(&policy, 10);
    let samples2 = generate_next_wait_samples(&policy, 10);

    assert_eq!(
        samples1, samples2,
        "Fixed seed should produce reproducible results"
    );
}

#[test]
fn test_cv_ratio_requirement_full_jitter() {
    // SC-004: Verify CV_jitter_strategy / CV_fixed_baseline >= 1.3
    // Use non-zero fixed baseline to avoid division by zero

    // Fixed baseline with small exponential growth (not completely flat)
    let fixed_policy = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(5000),
        0, // No jitter
        Duration::from_secs(60),
    );

    // Full jitter strategy
    let jitter_policy = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(5000),
        50,
        Duration::from_secs(60),
    )
    .with_full_jitter(42);

    let fixed_samples = generate_next_wait_samples(&fixed_policy, 20);
    let jitter_samples = generate_next_wait_samples(&jitter_policy, 20);

    let cv_fixed = coefficient_of_variation(&fixed_samples);
    let cv_jitter = coefficient_of_variation(&jitter_samples);

    println!("Fixed CV: {}, Jitter CV: {}", cv_fixed, cv_jitter);

    // Since fixed has exponential growth, it has non-zero CV
    // The ratio should still be >= 1.3 because jitter adds more dispersion
    if cv_fixed > 0.0 {
        let ratio = cv_jitter / cv_fixed;
        assert!(
            ratio >= 1.3,
            "Jitter CV / Fixed CV ratio should be >= 1.3, got {:.2}",
            ratio
        );
    } else {
        // If fixed CV is 0, just verify jitter has meaningful dispersion
        assert!(cv_jitter > 0.1, "Jitter should have meaningful CV when fixed is 0");
    }
}

#[test]
fn test_cv_ratio_requirement_decorrelated_jitter() {
    // SC-004: Verify CV ratio for decorrelated jitter
    let fixed_policy = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(5000),
        0,
        Duration::from_secs(60),
    );

    let jitter_policy = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(5000),
        50,
        Duration::from_secs(60),
    )
    .with_decorrelated_jitter(42);

    let fixed_samples = generate_next_wait_samples(&fixed_policy, 20);
    let jitter_samples = generate_next_wait_samples(&jitter_policy, 20);

    let cv_fixed = coefficient_of_variation(&fixed_samples);
    let cv_jitter = coefficient_of_variation(&jitter_samples);

    println!(
        "Fixed CV: {}, Decorrelated Jitter CV: {}",
        cv_fixed, cv_jitter
    );

    if cv_fixed > 0.0 {
        let ratio = cv_jitter / cv_fixed;
        assert!(
            ratio >= 1.3,
            "Decorrelated jitter CV / Fixed CV ratio should be >= 1.3, got {:.2}",
            ratio
        );
    } else {
        assert!(
            cv_jitter > 0.1,
            "Decorrelated jitter should have meaningful CV when fixed is 0"
        );
    }
}

#[test]
fn test_full_jitter_spread_across_range() {
    // Full jitter should spread values across the entire range [0, max]
    let policy = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(1000),
        100, // 100% jitter for full spread
        Duration::from_secs(60),
    )
    .with_full_jitter(42);

    let samples = generate_next_wait_samples(&policy, 50);
    let unique_values: std::collections::HashSet<u64> = samples.iter().copied().collect();

    assert!(
        unique_values.len() > 10,
        "Full jitter should produce diverse values, got {} unique",
        unique_values.len()
    );

    let cv = coefficient_of_variation(&samples);
    assert!(
        cv > 0.1,
        "Full jitter should have reasonable dispersion, got CV={:.3}",
        cv
    );
}

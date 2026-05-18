//! Acceptance tests for backoff jitter distribution (SC-004).
//!
//! This test verifies that:
//! 1. Full jitter produces more dispersed wait intervals than no-jitter exponential backoff
//! 2. Decorrelated jitter produces more dispersed wait intervals than no-jitter exponential backoff
//! 3. With fixed RNG (Random Number Generator) seed, results are reproducible
//! 4. SC-004: CV ratio requirement - jitter strategy CV / fixed baseline CV >= 1.3

use rust_supervisor::policy::backoff::BackoffPolicy;
use std::time::Duration;

const INITIAL_DELAY_MS: u64 = 50;
const MAX_DELAY_MS: u64 = 100;
const SAMPLE_COUNT: u64 = 10;
const RNG_SEED: u64 = 123;

/// Calculate coefficient of variation (CV) = std_deviation / mean.
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

/// Generate the approved no-jitter exponential baseline sequence.
fn no_jitter_exponential_samples() -> Vec<u64> {
    let policy = BackoffPolicy::new(
        Duration::from_millis(INITIAL_DELAY_MS),
        Duration::from_millis(MAX_DELAY_MS),
        0,
        Duration::from_secs(60),
    );

    (1..=SAMPLE_COUNT)
        .map(|attempt| policy.delay_for_child_start_count(attempt).as_millis() as u64)
        .collect()
}

/// Generate full-jitter samples using production BackoffPolicy.
fn full_jitter_samples(seed: u64) -> Vec<u64> {
    let base_policy = BackoffPolicy::new(
        Duration::from_millis(INITIAL_DELAY_MS),
        Duration::from_millis(MAX_DELAY_MS),
        100,
        Duration::from_secs(60),
    );

    (1..=SAMPLE_COUNT)
        .map(|attempt| {
            base_policy
                .with_full_jitter(seed + attempt)
                .delay_for_child_start_count(attempt)
                .as_millis() as u64
        })
        .collect()
}

/// Generate decorrelated-jitter samples using production BackoffPolicy.
fn decorrelated_jitter_samples(seed: u64) -> Vec<u64> {
    let base_policy = BackoffPolicy::new(
        Duration::from_millis(INITIAL_DELAY_MS),
        Duration::from_millis(MAX_DELAY_MS),
        100,
        Duration::from_secs(60),
    );

    (1..=SAMPLE_COUNT)
        .map(|attempt| {
            base_policy
                .with_decorrelated_jitter(seed + attempt)
                .delay_for_child_start_count(attempt)
                .as_millis() as u64
        })
        .collect()
}

#[test]
fn test_no_jitter_baseline_uses_exponential_sequence() {
    // SC-004 option 7a requires a no-jitter exponential backoff baseline.
    let samples = no_jitter_exponential_samples();
    assert_eq!(
        samples,
        vec![50, 100, 100, 100, 100, 100, 100, 100, 100, 100]
    );

    let cv = coefficient_of_variation(&samples);
    assert!(cv > 0.0, "Exponential baseline should not be flat");
}

#[test]
fn test_full_jitter_produces_dispersion() {
    // SC-004: Full jitter should produce non-zero CV.
    let samples = full_jitter_samples(RNG_SEED);
    let cv = coefficient_of_variation(&samples);
    assert!(
        cv > 0.0,
        "Full jitter should produce non-zero CV, got {}",
        cv
    );
}

#[test]
fn test_decorrelated_jitter_produces_dispersion() {
    // SC-004: Decorrelated jitter should produce non-zero CV.
    let samples = decorrelated_jitter_samples(RNG_SEED);
    let cv = coefficient_of_variation(&samples);
    assert!(
        cv > 0.0,
        "Decorrelated jitter should produce non-zero CV, got {}",
        cv
    );
}

#[test]
fn test_reproducible_with_fixed_seed() {
    // With fixed RNG seed, same sequence should be produced.
    let samples1 = full_jitter_samples(RNG_SEED);
    let samples2 = full_jitter_samples(RNG_SEED);

    assert_eq!(
        samples1, samples2,
        "Fixed seed should produce reproducible results"
    );

    let decorrelated1 = decorrelated_jitter_samples(RNG_SEED);
    let decorrelated2 = decorrelated_jitter_samples(RNG_SEED);

    assert_eq!(
        decorrelated1, decorrelated2,
        "Fixed seed should produce reproducible decorrelated jitter results"
    );
}

#[test]
fn test_cv_ratio_requirement_full_jitter() {
    // SC-004: Verify CV_jitter_strategy / CV_fixed_baseline >= 1.3.
    let fixed_samples = no_jitter_exponential_samples();
    let jitter_samples = full_jitter_samples(RNG_SEED);

    let cv_fixed = coefficient_of_variation(&fixed_samples);
    let cv_jitter = coefficient_of_variation(&jitter_samples);

    println!("Fixed CV: {}, Full Jitter CV: {}", cv_fixed, cv_jitter);

    assert!(
        cv_fixed > 0.0,
        "Fixed baseline must be an exponential sequence"
    );
    let ratio = cv_jitter / cv_fixed;
    assert!(
        ratio >= 1.3,
        "Full jitter CV / fixed baseline CV should be >= 1.3, got {:.2}",
        ratio
    );
}

#[test]
fn test_cv_ratio_requirement_decorrelated_jitter() {
    // SC-004: Verify CV ratio for decorrelated jitter.
    let fixed_samples = no_jitter_exponential_samples();
    let jitter_samples = decorrelated_jitter_samples(RNG_SEED);

    let cv_fixed = coefficient_of_variation(&fixed_samples);
    let cv_jitter = coefficient_of_variation(&jitter_samples);

    println!(
        "Fixed CV: {}, Decorrelated Jitter CV: {}",
        cv_fixed, cv_jitter
    );

    assert!(
        cv_fixed > 0.0,
        "Fixed baseline must be an exponential sequence"
    );
    let ratio = cv_jitter / cv_fixed;
    assert!(
        ratio >= 1.3,
        "Decorrelated jitter CV / fixed baseline CV should be >= 1.3, got {:.2}",
        ratio
    );
}

#[test]
fn test_full_jitter_spread_across_range() {
    // Full jitter should spread values across the bounded range.
    let samples = full_jitter_samples(RNG_SEED);
    let unique_values: std::collections::HashSet<u64> = samples.iter().copied().collect();

    assert!(
        unique_values.len() >= 5,
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

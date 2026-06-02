//! Role contract API for supervised units.
//!
//! This module owns user-facing role contracts and adapters that bridge those
//! contracts into the existing task factory runtime.

pub mod adapter;
pub mod context;
pub mod lifecycle;
pub mod result;
pub mod templates;
pub mod traits;

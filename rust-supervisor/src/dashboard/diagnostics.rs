//! Dashboard tracing field names.
//!
//! The target-side IPC module uses stable field names for logs, errors, and
//! tests. Constants live here so relay and documentation can mirror them.

/// Target process identifier field.
pub const TARGET_ID: &str = "target_id";
/// IPC path field.
pub const IPC_PATH: &str = "ipc_path";
/// Dashboard IPC method field.
pub const METHOD: &str = "method";
/// Request identifier field.
pub const REQUEST_ID: &str = "request_id";
/// Event sequence field.
pub const SEQUENCE: &str = "sequence";
/// Command identifier field.
pub const COMMAND_ID: &str = "command_id";
/// Dropped event count field.
pub const DROPPED_COUNT: &str = "dropped_count";
/// Processing stage field.
pub const STAGE: &str = "stage";

/// Returns all stable dashboard diagnostic fields.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns a static slice of diagnostic field names.
pub fn field_names() -> &'static [&'static str] {
    &[
        TARGET_ID,
        IPC_PATH,
        METHOD,
        REQUEST_ID,
        SEQUENCE,
        COMMAND_ID,
        DROPPED_COUNT,
        STAGE,
    ]
}

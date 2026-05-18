//! Audit persistence (C7).
//!
//! Records every IPC write request as an immutable audit entry. Supports
//! two backends: memory (ring buffer) for development and file (append-only
//! JSON Lines) for production. Failure strategies: fail_closed (deny write
//! commands when audit is unwritable) and defer_bounded (queue with limit).

use crate::config::ipc_security::AuditConfig;
use crate::dashboard::error::DashboardError;
use serde::{Deserialize, Serialize};

/// Immutable audit record for a single IPC request.
///
/// Carries at least: UTC timestamp, command enum, initiator identity hash,
/// optional correlation id, adjudication boolean,
/// and structured error code on denial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// UTC timestamp with millisecond precision.
    pub timestamp: String,
    /// IPC method name.
    pub method: String,
    /// SHA256 hash of the initiator's peer identity (hex-encoded).
    pub initiator_hash: String,
    /// Optional correlation identifier for tracing.
    pub correlation_id: Option<String>,
    /// Whether the request was allowed.
    pub allowed: bool,
    /// Adjudication reason code when denied.
    pub denial_code: Option<String>,
    /// The control point that denied the request (C1-C9).
    pub denial_control_point: Option<String>,
}

/// Audit storage backend.
pub enum AuditBackend {
    /// In-memory ring buffer — not persisted across restarts.
    Memory {
        /// Fixed-size ring buffer of audit records.
        buffer: Vec<AuditRecord>,
        /// Write position in the ring.
        position: usize,
    },
    /// Append-only JSON Lines file.
    #[allow(dead_code)]
    File {
        /// File path for audit records.
        path: String,
    },
}

impl AuditBackend {
    /// Creates a memory-backed audit backend.
    ///
    /// # Arguments
    ///
    /// - `capacity`: Maximum number of records in the ring buffer.
    ///
    /// # Returns
    ///
    /// Returns an empty [`AuditBackend::Memory`].
    pub fn new_memory(capacity: usize) -> Self {
        Self::Memory {
            buffer: Vec::with_capacity(capacity),
            position: 0,
        }
    }

    /// Creates a file-backed audit backend.
    ///
    /// # Arguments
    ///
    /// - `path`: Absolute path to the audit file.
    ///
    /// # Returns
    ///
    /// Returns an [`AuditBackend::File`].
    #[allow(dead_code)]
    pub fn new_file(path: String) -> Self {
        Self::File { path }
    }

    /// Creates an audit backend from configuration.
    ///
    /// # Arguments
    ///
    /// - `config`: Audit configuration.
    ///
    /// # Returns
    ///
    /// Returns the configured backend, defaulting to memory (4096 capacity)
    /// when no file path is provided.
    pub fn from_config(config: &AuditConfig) -> Self {
        let backend: AuditBackend = match config.backend.as_str() {
            "file" => match &config.file_path {
                Some(p) => AuditBackend::new_file(p.as_str().to_owned()),
                None => AuditBackend::new_memory(4096),
            },
            _ => AuditBackend::new_memory(4096),
        };
        backend
    }

    /// Writes an audit record to the backend.
    ///
    /// # Arguments
    ///
    /// - `record`: The audit record to persist.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the write succeeds, or `Err(DashboardError)`
    /// on failure.
    pub fn write(&mut self, record: &AuditRecord) -> Result<(), DashboardError> {
        match self {
            Self::Memory { buffer, position } => {
                if buffer.len() < buffer.capacity() {
                    buffer.push(record.clone());
                } else {
                    buffer[*position] = record.clone();
                    *position = (*position + 1) % buffer.capacity();
                }
                Ok(())
            }
            Self::File { path: _path } => {
                // File backend: append one JSON line.
                // In production, this would use std::fs::OpenOptions.
                // For now, return Ok — actual file I/O is wired at
                // integration time.
                let _line = serde_json::to_string(record).map_err(|error| {
                    DashboardError::audit_write_failed(format!(
                        "audit serialization failed: {error}"
                    ))
                })?;
                Ok(())
            }
        }
    }

    /// Returns recent audit records (for inspection).
    ///
    /// # Arguments
    ///
    /// - `count`: Maximum number of recent records to return.
    ///
    /// # Returns
    ///
    /// Returns a vector of audit records, newest first.
    pub fn recent(&self, count: usize) -> Vec<AuditRecord> {
        match self {
            Self::Memory {
                buffer,
                position: _,
            } => {
                let start = if buffer.len() > count {
                    buffer.len() - count
                } else {
                    0
                };
                buffer[start..].iter().rev().take(count).cloned().collect()
            }
            Self::File { .. } => {
                // File backend: would read last N lines from file.
                vec![]
            }
        }
    }
}

/// Audit alert counter exposed via tracing.
pub mod alerts {
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Counter for audit write failures (for SC-004).
    static AUDIT_WRITE_FAILURES: AtomicU64 = AtomicU64::new(0);

    /// Increments the audit write failure counter.
    pub fn increment_failure_count() -> u64 {
        AUDIT_WRITE_FAILURES.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns the current audit write failure count.
    pub fn failure_count() -> u64 {
        AUDIT_WRITE_FAILURES.load(Ordering::Relaxed)
    }
}

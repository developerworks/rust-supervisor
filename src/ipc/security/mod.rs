//! IPC security pipeline.
//!
//! Orchestrates the nine control points (C1-C9) in the contract-defined
//! execution order. The pipeline is loaded once from `IpcSecurityConfig`
//! and invoked per-request as a pre-dispatch filter by the dashboard IPC
//! service. C7 (audit) runs post-dispatch.

pub mod allowlist;
pub mod audit;
pub mod authz;
pub mod idempotency;
pub mod limits;
pub mod peer_identity;
pub mod replay;

use crate::config::ipc_security::IpcSecurityConfig;
use crate::dashboard::error::DashboardError;
use std::collections::HashMap;

use self::audit::AuditRecord;
use self::idempotency::IdempotencyCache;
use self::limits::TokenBucket;
use self::replay::ReplayWindow;

/// Assembled IPC security pipeline holding all control point instances.
pub struct IpcSecurityPipeline {
    /// Stored configuration for inspection.
    #[allow(dead_code)]
    config: IpcSecurityConfig,
    /// C4: replay protection sliding window.
    replay_window: ReplayWindow,
    /// C6: per-connection token buckets, keyed by connection identifier.
    rate_limiters: HashMap<String, TokenBucket>,
    /// C7: audit persistence backend.
    audit: audit::AuditBackend,
    /// C8: command idempotency cache.
    idempotency_cache: IdempotencyCache,
}

/// Outcome of pre-dispatch security checks.
pub enum CheckOutcome {
    /// All checks passed; proceed to dispatch.
    Passed,
    /// A control point denied the request; return this error.
    Denied(DashboardError),
}

impl IpcSecurityPipeline {
    /// Creates a new pipeline from configuration.
    ///
    /// # Arguments
    ///
    /// - `config`: IPC security configuration.
    ///
    /// # Returns
    ///
    /// Returns an initialized [`IpcSecurityPipeline`] with all control
    /// points ready.
    pub fn new(config: IpcSecurityConfig) -> Self {
        Self {
            replay_window: ReplayWindow::from_config(&config.replay_protection),
            rate_limiters: HashMap::new(),
            audit: audit::AuditBackend::from_config(&config.audit),
            idempotency_cache: IdempotencyCache::from_config(&config.idempotency),
            config,
        }
    }

    /// Runs pre-dispatch security checks.
    ///
    /// Execution order (per contract):
    /// C6 → C5 → C2 → C4 → C3
    ///
    /// C1 (socket owner) runs at bind time and is not in the per-request
    /// pipeline. C9 (allowlist) runs at extension points.
    ///
    /// # Arguments
    ///
    /// - `method`: IPC method name.
    /// - `request_id`: Request identifier (for C4 replay check and C8 cache).
    /// - `raw_body_len`: Byte length of the raw request body (for C5).
    /// - `peer_identity`: Extracted peer identity snapshot (for C2/C3).
    /// - `connection_id`: Opaque connection identifier (for per-connection C6).
    ///
    /// # Returns
    ///
    /// Returns `CheckOutcome::Passed` when all checks pass, or
    /// `CheckOutcome::Denied(error)` with the denial error. The caller
    /// must write audit records and execute the actual dispatch.
    pub fn check(
        &mut self,
        method: &str,
        request_id: &str,
        raw_body_len: usize,
        peer_identity: &peer_identity::PeerIdentity,
        connection_id: &str,
    ) -> CheckOutcome {
        // C6: Rate limit
        let rate_limiter = self
            .rate_limiters
            .entry(connection_id.to_string())
            .or_insert_with(|| TokenBucket::from_config(&self.config.rate_limit));
        if let Err(err) = rate_limiter.check_rate_limit(&self.config.rate_limit) {
            tracing::warn!(
                target: "rust_supervisor::ipc::security::rate_limit",
                %connection_id,
                "rate limit exceeded"
            );
            return CheckOutcome::Denied(err);
        }

        // C5: Size limit
        if let Err(err) = limits::check_request_size(raw_body_len, &self.config.request_size_limit)
        {
            tracing::warn!(
                target: "rust_supervisor::ipc::security::size_limit",
                actual = raw_body_len,
                limit = self.config.request_size_limit.max_bytes,
                "request too large"
            );
            return CheckOutcome::Denied(err);
        }

        // C2: Peer credentials
        if let Err(err) =
            peer_identity::verify_peer_identity(peer_identity, &self.config.peer_identity)
        {
            tracing::warn!(
                target: "rust_supervisor::ipc::security::peer_credentials",
                peer_uid = peer_identity.uid,
                "peer credential check failed"
            );
            return CheckOutcome::Denied(err);
        }

        // C4: Replay protection
        if self.config.replay_protection.enabled {
            if let Err(err) = self.replay_window.check_and_record(request_id) {
                tracing::warn!(
                    target: "rust_supervisor::ipc::security::replay",
                    %request_id,
                    "replay detected"
                );
                return CheckOutcome::Denied(err);
            }
        }

        // C3: Command authorization
        if let Err(err) =
            authz::verify_authorization(method, peer_identity.uid, &self.config.authorization)
        {
            tracing::warn!(
                target: "rust_supervisor::ipc::security::authorization",
                %method,
                peer_uid = peer_identity.uid,
                "command not authorized"
            );
            return CheckOutcome::Denied(err);
        }

        // C8: Idempotency — check cache before letting dispatch happen
        // The caller checks cache hit via `check_idempotency`.
        CheckOutcome::Passed
    }

    /// Checks the idempotency cache for a cached response (C8).
    ///
    /// Called after `check()` passes but before dispatch.
    ///
    /// # Arguments
    ///
    /// - `request_id`: Request identifier.
    ///
    /// # Returns
    ///
    /// Returns `Some(cached_result_json)` if a cached result exists,
    /// or `None` if no cache hit.
    pub fn check_idempotency(&self, request_id: &str) -> Option<String> {
        if self.config.idempotency.enabled {
            self.idempotency_cache.get(request_id)
        } else {
            None
        }
    }

    /// Caches a dispatch result for idempotency (C8).
    ///
    /// # Arguments
    ///
    /// - `request_id`: Request identifier.
    /// - `response_json`: Serialized response to cache.
    pub fn cache_result(&mut self, request_id: &str, response_json: &str) {
        if self.config.idempotency.enabled {
            self.idempotency_cache
                .put(request_id.to_string(), response_json.to_string());
        }
    }

    /// Writes an audit record after dispatch (C7).
    ///
    /// # Arguments
    ///
    /// - `method`: IPC method name.
    /// - `peer_identity`: Peer identity snapshot.
    /// - `allowed`: Whether the request was allowed.
    /// - `denial_error`: The denial error if denied.
    /// - `denial_control_point`: Which control point denied (C1-C9 or "dispatch").
    pub fn write_audit(
        &mut self,
        method: &str,
        peer_identity: &peer_identity::PeerIdentity,
        allowed: bool,
        denial_error: Option<&DashboardError>,
        denial_control_point: &str,
    ) {
        if !self.config.audit.enabled {
            return;
        }
        let hash = format!("uid:{}:pid:{}", peer_identity.uid, peer_identity.pid);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();
        let record = AuditRecord {
            timestamp: now,
            method: method.to_string(),
            initiator_hash: hash,
            correlation_id: None,
            allowed,
            denial_code: denial_error.map(|e| e.code.clone()),
            denial_control_point: if allowed {
                None
            } else {
                Some(denial_control_point.to_string())
            },
        };
        if let Err(_err) = self.audit.write(&record) {
            let count = audit::alerts::increment_failure_count();
            tracing::error!(
                target: "rust_supervisor::ipc::security::audit",
                failure_count = count,
                "audit write failed"
            );
        }
    }
}

//! IPC security pipeline.
//!
//! Orchestrates the nine control points (C1-C9) in the contract-defined
//! execution order. The pipeline is loaded once from `IpcSecurityConfig`
//! plus the root audit configuration, then invoked per-request as a
//! pre-dispatch filter by the dashboard IPC service. C7 (audit) runs
//! post-dispatch.

pub mod allowlist;
pub mod audit;
pub mod authz;
pub mod idempotency;
pub mod limits;
pub mod peer_identity;
pub mod replay;

use crate::config::audit::AuditConfig;
use crate::config::ipc_security::IpcSecurityConfig;
use crate::dashboard::error::DashboardError;
use std::collections::HashMap;

pub use self::audit::AuditFailureStrategy;
use self::audit::AuditRecord;
use self::idempotency::IdempotencyCache;
use self::limits::TokenBucket;
use self::replay::ReplayWindow;

/// Assembled IPC security pipeline holding all control point instances.
pub struct IpcSecurityPipeline {
    /// Stored configuration for inspection.
    config: IpcSecurityConfig,
    /// Root audit configuration used by C7.
    audit_config: AuditConfig,
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
    /// - `audit_config`: Root audit persistence configuration.
    ///
    /// # Returns
    ///
    /// Returns an initialized [`IpcSecurityPipeline`] with all control
    /// points ready.
    pub fn new(config: IpcSecurityConfig, audit_config: AuditConfig) -> Self {
        Self {
            replay_window: ReplayWindow::from_config(&config.replay_protection),
            rate_limiters: HashMap::new(),
            audit: audit::AuditBackend::from_config(&audit_config),
            idempotency_cache: IdempotencyCache::from_config(&config.idempotency),
            config,
            audit_config,
        }
    }

    /// Runs pre-dispatch security checks, excluding C4 (replay) and C8
    /// (idempotency) which are handled separately to resolve ordering.
    ///
    /// Execution order (per contract, with corrected C4/C8 ordering):
    /// C6 → C5 → C2 → C3
    ///
    /// C4 (replay protection) and C8 (idempotency) are ordered by the
    /// caller: C8 is checked first; if the response is already cached,
    /// it is returned without recording in C4. Otherwise C4 records the
    /// request_id and dispatch proceeds — see `check_replay_and_record`.
    ///
    /// C1 (socket owner) runs at bind time. C9 (allowlist) runs at
    /// extension points. C7 (audit) runs post-dispatch.
    ///
    /// # Arguments
    ///
    /// - `method`: IPC method name.
    /// - `raw_body_len`: Byte length of the raw request body (for C5).
    /// - `peer_identity`: Extracted peer identity snapshot (for C2/C3).
    /// - `connection_id`: Opaque connection identifier (for per-connection C6).
    ///
    /// # Returns
    ///
    /// Returns `CheckOutcome::Passed` when all checks pass, or
    /// `CheckOutcome::Denied(error)` with the denial error.
    pub fn check(
        &mut self,
        method: &str,
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

        CheckOutcome::Passed
    }

    /// Checks C4 replay protection and records the request_id.
    ///
    /// Must be called **after** C8 idempotency check: if the response is
    /// already cached, the caller returns early without recording in C4.
    /// This prevents C4 from rejecting a legitimate retry that hits the
    /// idempotency cache.
    ///
    /// # Arguments
    ///
    /// - `request_id`: Request identifier to check and record.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` for a first submission, or
    /// `Err(DashboardError)` with code `replay_detected` for a replay.
    pub fn check_replay_and_record(&mut self, request_id: &str) -> Result<(), DashboardError> {
        if !self.config.replay_protection.enabled {
            return Ok(());
        }
        self.replay_window.check_and_record(request_id)
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

    /// Returns the C5 request size limit in bytes, or `None` if disabled.
    pub fn request_size_limit_bytes(&self) -> Option<usize> {
        if self.config.request_size_limit.enabled {
            Some(self.config.request_size_limit.max_bytes)
        } else {
            None
        }
    }

    /// Returns the configured high-risk command method list.
    ///
    /// This is the list from `AuthorizationConfig.high_risk_commands`.
    /// When empty (the config is not populated), the caller should
    /// fall back to a hardcoded default set.
    pub fn high_risk_methods(&self) -> &[String] {
        &self.config.authorization.high_risk_commands
    }

    /// Checks whether an external command path is allowed by C9 allowlist.
    ///
    /// This is called at extension points where external executables may
    /// be invoked under supervisor control. The allowlist rejects paths
    /// that are not explicitly configured.
    ///
    /// # Arguments
    ///
    /// - `path`: Absolute executable path to check.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the path is allowed, or
    /// `Err(DashboardError)` with `allowlist_empty` or `allowlist_denied`.
    pub fn check_ext_command_allowlist(&self, path: &str) -> Result<(), DashboardError> {
        crate::ipc::security::allowlist::check_allowlist(path, &self.config.allowlist)
    }

    /// Returns a reference to the audit backend for health checks.
    pub fn audit_backend(&self) -> &audit::AuditBackend {
        &self.audit
    }

    /// Returns the failure strategy from config.
    fn audit_failure_strategy(&self) -> audit::AuditFailureStrategy {
        audit::AuditFailureStrategy::from_config(&self.audit_config.failure_strategy)
    }

    /// Writes an audit record after dispatch (C7).
    ///
    /// Returns `Ok(())` on success or `Err(DashboardError)` when the audit
    /// backend is unwritable. The caller should fail closed for high-risk
    /// commands.
    ///
    /// When `is_high_risk` is `true` and the configured failure strategy is
    /// `fail_closed`, any backend write error is propagated so the caller
    /// can reject the command. When `defer_bounded`, errors are logged but
    /// not propagated.
    ///
    /// # Arguments
    ///
    /// - `method`: IPC method name.
    /// - `peer_identity`: Peer identity snapshot.
    /// - `allowed`: Whether the request was allowed.
    /// - `denial_error`: The denial error if denied.
    /// - `denial_control_point`: Which control point denied (C1-C9 or "dispatch").
    /// - `is_high_risk`: Whether this is a high-risk command (e.g. restart, shutdown).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the audit record was written, or
    /// `Err(DashboardError)` when the backend is unwritable and the
    /// failure strategy requires fail-closed.
    pub fn write_audit(
        &mut self,
        method: &str,
        peer_identity: &peer_identity::PeerIdentity,
        allowed: bool,
        denial_error: Option<&DashboardError>,
        denial_control_point: &str,
        is_high_risk: bool,
    ) -> Result<(), DashboardError> {
        if !self.audit_config.enabled {
            return Ok(());
        }
        let hash = format!(
            "uid:{uid}:pid:{pid}",
            uid = peer_identity.uid,
            pid = peer_identity.pid
        );
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
        let strategy = self.audit_failure_strategy();
        let write_result = self.audit.write(&record);
        match write_result {
            Ok(()) => Ok(()),
            Err(err) => {
                let count = audit::alerts::increment_failure_count();
                tracing::error!(
                    target: "rust_supervisor::ipc::security::audit",
                    failure_count = count,
                    ?err,
                    high_risk = is_high_risk,
                    strategy = ?strategy,
                    "audit write failed"
                );
                // fail_closed + high-risk → propagate error so caller can reject
                if is_high_risk && strategy == audit::AuditFailureStrategy::FailClosed {
                    Err(err)
                } else {
                    // defer_bounded or non-high-risk: log only, swallow error
                    Ok(())
                }
            }
        }
    }
}

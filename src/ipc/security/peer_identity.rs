//! Peer identity verification (C1, C2).
//!
//! C1: socket owner verification — validates socket file ownership
//!      before bind, rejects symlinks.
//! C2: peer credentials verification — validates connecting process
//!      identity via SO_PEERCRED / LOCAL_PEERCRED.

use crate::config::ipc_security::PeerIdentityConfig;
use crate::dashboard::error::DashboardError;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::UnixStream as StdUnixStream;

// ---------------------------------------------------------------------------
// Peer identity record
// ---------------------------------------------------------------------------

/// Record of peer identity taken from a connected Unix socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentity {
    /// Process identifier of the peer.
    pub pid: u32,
    /// User identifier of the peer.
    pub uid: u32,
    /// Group identifier of the peer.
    pub gid: u32,
}

/// Extracts peer identity from a connected std Unix stream.
///
/// Linux: uses `SO_PEERCRED` → `libc::ucred`.
/// macOS / FreeBSD: uses `LOCAL_PEERCRED` → `libc::xucred`
/// (provides uid only; gid set to 0).
///
/// # Arguments
///
/// - `stream`: A connected `std::os::unix::net::UnixStream`.
///
/// # Returns
///
/// Returns `PeerIdentity` on success, or `DashboardError` if the kernel
/// does not support peer credentials on this platform.
pub fn extract_peer_identity(stream: &StdUnixStream) -> Result<PeerIdentity, DashboardError> {
    #[cfg(target_os = "linux")]
    {
        extract_peer_identity_linux(stream)
    }
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    {
        extract_peer_identity_macos(stream)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
    {
        let _ = stream;
        Err(DashboardError::peer_cred_unavailable(
            "peer credentials not supported on this platform",
        ))
    }
}

#[cfg(target_os = "linux")]
/// Extracts peer identity via Linux SO_PEERCRED.
fn extract_peer_identity_linux(stream: &StdUnixStream) -> Result<PeerIdentity, DashboardError> {
    use std::os::unix::io::AsRawFd;

    let fd = stream.as_raw_fd();
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut cred_len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;

    let ret = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut _ as *mut libc::c_void,
            &mut cred_len,
        )
    };

    if ret != 0 {
        return Err(DashboardError::peer_cred_unavailable(format!(
            "getsockopt SO_PEERCRED failed: {}",
            std::io::Error::last_os_error()
        )));
    }

    Ok(PeerIdentity {
        pid: cred.pid as u32,
        uid: cred.uid,
        gid: cred.gid,
    })
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
/// Extracts peer identity via `LOCAL_PEERCRED` (macOS / FreeBSD `xucred`).
///
/// On macOS the credential structure provides `cr_uid` and `cr_groups[]`
/// but no single `cr_gid`. This function uses `cr_uid` for identity and
/// sets `gid` to 0 (gid checks are opt-in).
///
/// # Arguments
///
/// - `stream`: Connected Unix-domain stream socket.
///
/// # Returns
///
/// Returns [`PeerIdentity`] on success, or a `DashboardError` when
/// `getsockopt` fails.
fn extract_peer_identity_macos(stream: &StdUnixStream) -> Result<PeerIdentity, DashboardError> {
    use std::os::unix::io::AsRawFd;

    let fd = stream.as_raw_fd();
    let mut cred: libc::xucred = unsafe { std::mem::zeroed() };
    let mut cred_len = std::mem::size_of::<libc::xucred>() as libc::socklen_t;

    let ret = unsafe {
        libc::getsockopt(
            fd,
            0, // SOL_LOCAL
            libc::LOCAL_PEERCRED,
            &mut cred as *mut _ as *mut libc::c_void,
            &mut cred_len,
        )
    };

    if ret != 0 {
        return Err(DashboardError::peer_cred_unavailable(format!(
            "getsockopt LOCAL_PEERCRED failed: {}",
            std::io::Error::last_os_error()
        )));
    }

    // macOS xucred provides cr_uid and cr_groups[] but no single cr_gid.
    // Use cr_uid for identity; gid is set to 0 (gid checks are opt-in).
    let gid = if cred.cr_ngroups > 0 {
        cred.cr_groups[0] as u32
    } else {
        0
    };

    Ok(PeerIdentity {
        pid: 0, // macOS LOCAL_PEERCRED does not provide pid
        uid: cred.cr_uid,
        gid,
    })
}

/// Verifies peer identity against the configured identity expectations.
///
/// # Arguments
///
/// - `peer`: Extracted peer identity record.
/// - `config`: Peer identity verification configuration.
///
/// # Returns
///
/// Returns `Ok(())` when the peer passes all checks, or `Err(DashboardError)`
/// with the first failing check's error code.
pub fn verify_peer_identity(
    peer: &PeerIdentity,
    config: &PeerIdentityConfig,
) -> Result<(), DashboardError> {
    if !config.enabled {
        return Ok(());
    }

    // C2: uid match
    if config.require_uid_match {
        let my_uid = unsafe { libc::getuid() };
        if peer.uid != my_uid {
            return Err(DashboardError::peer_cred_uid_mismatch(my_uid, peer.uid));
        }
    }

    // C2: gid whitelist
    if !config.allowed_gids.is_empty() && !config.allowed_gids.contains(&peer.gid) {
        return Err(DashboardError::peer_cred_gid_not_allowed(peer.gid));
    }

    // C2: pid whitelist
    if !config.allowed_pids.is_empty() && !config.allowed_pids.contains(&peer.pid) {
        return Err(DashboardError::peer_cred_pid_not_allowed(peer.pid));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// C1: Socket owner & symlink checks for bind preparation
// ---------------------------------------------------------------------------

/// Prepares a socket path for binding (C1).
///
/// Rejects symlinks and validates socket file ownership before allowing
/// a replacement bind. This is called before `tokio::net::UnixListener::bind`.
///
/// # Arguments
///
/// - `path`: The configured socket file path.
///
/// # Returns
///
/// Returns `Ok(())` when binding may proceed. Returns `Err(DashboardError)`
/// with `ipc_symlink_rejected` or `ipc_socket_owner_mismatch` on failure.
pub fn prepare_socket_path_for_bind(path: &std::path::Path) -> Result<(), DashboardError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(DashboardError::new(
                "ipc_bind",
                "ipc_bind",
                None,
                format!("failed to stat socket path: {e}"),
                false,
            ));
        }
    };

    // Reject symlinks
    if metadata.file_type().is_symlink() {
        return Err(DashboardError::new(
            "ipc_symlink_rejected",
            "ipc_bind",
            None,
            "IPC path is a symlink — rejected for security",
            false,
        ));
    }

    // If path exists and is a socket, check ownership
    if metadata.file_type().is_socket() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let owner_uid = metadata.uid();
            let my_uid = unsafe { libc::getuid() };
            if owner_uid != my_uid {
                return Err(DashboardError::ipc_socket_owner_mismatch(format!(
                    "socket owner uid {owner_uid} != process uid {my_uid}"
                )));
            }
        }
    }

    Ok(())
}

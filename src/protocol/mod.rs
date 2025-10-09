//! Remote synchronization protocol implementation
//!
//! This module implements the rsync wire protocol for compatibility with
//! existing rsync servers, as well as modern extensions using QUIC and
//! merkle trees.

pub mod rsync;
pub mod ssh;

#[cfg(feature = "quic")]
pub mod quic;

use crate::cli::{Args, Location};
use crate::sync::SyncStats;
use anyhow::Result;

/// Main entry point for remote sync operations
pub async fn remote_sync(
    args: &Args,
    source: &Location,
    destination: &Location,
) -> Result<SyncStats> {
    // Determine sync direction
    match (source, destination) {
        (Location::Local(src), Location::Remote { user, host, path }) => {
            // Push: local → remote
            push_to_remote(args, src, user.as_deref(), host, path).await
        }
        (Location::Remote { user, host, path }, Location::Local(dest)) => {
            // Pull: remote → local
            pull_from_remote(args, user.as_deref(), host, path, dest).await
        }
        (Location::Remote { .. }, Location::Remote { .. }) => {
            anyhow::bail!("Remote-to-remote sync not supported yet")
        }
        (Location::Local(_), Location::Local(_)) => {
            unreachable!("Local-to-local should have been handled by sync_files")
        }
    }
}

/// Push files from local to remote
async fn push_to_remote(
    args: &Args,
    local_path: &std::path::Path,
    user: Option<&str>,
    host: &str,
    remote_path: &std::path::Path,
) -> Result<SyncStats> {
    // Connect to remote via SSH
    let mut connection = ssh::SshConnection::connect(
        host,
        user.unwrap_or_else(|| whoami::username().as_str()),
        &args.remote_shell,
    )
    .await?;

    // Start remote arsync in server mode
    connection.start_server(remote_path).await?;

    // Try to negotiate QUIC if supported
    #[cfg(feature = "quic")]
    {
        if let Ok(quic_conn) = quic::negotiate_quic(&mut connection).await {
            return quic::push_via_quic(args, local_path, quic_conn).await;
        }
    }

    // Fall back to rsync wire protocol over SSH
    rsync::push_via_rsync_protocol(args, local_path, &mut connection).await
}

/// Pull files from remote to local
async fn pull_from_remote(
    args: &Args,
    user: Option<&str>,
    host: &str,
    remote_path: &std::path::Path,
    local_path: &std::path::Path,
) -> Result<SyncStats> {
    // Connect to remote via SSH
    let mut connection = ssh::SshConnection::connect(
        host,
        user.unwrap_or_else(|| whoami::username().as_str()),
        &args.remote_shell,
    )
    .await?;

    // Start remote arsync in server mode
    connection.start_server(remote_path).await?;

    // Try to negotiate QUIC if supported
    #[cfg(feature = "quic")]
    {
        if let Ok(quic_conn) = quic::negotiate_quic(&mut connection).await {
            return quic::pull_via_quic(args, quic_conn, local_path).await;
        }
    }

    // Fall back to rsync wire protocol over SSH
    rsync::pull_via_rsync_protocol(args, &mut connection, local_path).await
}

//! Remote synchronization protocol implementation
//!
//! This module implements the rsync wire protocol for compatibility with
//! existing rsync servers, as well as modern extensions using QUIC and
//! merkle trees.

pub mod checksum;
pub mod pipe;
pub mod rsync;
pub mod rsync_compat;
pub mod ssh;
pub mod transport;
pub mod varint;

#[cfg(feature = "quic")]
pub mod quic;

use crate::cli::{Args, Location};
use crate::sync::SyncStats;
use anyhow::Result;
use std::path::Path;

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
    let username = user.map(String::from).unwrap_or_else(whoami::username);
    let mut connection = ssh::SshConnection::connect(host, &username, &args.remote_shell).await?;

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
    let username = user.map(String::from).unwrap_or_else(whoami::username);
    let mut connection = ssh::SshConnection::connect(host, &username, &args.remote_shell).await?;

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

/// Pipe sender mode (for protocol testing)
pub async fn pipe_sender(args: &Args, source: &Location) -> Result<SyncStats> {
    let source_path = match source {
        Location::Local(path) => path,
        Location::Remote { .. } => {
            anyhow::bail!("Pipe mode requires local source path");
        }
    };

    // Create pipe transport from stdin/stdout
    let transport = pipe::PipeTransport::from_stdio()?;

    // Send via rsync protocol
    rsync::send_via_pipe(args, source_path, transport).await
}

/// Pipe receiver mode (for protocol testing)
pub async fn pipe_receiver(args: &Args, destination: &Location) -> Result<SyncStats> {
    let dest_path = match destination {
        Location::Local(path) => path,
        Location::Remote { .. } => {
            anyhow::bail!("Pipe mode requires local destination path");
        }
    };

    // Create pipe transport from stdin/stdout
    let transport = pipe::PipeTransport::from_stdio()?;

    // Receive via rsync protocol
    rsync::receive_via_pipe(args, transport, dest_path).await
}

//! rsync wire protocol implementation
//!
//! Implements the rsync wire protocol for compatibility with rsync servers.
//! Based on the rsync technical report and protocol specification.

use crate::cli::Args;
use crate::protocol::checksum::{rolling_checksum, rolling_checksum_update, strong_checksum};
use crate::protocol::pipe::PipeTransport;
use crate::protocol::ssh::SshConnection;
use crate::protocol::transport::{self, Transport};
use crate::sync::SyncStats;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use walkdir;

/// Protocol version we support
const PROTOCOL_VERSION: u8 = 31; // rsync 3.2+

/// Minimum protocol version we accept
const MIN_PROTOCOL_VERSION: u8 = 27; // rsync 3.0+

/// Default block size for checksums (rsync default)
const DEFAULT_BLOCK_SIZE: usize = 700;

/// Minimum block size
const MIN_BLOCK_SIZE: usize = 128;

/// Rolling checksum constants (Adler-32 style)
const ROLLING_MODULUS: u32 = 65521;

/// Push files to remote using rsync protocol over SSH
pub async fn push_via_rsync_protocol(
    _args: &Args,
    _local_path: &Path,
    _connection: &mut SshConnection,
) -> Result<SyncStats> {
    let _start = Instant::now();

    // TODO: Implement rsync protocol
    // Phase 1: Protocol handshake
    // Phase 2: File list generation and exchange
    // Phase 3: Block checksum exchange (per file)
    // Phase 4: Delta generation and transmission
    // Phase 5: Metadata update

    anyhow::bail!("rsync protocol implementation in progress")
}

/// Pull files from remote using rsync protocol over SSH
pub async fn pull_via_rsync_protocol(
    _args: &Args,
    _connection: &mut SshConnection,
    _local_path: &Path,
) -> Result<SyncStats> {
    let _start = Instant::now();

    // TODO: Implement rsync protocol (receiver side)

    anyhow::bail!("rsync protocol implementation in progress")
}

/// rsync protocol handshake
async fn handshake(_connection: &mut SshConnection) -> Result<u8> {
    // TODO: Send our protocol version
    // TODO: Receive remote protocol version
    // TODO: Negotiate common version
    Ok(PROTOCOL_VERSION)
}

/// File list entry in rsync protocol
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub mtime: i64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
}

/// Generate file list for transmission
async fn generate_file_list(_path: &Path, _args: &Args) -> Result<Vec<FileEntry>> {
    // TODO: Traverse directory
    // TODO: Generate FileEntry for each file
    // TODO: Handle recursion, symlinks, etc.
    Ok(vec![])
}

/// Send file list over connection
async fn send_file_list(_connection: &mut SshConnection, _files: &[FileEntry]) -> Result<()> {
    // TODO: Encode file list in rsync wire format
    // TODO: Send over SSH connection
    Ok(())
}

/// Receive file list from connection
async fn receive_file_list(_connection: &mut SshConnection) -> Result<Vec<FileEntry>> {
    // TODO: Receive and decode file list
    Ok(vec![])
}

/// Block checksum for rsync algorithm
#[derive(Debug, Clone)]
pub struct BlockChecksum {
    pub weak_checksum: u32,        // Rolling checksum (Adler-32 style)
    pub strong_checksum: [u8; 16], // MD5 or SHA-256
}

/// Generate block checksums for a file
async fn generate_block_checksums(_path: &Path, _block_size: usize) -> Result<Vec<BlockChecksum>> {
    // TODO: Read file in blocks
    // TODO: Compute weak checksum (rolling)
    // TODO: Compute strong checksum (MD5/SHA-256)
    Ok(vec![])
}

/// rsync delta instruction
#[derive(Debug, Clone)]
pub enum DeltaInstruction {
    /// Copy block from local file at given offset
    CopyBlock { offset: u64, length: usize },
    /// Literal data to write
    LiteralData(Vec<u8>),
}

/// Generate delta for a file using rsync algorithm
async fn generate_delta(
    _local_file: &Path,
    _remote_checksums: &[BlockChecksum],
) -> Result<Vec<DeltaInstruction>> {
    // TODO: Implement rsync rolling checksum algorithm
    // TODO: Find matching blocks
    // TODO: Generate delta instructions
    Ok(vec![])
}

/// Apply delta to reconstruct file
async fn apply_delta(
    _local_file: &Path,
    _delta: &[DeltaInstruction],
    _output: &Path,
) -> Result<()> {
    // TODO: Read local file (if exists)
    // TODO: Apply delta instructions
    // TODO: Write output file
    Ok(())
}

// ============================================================================
// Pipe Mode Implementation (for testing)
// ============================================================================

/// Send files via pipe transport (for testing)
pub async fn send_via_pipe(
    args: &Args,
    source_path: &Path,
    mut transport: PipeTransport,
) -> Result<SyncStats> {
    let start = Instant::now();

    debug!("Sender: Starting protocol handshake");

    // Phase 1: Handshake
    let remote_version = handshake_sender(&mut transport).await?;
    debug!(
        "Sender: Handshake complete, remote version: {}",
        remote_version
    );

    // Phase 2: Send file list
    debug!(
        "Sender: Generating file list from: {}",
        source_path.display()
    );
    let files = generate_file_list_simple(source_path, args).await?;
    info!("Sender: Found {} files to send", files.len());

    send_file_list_simple(&mut transport, &files).await?;
    debug!("Sender: File list sent");

    // Phase 3: Send file contents
    // For now, send whole files as literal data (no delta/checksum optimization)
    let mut bytes_sent = 0u64;
    for file in &files {
        debug!("Sender: Sending file: {}", file.path);
        let file_path = source_path.join(&file.path);

        // Read entire file content
        let content = fs::read(&file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", file.path, e))?;

        // Send content length (for verification, though receiver already knows from file list)
        let content_len = content.len() as u64;
        transport::write_all(&mut transport, &content_len.to_le_bytes()).await?;

        // Send file content
        transport::write_all(&mut transport, &content).await?;
        bytes_sent += content.len() as u64;

        debug!("Sender: Sent {} bytes for {}", content.len(), file.path);
    }

    // Flush to ensure all data is sent
    transport.flush().await?;

    info!("Sender: Transfer complete, sent {} bytes", bytes_sent);

    Ok(SyncStats {
        files_copied: files.len() as u64,
        bytes_copied: files.iter().map(|f| f.size).sum(),
        duration: start.elapsed(),
    })
}

/// Receive files via pipe transport (for testing)
pub async fn receive_via_pipe(
    args: &Args,
    mut transport: PipeTransport,
    dest_path: &Path,
) -> Result<SyncStats> {
    let start = Instant::now();

    debug!("Receiver: Starting protocol handshake");

    // Phase 1: Handshake
    let remote_version = handshake_receiver(&mut transport).await?;
    debug!(
        "Receiver: Handshake complete, remote version: {}",
        remote_version
    );

    // Phase 2: Receive file list
    debug!("Receiver: Receiving file list");
    let files = receive_file_list_simple(&mut transport).await?;
    info!("Receiver: Received {} files", files.len());

    // Phase 3: Receive file contents and apply metadata
    let mut bytes_received = 0u64;
    for file in &files {
        debug!("Receiver: Receiving file: {}", file.path);
        let file_path = dest_path.join(&file.path);

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if file.is_symlink {
            // Handle symlink
            if let Some(target) = &file.symlink_target {
                debug!("Receiver: Creating symlink {} -> {}", file.path, target);
                #[cfg(unix)]
                {
                    use std::os::unix::fs as unix_fs;
                    unix_fs::symlink(target, &file_path)?;
                }
            }
        } else {
            // Regular file - receive content
            let mut len_buf = [0u8; 8];
            transport::read_exact(&mut transport, &mut len_buf).await?;
            let content_len = u64::from_le_bytes(len_buf);

            // Verify length matches what we expect
            if content_len != file.size {
                anyhow::bail!(
                    "File size mismatch for {}: expected {}, got {}",
                    file.path,
                    file.size,
                    content_len
                );
            }

            // Receive file content
            let mut content = vec![0u8; content_len as usize];
            transport::read_exact(&mut transport, &mut content).await?;

            // Write to file
            fs::write(&file_path, &content)?;
            bytes_received += content.len() as u64;

            debug!("Receiver: Wrote {} bytes to {}", content.len(), file.path);
        }

        // Apply metadata (permissions, timestamps, ownership)
        apply_metadata(&file_path, file)?;
    }

    info!(
        "Receiver: Transfer complete, received {} bytes",
        bytes_received
    );

    Ok(SyncStats {
        files_copied: files.len() as u64,
        bytes_copied: files.iter().map(|f| f.size).sum(),
        duration: start.elapsed(),
    })
}

// ============================================================================
// Protocol Implementation (Minimal for Testing)
// ============================================================================

/// Handshake as sender
async fn handshake_sender<T: Transport>(transport: &mut T) -> Result<u8> {
    // Send our protocol version
    transport::write_all(transport, &[PROTOCOL_VERSION]).await?;

    // Receive remote protocol version
    let mut version_buf = [0u8; 1];
    transport::read_exact(transport, &mut version_buf).await?;
    let remote_version = version_buf[0];

    if remote_version < MIN_PROTOCOL_VERSION {
        anyhow::bail!(
            "Remote protocol version {} too old (need at least {})",
            remote_version,
            MIN_PROTOCOL_VERSION
        );
    }

    Ok(remote_version)
}

/// Handshake as receiver
async fn handshake_receiver<T: Transport>(transport: &mut T) -> Result<u8> {
    // Receive remote protocol version
    let mut version_buf = [0u8; 1];
    transport::read_exact(transport, &mut version_buf).await?;
    let remote_version = version_buf[0];

    if remote_version < MIN_PROTOCOL_VERSION {
        anyhow::bail!(
            "Remote protocol version {} too old (need at least {})",
            remote_version,
            MIN_PROTOCOL_VERSION
        );
    }

    // Send our protocol version
    transport::write_all(transport, &[PROTOCOL_VERSION]).await?;

    Ok(remote_version)
}

/// Generate simple file list (minimal implementation)
async fn generate_file_list_simple(path: &Path, args: &Args) -> Result<Vec<FileEntry>> {
    let mut files = Vec::new();

    if path.is_file() {
        // Single file
        let metadata = fs::metadata(path)?;
        files.push(FileEntry {
            path: path
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("No filename"))?
                .to_string_lossy()
                .to_string(),
            size: metadata.len(),
            mtime: metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64,
            mode: metadata.permissions().mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            is_symlink: false,
            symlink_target: None,
        });
    } else if path.is_dir() && args.should_recurse() {
        // Recursively list directory (blocking I/O for simplicity in protocol code)
        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                let rel_path = entry
                    .path()
                    .strip_prefix(path)?
                    .to_string_lossy()
                    .to_string();
                files.push(FileEntry {
                    path: rel_path,
                    size: metadata.len(),
                    mtime: metadata
                        .modified()?
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs() as i64,
                    mode: metadata.permissions().mode(),
                    uid: metadata.uid(),
                    gid: metadata.gid(),
                    is_symlink: false,
                    symlink_target: None,
                });
            } else if metadata.is_symlink() {
                let symlink_target = fs::read_link(entry.path())?.to_string_lossy().to_string();
                let rel_path = entry
                    .path()
                    .strip_prefix(path)?
                    .to_string_lossy()
                    .to_string();
                files.push(FileEntry {
                    path: rel_path,
                    size: 0, // Symlinks have no size
                    mtime: metadata
                        .modified()?
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs() as i64,
                    mode: metadata.permissions().mode(),
                    uid: metadata.uid(),
                    gid: metadata.gid(),
                    is_symlink: true,
                    symlink_target: Some(symlink_target),
                });
            }
        }
    }

    Ok(files)
}

/// Apply full metadata to a file
fn apply_metadata(path: &Path, file: &FileEntry) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Set permissions
    let permissions = std::fs::Permissions::from_mode(file.mode);
    if let Err(e) = fs::set_permissions(path, permissions) {
        warn!("Failed to set permissions on {}: {}", path.display(), e);
    }

    // Set ownership (requires root privileges, so we'll try but not fail)
    #[cfg(unix)]
    {
        // Note: chown/lchown not in std, would need nix crate
        // For now, we'll skip ownership (rsync also needs --owner --group flags)
        // This matches rsync's behavior when run without privileges
        debug!(
            "Skipping ownership for {} (uid={}, gid={}) - requires privileges",
            path.display(),
            file.uid,
            file.gid
        );
    }

    // Set modification time
    if !file.is_symlink {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mtime = UNIX_EPOCH + Duration::from_secs(file.mtime as u64);
        if let Err(e) = filetime::set_file_mtime(path, filetime::FileTime::from_system_time(mtime))
        {
            warn!("Failed to set mtime on {}: {}", path.display(), e);
        }
    }

    Ok(())
}

/// Send file list (with full metadata)
async fn send_file_list_simple<T: Transport>(transport: &mut T, files: &[FileEntry]) -> Result<()> {
    // Send file count as 4-byte little-endian
    let count = files.len() as u32;
    transport::write_all(transport, &count.to_le_bytes()).await?;

    // Send each file entry
    for file in files {
        // Flags byte: bit 0 = is_symlink
        let flags = if file.is_symlink { 1u8 } else { 0u8 };
        transport::write_all(transport, &[flags]).await?;

        // Path length + path
        let path_bytes = file.path.as_bytes();
        let path_len = path_bytes.len() as u32;
        transport::write_all(transport, &path_len.to_le_bytes()).await?;
        transport::write_all(transport, path_bytes).await?;

        // File size
        transport::write_all(transport, &file.size.to_le_bytes()).await?;

        // Full metadata
        transport::write_all(transport, &file.mtime.to_le_bytes()).await?;
        transport::write_all(transport, &file.mode.to_le_bytes()).await?;
        transport::write_all(transport, &file.uid.to_le_bytes()).await?;
        transport::write_all(transport, &file.gid.to_le_bytes()).await?;

        // Symlink target if applicable
        if file.is_symlink {
            if let Some(target) = &file.symlink_target {
                let target_bytes = target.as_bytes();
                let target_len = target_bytes.len() as u32;
                transport::write_all(transport, &target_len.to_le_bytes()).await?;
                transport::write_all(transport, target_bytes).await?;
            }
        }
    }

    Ok(())
}

/// Receive file list (with full metadata)
async fn receive_file_list_simple<T: Transport>(transport: &mut T) -> Result<Vec<FileEntry>> {
    // Receive file count
    let mut count_buf = [0u8; 4];
    transport::read_exact(transport, &mut count_buf).await?;
    let count = u32::from_le_bytes(count_buf) as usize;

    let mut files = Vec::with_capacity(count);

    // Receive each file entry
    for _ in 0..count {
        // Flags byte
        let mut flags_buf = [0u8; 1];
        transport::read_exact(transport, &mut flags_buf).await?;
        let is_symlink = (flags_buf[0] & 1) != 0;

        // Path length
        let mut path_len_buf = [0u8; 4];
        transport::read_exact(transport, &mut path_len_buf).await?;
        let path_len = u32::from_le_bytes(path_len_buf) as usize;

        // Path
        let mut path_buf = vec![0u8; path_len];
        transport::read_exact(transport, &mut path_buf).await?;
        let path = String::from_utf8(path_buf)?;

        // File size
        let mut size_buf = [0u8; 8];
        transport::read_exact(transport, &mut size_buf).await?;
        let size = u64::from_le_bytes(size_buf);

        // Full metadata
        let mut mtime_buf = [0u8; 8];
        transport::read_exact(transport, &mut mtime_buf).await?;
        let mtime = i64::from_le_bytes(mtime_buf);

        let mut mode_buf = [0u8; 4];
        transport::read_exact(transport, &mut mode_buf).await?;
        let mode = u32::from_le_bytes(mode_buf);

        let mut uid_buf = [0u8; 4];
        transport::read_exact(transport, &mut uid_buf).await?;
        let uid = u32::from_le_bytes(uid_buf);

        let mut gid_buf = [0u8; 4];
        transport::read_exact(transport, &mut gid_buf).await?;
        let gid = u32::from_le_bytes(gid_buf);

        // Symlink target if applicable
        let symlink_target = if is_symlink {
            let mut target_len_buf = [0u8; 4];
            transport::read_exact(transport, &mut target_len_buf).await?;
            let target_len = u32::from_le_bytes(target_len_buf) as usize;

            let mut target_buf = vec![0u8; target_len];
            transport::read_exact(transport, &mut target_buf).await?;
            Some(String::from_utf8(target_buf)?)
        } else {
            None
        };

        files.push(FileEntry {
            path,
            size,
            mtime,
            mode,
            uid,
            gid,
            is_symlink,
            symlink_target,
        });
    }

    Ok(files)
}

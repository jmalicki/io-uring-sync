//! rsync Wire Protocol Compatibility Layer
//!
//! This module implements the actual rsync wire protocol for interoperability
//! with rsync processes. rsync uses a multiplexed I/O protocol with message tags.

use crate::protocol::rsync::FileEntry;
use crate::protocol::transport::{self, Transport};
use crate::protocol::varint::{self, encode_varint, encode_varint_into};
use anyhow::Result;
use std::io::Cursor;
use tracing::{debug, warn};

/// rsync protocol uses multiplexed I/O with tags
/// Tag values from rsync source code (io.c)
const MPLEX_BASE: u8 = 7;

/// Message tags (rsync protocol)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageTag {
    Data = MPLEX_BASE,            // Regular data
    ErrorXfer = MPLEX_BASE + 1,   // Transfer error
    Info = MPLEX_BASE + 2,        // Info message
    Error = MPLEX_BASE + 3,       // Error message
    Warning = MPLEX_BASE + 4,     // Warning
    ErrorSocket = MPLEX_BASE + 5, // Socket error
    Log = MPLEX_BASE + 6,         // Log message
    Client = MPLEX_BASE + 7,      // Client message
    Redo = MPLEX_BASE + 9,        // Redo request
    FList = MPLEX_BASE + 20,      // File list data
    FName = MPLEX_BASE + 21,      // Filename
    IoError = MPLEX_BASE + 22,    // I/O error
    Success = MPLEX_BASE + 100,   // Success
    NoSend = MPLEX_BASE + 101,    // Nothing to send
}

impl MessageTag {
    fn from_u8(tag: u8) -> Option<Self> {
        match tag {
            7 => Some(Self::Data),
            8 => Some(Self::ErrorXfer),
            9 => Some(Self::Info),
            10 => Some(Self::Error),
            11 => Some(Self::Warning),
            12 => Some(Self::ErrorSocket),
            13 => Some(Self::Log),
            14 => Some(Self::Client),
            16 => Some(Self::Redo),
            27 => Some(Self::FList),
            28 => Some(Self::FName),
            29 => Some(Self::IoError),
            107 => Some(Self::Success),
            108 => Some(Self::NoSend),
            _ => None,
        }
    }
}

/// rsync multiplexed message
#[derive(Debug)]
pub struct MultiplexMessage {
    pub tag: MessageTag,
    pub data: Vec<u8>,
}

/// Read a multiplexed message from rsync protocol stream
pub async fn read_mplex_message<T: Transport>(transport: &mut T) -> Result<MultiplexMessage> {
    // Read tag byte
    let mut tag_buf = [0u8; 1];
    transport::read_exact(transport, &mut tag_buf).await?;
    let tag_byte = tag_buf[0];

    // Parse tag
    let tag = MessageTag::from_u8(tag_byte)
        .ok_or_else(|| anyhow::anyhow!("Unknown rsync message tag: {}", tag_byte))?;

    // Read length (3 bytes, little-endian, but only lower 24 bits used)
    let mut len_buf = [0u8; 3];
    transport::read_exact(transport, &mut len_buf).await?;
    let length = u32::from_le_bytes([len_buf[0], len_buf[1], len_buf[2], 0]) as usize;

    // Read data
    let mut data = vec![0u8; length];
    if length > 0 {
        transport::read_exact(transport, &mut data).await?;
    }

    debug!("Read rsync message: tag={:?}, length={}", tag, length);

    Ok(MultiplexMessage { tag, data })
}

/// Write a multiplexed message to rsync protocol stream
pub async fn write_mplex_message<T: Transport>(
    transport: &mut T,
    tag: MessageTag,
    data: &[u8],
) -> Result<()> {
    let length = data.len();

    if length > 0xFFFFFF {
        anyhow::bail!("Message too large for rsync protocol: {} bytes", length);
    }

    // Write tag
    transport::write_all(transport, &[tag as u8]).await?;

    // Write length (3 bytes, little-endian)
    let len_bytes = (length as u32).to_le_bytes();
    transport::write_all(transport, &len_bytes[0..3]).await?;

    // Write data
    if length > 0 {
        transport::write_all(transport, data).await?;
    }

    debug!("Wrote rsync message: tag={:?}, length={}", tag, length);

    Ok(())
}

/// Read a data message, handling multiplexed protocol
pub async fn read_data<T: Transport>(transport: &mut T, buffer: &mut [u8]) -> Result<usize> {
    loop {
        let msg = read_mplex_message(transport).await?;

        match msg.tag {
            MessageTag::Data => {
                // This is the data we want
                let copy_len = msg.data.len().min(buffer.len());
                buffer[..copy_len].copy_from_slice(&msg.data[..copy_len]);
                return Ok(copy_len);
            }
            MessageTag::Info | MessageTag::Log => {
                // Log messages, print to stderr
                if let Ok(text) = String::from_utf8(msg.data) {
                    debug!("rsync: {}", text.trim());
                }
            }
            MessageTag::Error | MessageTag::ErrorXfer => {
                // Error message
                let error_msg = String::from_utf8_lossy(&msg.data);
                anyhow::bail!("rsync error: {}", error_msg);
            }
            MessageTag::Warning => {
                let warn_msg = String::from_utf8_lossy(&msg.data);
                warn!("rsync warning: {}", warn_msg);
            }
            _ => {
                debug!("Unexpected rsync message tag: {:?}", msg.tag);
            }
        }
    }
}

/// Write data with multiplexed protocol
pub async fn write_data<T: Transport>(transport: &mut T, data: &[u8]) -> Result<()> {
    write_mplex_message(transport, MessageTag::Data, data).await
}

/// Multiplexed reader with buffering
pub struct MultiplexReader<T: Transport> {
    transport: T,
    buffer: Vec<u8>,
    buffer_pos: usize,
}

impl<T: Transport> MultiplexReader<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            buffer: Vec::new(),
            buffer_pos: 0,
        }
    }

    /// Read next multiplexed message
    pub async fn read_message(&mut self) -> Result<MultiplexMessage> {
        read_mplex_message(&mut self.transport).await
    }

    /// Read data message, skipping INFO/LOG messages
    pub async fn read_data(&mut self, buf: &mut [u8]) -> Result<usize> {
        loop {
            let msg = self.read_message().await?;

            match msg.tag {
                MessageTag::Data => {
                    let copy_len = msg.data.len().min(buf.len());
                    buf[..copy_len].copy_from_slice(&msg.data[..copy_len]);
                    return Ok(copy_len);
                }
                MessageTag::Info | MessageTag::Log => {
                    if let Ok(text) = String::from_utf8(msg.data) {
                        debug!("rsync: {}", text.trim());
                    }
                }
                MessageTag::Error | MessageTag::ErrorXfer => {
                    let error_msg = String::from_utf8_lossy(&msg.data);
                    anyhow::bail!("rsync error: {}", error_msg);
                }
                MessageTag::Warning => {
                    let warn_msg = String::from_utf8_lossy(&msg.data);
                    warn!("rsync warning: {}", warn_msg);
                }
                _ => {
                    debug!("Ignoring rsync message tag: {:?}", msg.tag);
                }
            }
        }
    }

    /// Expect a specific message tag
    pub async fn expect_tag(&mut self, expected: MessageTag) -> Result<Vec<u8>> {
        let msg = self.read_message().await?;

        if msg.tag == expected {
            Ok(msg.data)
        } else {
            anyhow::bail!("Expected tag {:?}, got {:?}", expected, msg.tag);
        }
    }

    /// Read multiple messages until end marker
    pub async fn read_until_empty(&mut self, expected_tag: MessageTag) -> Result<Vec<Vec<u8>>> {
        let mut messages = Vec::new();

        loop {
            let msg = self.read_message().await?;

            if msg.tag != expected_tag {
                anyhow::bail!("Expected tag {:?}, got {:?}", expected_tag, msg.tag);
            }

            if msg.data.is_empty() {
                // Empty message = end marker
                break;
            }

            messages.push(msg.data);
        }

        Ok(messages)
    }
}

/// Multiplexed writer
pub struct MultiplexWriter<T: Transport> {
    transport: T,
}

impl<T: Transport> MultiplexWriter<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Write a tagged message
    pub async fn write_message(&mut self, tag: MessageTag, data: &[u8]) -> Result<()> {
        write_mplex_message(&mut self.transport, tag, data).await
    }

    /// Write data message
    pub async fn write_data(&mut self, data: &[u8]) -> Result<()> {
        self.write_message(MessageTag::Data, data).await
    }

    /// Write info message
    pub async fn write_info(&mut self, message: &str) -> Result<()> {
        self.write_message(MessageTag::Info, message.as_bytes())
            .await
    }

    /// Write error message
    pub async fn write_error(&mut self, message: &str) -> Result<()> {
        self.write_message(MessageTag::Error, message.as_bytes())
            .await
    }

    /// Flush underlying transport
    pub async fn flush(&mut self) -> Result<()> {
        self.transport.flush().await
    }
}

// ============================================================================
// File List Encoding/Decoding (rsync format)
// ============================================================================

/// rsync file list flags (from flist.c)
mod file_flags {
    pub const XMIT_TOP_DIR: u8 = 0x01; // Top-level directory
    pub const XMIT_SAME_MODE: u8 = 0x02; // Mode unchanged
    pub const XMIT_EXTENDED_FLAGS: u8 = 0x04; // Extended flags follow
    pub const XMIT_SAME_UID: u8 = 0x10; // UID unchanged
    pub const XMIT_SAME_GID: u8 = 0x20; // GID unchanged
    pub const XMIT_SAME_NAME: u8 = 0x40; // Name matches previous (hardlink)
    pub const XMIT_LONG_NAME: u8 = 0x80; // Name > 255 bytes
}

/// Encode file list in rsync wire format (simplified - no delta encoding yet)
///
/// This sends each file as MSG_FLIST tagged message with varint-encoded fields.
/// Simplified version: no mtime deltas, no directory grouping.
pub async fn encode_file_list_rsync<T: Transport>(
    writer: &mut MultiplexWriter<T>,
    files: &[FileEntry],
) -> Result<()> {
    debug!("Encoding {} files in rsync format", files.len());

    for file in files {
        let mut entry = Vec::new();

        // Flags byte
        let mut flags = 0u8;
        if file.path.len() > 255 {
            flags |= file_flags::XMIT_LONG_NAME;
        }
        // For now, no delta encoding, so no SAME_MODE/SAME_UID/SAME_GID flags

        entry.push(flags);

        // Path (varint length + bytes)
        encode_varint_into(file.path.len() as u64, &mut entry);
        entry.extend(file.path.as_bytes());

        // File size (varint)
        encode_varint_into(file.size, &mut entry);

        // Mtime (varint, absolute - no delta encoding yet)
        // rsync uses unsigned for mtime, we need to convert
        let mtime_unsigned = if file.mtime < 0 {
            0u64 // Clamp negative times to 0
        } else {
            file.mtime as u64
        };
        encode_varint_into(mtime_unsigned, &mut entry);

        // Mode (varint)
        encode_varint_into(file.mode as u64, &mut entry);

        // uid/gid (varint)
        encode_varint_into(file.uid as u64, &mut entry);
        encode_varint_into(file.gid as u64, &mut entry);

        // Symlink target if applicable
        if file.is_symlink {
            if let Some(ref target) = file.symlink_target {
                encode_varint_into(target.len() as u64, &mut entry);
                entry.extend(target.as_bytes());
            }
        }

        // Send as MSG_FLIST tagged message
        writer.write_message(MessageTag::FList, &entry).await?;

        debug!("Encoded file: {} ({} bytes)", file.path, entry.len());
    }

    // End-of-list marker (empty MSG_FLIST)
    writer.write_message(MessageTag::FList, &[]).await?;
    debug!("Sent end-of-list marker");

    Ok(())
}

/// Decode file list from rsync wire format
///
/// Reads MSG_FLIST tagged messages until empty message (end marker).
pub async fn decode_file_list_rsync<T: Transport>(
    reader: &mut MultiplexReader<T>,
) -> Result<Vec<FileEntry>> {
    debug!("Decoding file list in rsync format");

    let mut files = Vec::new();

    loop {
        // Read next file list message
        let msg = reader.read_message().await?;

        if msg.tag != MessageTag::FList {
            anyhow::bail!("Expected MSG_FLIST, got {:?}", msg.tag);
        }

        if msg.data.is_empty() {
            // End of list marker
            debug!("Received end-of-list marker");
            break;
        }

        // Decode file entry from message data
        let file = decode_file_entry(&msg.data)?;
        debug!("Decoded file: {} ({} bytes)", file.path, file.size);
        files.push(file);
    }

    debug!("Decoded {} files total", files.len());
    Ok(files)
}

/// Decode a single file entry from bytes
fn decode_file_entry(data: &[u8]) -> Result<FileEntry> {
    let mut cursor = Cursor::new(data);

    // Flags byte
    let mut flags_buf = [0u8; 1];
    std::io::Read::read_exact(&mut cursor, &mut flags_buf)?;
    let flags = flags_buf[0];

    // Path length and path
    let path_len = decode_varint_sync(&mut cursor)? as usize;
    let mut path_buf = vec![0u8; path_len];
    std::io::Read::read_exact(&mut cursor, &mut path_buf)?;
    let path = String::from_utf8(path_buf)?;

    // File size
    let size = decode_varint_sync(&mut cursor)?;

    // Mtime
    let mtime = decode_varint_sync(&mut cursor)? as i64;

    // Mode
    let mode = decode_varint_sync(&mut cursor)? as u32;

    // uid/gid
    let uid = decode_varint_sync(&mut cursor)? as u32;
    let gid = decode_varint_sync(&mut cursor)? as u32;

    // Check if symlink (we need to infer from mode bits)
    let is_symlink = (mode & 0o170000) == 0o120000; // S_IFLNK

    // Symlink target if applicable
    let symlink_target = if is_symlink {
        let target_len = decode_varint_sync(&mut cursor)? as usize;
        let mut target_buf = vec![0u8; target_len];
        std::io::Read::read_exact(&mut cursor, &mut target_buf)?;
        Some(String::from_utf8(target_buf)?)
    } else {
        None
    };

    Ok(FileEntry {
        path,
        size,
        mtime,
        mode,
        uid,
        gid,
        is_symlink,
        symlink_target,
    })
}

/// Decode varint from synchronous reader (for use with Cursor)
fn decode_varint_sync(reader: &mut Cursor<&[u8]>) -> Result<u64> {
    use std::io::Read;

    let mut result = 0u64;
    let mut shift = 0;

    loop {
        let mut byte_buf = [0u8; 1];
        reader.read_exact(&mut byte_buf)?;
        let byte = byte_buf[0];

        result |= ((byte & 0x7F) as u64) << shift;

        if (byte & 0x80) == 0 {
            break;
        }

        shift += 7;

        if shift > 63 {
            anyhow::bail!("Varint overflow");
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_tag_parsing() {
        assert_eq!(MessageTag::from_u8(7), Some(MessageTag::Data));
        assert_eq!(MessageTag::from_u8(9), Some(MessageTag::Info));
        assert_eq!(MessageTag::from_u8(10), Some(MessageTag::Error));
        assert_eq!(MessageTag::from_u8(255), None);
    }

    #[test]
    fn test_length_encoding() {
        // rsync uses 3-byte length encoding
        let length = 12345u32;
        let bytes = length.to_le_bytes();
        let encoded = [bytes[0], bytes[1], bytes[2]];

        let decoded = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], 0]);
        assert_eq!(decoded, length);
    }

    #[test]
    fn test_file_entry_roundtrip() {
        // Create a test file entry
        let original = FileEntry {
            path: "test/file.txt".to_string(),
            size: 12345,
            mtime: 1696800000,
            mode: 0o100644,
            uid: 1000,
            gid: 1000,
            is_symlink: false,
            symlink_target: None,
        };

        // Encode it manually (simulating what encode_file_list_rsync does)
        let mut entry = Vec::new();
        entry.push(0u8); // flags
        encode_varint_into(original.path.len() as u64, &mut entry);
        entry.extend(original.path.as_bytes());
        encode_varint_into(original.size, &mut entry);
        encode_varint_into(original.mtime as u64, &mut entry);
        encode_varint_into(original.mode as u64, &mut entry);
        encode_varint_into(original.uid as u64, &mut entry);
        encode_varint_into(original.gid as u64, &mut entry);

        // Decode it
        let decoded = decode_file_entry(&entry).expect("Should decode");

        assert_eq!(decoded.path, original.path);
        assert_eq!(decoded.size, original.size);
        assert_eq!(decoded.mtime, original.mtime);
        assert_eq!(decoded.mode, original.mode);
        assert_eq!(decoded.uid, original.uid);
        assert_eq!(decoded.gid, original.gid);
    }

    #[test]
    fn test_symlink_entry() {
        let symlink = FileEntry {
            path: "link".to_string(),
            size: 0,
            mtime: 1696800000,
            mode: 0o120777, // S_IFLNK | 0777
            uid: 1000,
            gid: 1000,
            is_symlink: true,
            symlink_target: Some("target/path".to_string()),
        };

        // Encode manually
        let mut entry = Vec::new();
        entry.push(0u8);
        encode_varint_into(symlink.path.len() as u64, &mut entry);
        entry.extend(symlink.path.as_bytes());
        encode_varint_into(symlink.size, &mut entry);
        encode_varint_into(symlink.mtime as u64, &mut entry);
        encode_varint_into(symlink.mode as u64, &mut entry);
        encode_varint_into(symlink.uid as u64, &mut entry);
        encode_varint_into(symlink.gid as u64, &mut entry);
        let target = symlink.symlink_target.as_ref().unwrap();
        encode_varint_into(target.len() as u64, &mut entry);
        entry.extend(target.as_bytes());

        // Decode
        let decoded = decode_file_entry(&entry).expect("Should decode");

        assert_eq!(decoded.is_symlink, true);
        assert_eq!(decoded.symlink_target.as_ref().unwrap(), "target/path");
    }
}

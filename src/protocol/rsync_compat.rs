//! rsync Wire Protocol Compatibility Layer
//!
//! This module implements the actual rsync wire protocol for interoperability
//! with rsync processes. rsync uses a multiplexed I/O protocol with message tags.

use crate::protocol::transport::{self, Transport};
use anyhow::Result;
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
}

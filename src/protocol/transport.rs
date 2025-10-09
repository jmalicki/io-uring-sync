//! Generic transport abstraction for rsync protocol
//!
//! The rsync wire protocol is transport-agnostic - it works over any
//! bidirectional byte stream (pipes, TCP, SSH, QUIC, etc.)

use anyhow::Result;
use async_trait::async_trait;

/// Generic transport for rsync protocol
///
/// Represents a bidirectional byte stream that can carry rsync protocol messages.
/// Implementations include: pipes (testing), SSH (production), QUIC (future).
#[async_trait]
pub trait Transport: Send {
    /// Read bytes from transport
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Write bytes to transport
    async fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Flush any buffered writes
    async fn flush(&mut self) -> Result<()>;

    /// Get transport name (for debugging)
    fn name(&self) -> &str {
        "unknown"
    }

    /// Check if transport supports parallel streams
    fn supports_multiplexing(&self) -> bool {
        false
    }
}

/// Helper to read exact number of bytes
pub async fn read_exact<T: Transport>(transport: &mut T, buf: &mut [u8]) -> Result<()> {
    let mut offset = 0;
    while offset < buf.len() {
        let n = transport.read(&mut buf[offset..]).await?;
        if n == 0 {
            anyhow::bail!("Unexpected EOF while reading {} bytes", buf.len());
        }
        offset += n;
    }
    Ok(())
}

/// Helper to write all bytes
pub async fn write_all<T: Transport>(transport: &mut T, buf: &[u8]) -> Result<()> {
    let mut offset = 0;
    while offset < buf.len() {
        let n = transport.write(&buf[offset..]).await?;
        offset += n;
    }
    transport.flush().await?;
    Ok(())
}

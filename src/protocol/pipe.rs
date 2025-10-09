//! Pipe-based transport for rsync protocol testing
//!
//! Enables testing rsync wire protocol via pipes (stdin/stdout or in-memory)
//! without requiring SSH or network infrastructure.

use super::transport::Transport;
use anyhow::Result;
use async_trait::async_trait;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Pipe-based transport for rsync protocol
pub struct PipeTransport {
    reader: Box<dyn tokio::io::AsyncRead + Unpin + Send>,
    writer: Box<dyn tokio::io::AsyncWrite + Unpin + Send>,
    name: String,
}

impl PipeTransport {
    /// Create from stdin/stdout (for --pipe mode)
    ///
    /// # Safety
    ///
    /// Assumes FDs 0 and 1 are valid and will not be closed elsewhere
    pub fn from_stdio() -> Result<Self> {
        use std::os::unix::io::FromRawFd;

        // Duplicate FDs so we don't close stdin/stdout
        let stdin_fd = unsafe { libc::dup(0) };
        let stdout_fd = unsafe { libc::dup(1) };

        if stdin_fd < 0 || stdout_fd < 0 {
            anyhow::bail!("Failed to duplicate stdin/stdout");
        }

        let stdin = unsafe { std::fs::File::from_raw_fd(stdin_fd) };
        let stdout = unsafe { std::fs::File::from_raw_fd(stdout_fd) };

        Ok(Self {
            reader: Box::new(tokio::fs::File::from_std(stdin)),
            writer: Box::new(tokio::fs::File::from_std(stdout)),
            name: "stdio".to_string(),
        })
    }

    /// Create from specific file descriptors
    ///
    /// # Safety
    ///
    /// Caller must ensure FDs are valid and not closed elsewhere
    pub unsafe fn from_fds(read_fd: RawFd, write_fd: RawFd, name: String) -> Result<Self> {
        let reader_file = std::fs::File::from_raw_fd(read_fd);
        let writer_file = std::fs::File::from_raw_fd(write_fd);

        Ok(Self {
            reader: Box::new(tokio::fs::File::from_std(reader_file)),
            writer: Box::new(tokio::fs::File::from_std(writer_file)),
            name,
        })
    }

    /// Create a pair of in-memory pipes for testing
    ///
    /// Returns (transport_a, transport_b) where:
    /// - transport_a writes go to transport_b reads
    /// - transport_b writes go to transport_a reads
    pub fn create_memory_pipe_pair() -> (Self, Self) {
        // Create two bidirectional channels
        let (a_read, b_write) = tokio::io::duplex(64 * 1024);
        let (b_read, a_write) = tokio::io::duplex(64 * 1024);

        let transport_a = Self {
            reader: Box::new(a_read),
            writer: Box::new(a_write),
            name: "memory_pipe_a".to_string(),
        };

        let transport_b = Self {
            reader: Box::new(b_read),
            writer: Box::new(b_write),
            name: "memory_pipe_b".to_string(),
        };

        (transport_a, transport_b)
    }
}

#[async_trait]
impl Transport for PipeTransport {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.reader.read(buf).await?;
        Ok(n)
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let n = self.writer.write(buf).await?;
        Ok(n)
    }

    async fn flush(&mut self) -> Result<()> {
        self.writer.flush().await?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_pipe_communication() {
        let (mut pipe_a, mut pipe_b) = PipeTransport::create_memory_pipe_pair();

        // Write from A, read from B
        let send_task = tokio::spawn(async move {
            pipe_a.write(b"Hello").await.unwrap();
            pipe_a.flush().await.unwrap();
            pipe_a
        });

        let recv_task = tokio::spawn(async move {
            let mut buf = [0u8; 5];
            pipe_b.read(&mut buf).await.unwrap();
            assert_eq!(&buf, b"Hello");
            pipe_b
        });

        let (pipe_a, pipe_b) = tokio::join!(send_task, recv_task);
        let mut pipe_a = pipe_a.unwrap();
        let mut pipe_b = pipe_b.unwrap();

        // Write from B, read from A
        pipe_b.write(b"World").await.unwrap();
        pipe_b.flush().await.unwrap();

        let mut buf = [0u8; 5];
        pipe_a.read(&mut buf).await.unwrap();
        assert_eq!(&buf, b"World");
    }
}

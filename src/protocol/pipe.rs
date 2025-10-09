//! Pipe-based transport for rsync protocol testing
//!
//! Enables testing rsync wire protocol via pipes (stdin/stdout or in-memory)
//! without requiring SSH or network infrastructure.
//!
//! Note: Uses blocking I/O for simplicity since pipes are fast enough
//! and we want to avoid runtime conflicts (compio vs tokio).

use super::transport::Transport;
use anyhow::Result;
use async_trait::async_trait;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

/// Pipe-based transport for rsync protocol
pub struct PipeTransport {
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
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
            reader: Box::new(stdin),
            writer: Box::new(stdout),
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
            reader: Box::new(reader_file),
            writer: Box::new(writer_file),
            name,
        })
    }
}

#[async_trait]
impl Transport for PipeTransport {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Blocking I/O is fine for pipes (they're fast)
        let n = self.reader.read(buf)?;
        Ok(n)
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        // Blocking I/O is fine for pipes (they're fast)
        let n = self.writer.write(buf)?;
        Ok(n)
    }

    async fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

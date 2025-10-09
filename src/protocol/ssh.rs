//! SSH connection management for remote sync
//!
//! Handles SSH connections using the configured remote shell (typically ssh)

use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// SSH connection to remote host
pub struct SshConnection {
    process: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    host: String,
    user: String,
}

impl SshConnection {
    /// Connect to remote host via SSH
    pub async fn connect(host: &str, user: &str, remote_shell: &str) -> Result<Self> {
        // Build SSH command
        let mut cmd = Command::new(remote_shell);
        cmd.arg(format!("{}@{}", user, host))
            .arg("--") // Separator for SSH args vs remote command
            .arg("arsync")
            .arg("--server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        // Spawn SSH process
        let mut process = cmd.spawn()?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;

        Ok(Self {
            process,
            stdin,
            stdout,
            host: host.to_string(),
            user: user.to_string(),
        })
    }

    /// Start remote server (send initial protocol negotiation)
    pub async fn start_server(&mut self, _path: &Path) -> Result<()> {
        // For now, just verify connection is alive
        // TODO: Implement protocol negotiation
        Ok(())
    }

    /// Send bytes over SSH channel
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.stdin.write_all(data).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Receive bytes from SSH channel
    pub async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let n = self.stdout.read(buffer).await?;
        Ok(n)
    }

    /// Close the SSH connection
    pub async fn close(mut self) -> Result<()> {
        drop(self.stdin);
        drop(self.stdout);
        self.process.wait().await?;
        Ok(())
    }
}

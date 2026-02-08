//! SFTP client implementation using `russh` and `russh-sftp`.
//!
//! Provides [`RemoteSession`] for connecting to remote servers via SSH
//! and listing directories over SFTP.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use russh::client;
use russh::keys::key::PublicKey;
use russh_sftp::client::SftpSession;

use crate::fs::entry::FileEntry;

/// Configuration for an SFTP connection.
#[derive(Debug, Clone)]
pub struct SftpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl SftpConfig {
    /// Returns a display label like `"user@host"` or `"user@host:port"`.
    pub fn display_label(&self) -> String {
        if self.port == 22 {
            format!("{}@{}", self.username, self.host)
        } else {
            format!("{}@{}:{}", self.username, self.host, self.port)
        }
    }
}

/// Errors that can occur during SFTP operations.
#[derive(Debug, thiserror::Error)]
pub enum SftpError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("SFTP error: {0}")]
    Sftp(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("session disconnected")]
    Disconnected,
}

/// Internal SSH client handler. Accepts all server keys for now (MVP).
struct SshHandler;

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    /// Accept all host keys in MVP. Phase 2 will add known_hosts verification.
    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// An active SSH/SFTP session to a remote server.
pub struct RemoteSession {
    config: SftpConfig,
    sftp: SftpSession,
    _handle: client::Handle<SshHandler>,
}

impl RemoteSession {
    /// Connects to a remote server via SSH and opens an SFTP session.
    pub async fn connect(config: SftpConfig) -> Result<Self, SftpError> {
        let ssh_config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(30)),
            ..Default::default()
        };

        let mut handle = client::connect(
            Arc::new(ssh_config),
            (config.host.as_str(), config.port),
            SshHandler,
        )
        .await
        .map_err(|e| SftpError::ConnectionFailed(e.to_string()))?;

        let auth_ok = handle
            .authenticate_password(&config.username, &config.password)
            .await
            .map_err(|e| SftpError::AuthFailed(e.to_string()))?;

        if !auth_ok {
            return Err(SftpError::AuthFailed(
                "invalid username or password".to_string(),
            ));
        }

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| SftpError::ConnectionFailed(e.to_string()))?;

        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| SftpError::Sftp(e.to_string()))?;

        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| SftpError::Sftp(e.to_string()))?;

        Ok(Self {
            config,
            sftp,
            _handle: handle,
        })
    }

    /// Lists the contents of a remote directory, returning `FileEntry` values.
    ///
    /// `.` and `..` entries are automatically filtered out.
    pub async fn list_directory(&self, remote_path: &str) -> Result<Vec<FileEntry>, SftpError> {
        let dir_entries = self
            .sftp
            .read_dir(remote_path)
            .await
            .map_err(|e| SftpError::Sftp(e.to_string()))?;

        let entries: Vec<FileEntry> = dir_entries
            .map(|de| {
                let name = de.file_name();
                let attrs = de.metadata();
                let is_dir = attrs.is_dir();
                let is_symlink = attrs.file_type().is_symlink();
                let size = attrs.len();
                let modified = attrs.modified().ok();
                let is_hidden = name.starts_with('.');
                let path = PathBuf::from(format!("{}/{}", remote_path.trim_end_matches('/'), name));

                FileEntry::from_remote(path, name, size, modified, is_dir, is_hidden, is_symlink)
            })
            .collect();

        Ok(entries)
    }

    /// Returns the connection configuration.
    pub fn config(&self) -> &SftpConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_label_default_port() {
        let config = SftpConfig {
            host: "example.com".to_string(),
            port: 22,
            username: "kim".to_string(),
            password: "secret".to_string(),
        };
        assert_eq!(config.display_label(), "kim@example.com");
    }

    #[test]
    fn display_label_custom_port() {
        let config = SftpConfig {
            host: "example.com".to_string(),
            port: 2222,
            username: "kim".to_string(),
            password: "secret".to_string(),
        };
        assert_eq!(config.display_label(), "kim@example.com:2222");
    }

    #[test]
    fn sftp_error_display() {
        let err = SftpError::ConnectionFailed("timeout".to_string());
        assert_eq!(err.to_string(), "connection failed: timeout");

        let err = SftpError::AuthFailed("bad password".to_string());
        assert_eq!(err.to_string(), "authentication failed: bad password");

        let err = SftpError::Disconnected;
        assert_eq!(err.to_string(), "session disconnected");
    }
}

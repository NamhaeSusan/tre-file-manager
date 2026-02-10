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
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

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

    #[error("host key verification failed for {host}:{port}\nExpected: {expected}\nReceived: {received}\n\nWARNING: Host key has changed! This could indicate a man-in-the-middle attack.\nIf you trust this new key, remove the old entry from ~/.config/trefm/known_hosts")]
    HostKeyMismatch {
        host: String,
        port: u16,
        expected: String,
        received: String,
    },

    #[error("host key verification error: {0}")]
    HostKeyError(String),
}

/// Internal SSH client handler with TOFU (Trust On First Use) host key verification.
struct SshHandler {
    host: String,
    port: u16,
    #[cfg(test)]
    test_known_hosts_path: Option<PathBuf>,
}

impl SshHandler {
    fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            #[cfg(test)]
            test_known_hosts_path: None,
        }
    }

    #[cfg(test)]
    fn new_with_known_hosts(host: String, port: u16, known_hosts_path: PathBuf) -> Self {
        Self {
            host,
            port,
            test_known_hosts_path: Some(known_hosts_path),
        }
    }

    /// Returns the path to the known_hosts file: `~/.config/trefm/known_hosts`
    fn known_hosts_path(&self) -> Result<PathBuf, std::io::Error> {
        #[cfg(test)]
        if let Some(ref path) = self.test_known_hosts_path {
            return Ok(path.clone());
        }

        let home = std::env::var("HOME").map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "HOME environment variable not set",
            )
        })?;
        let config_dir = PathBuf::from(home).join(".config").join("trefm");
        Ok(config_dir.join("known_hosts"))
    }

    /// Computes the SHA-256 fingerprint of a public key.
    fn compute_fingerprint(public_key: &PublicKey) -> String {
        // Use the Debug format to get a stable byte representation of the key
        let key_repr = format!("{:?}", public_key);
        let mut hasher = Sha256::new();
        hasher.update(key_repr.as_bytes());
        let hash = hasher.finalize();
        let hex_str: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        format!("SHA256:{}", hex_str)
    }

    /// Reads the known_hosts file and returns the stored fingerprint for this host:port.
    async fn read_known_host(&self) -> Result<Option<String>, std::io::Error> {
        let path = self.known_hosts_path()?;
        if !path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let host_key = format!("{}:{}", self.host, self.port);

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((stored_host, fingerprint)) = line.split_once(' ') {
                if stored_host == host_key {
                    return Ok(Some(fingerprint.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Appends a new host key entry to the known_hosts file.
    async fn append_known_host(&self, fingerprint: &str) -> Result<(), std::io::Error> {
        let path = self.known_hosts_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let host_key = format!("{}:{}", self.host, self.port);
        let entry = format!("{} {}\n", host_key, fingerprint);

        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?
            .write_all(entry.as_bytes())
            .await?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&path).await?.permissions();
            perms.set_mode(0o600);
            tokio::fs::set_permissions(&path, perms).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    /// Implements TOFU (Trust On First Use) host key verification.
    ///
    /// - On first connection to unknown host: accept the key and save it
    /// - On subsequent connections: verify the key matches the stored one
    /// - If key has changed: reject with HostKeyMismatch error
    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let received_fingerprint = Self::compute_fingerprint(server_public_key);

        match self.read_known_host().await {
            Ok(Some(expected_fingerprint)) => {
                // Host is known - verify the key matches
                if expected_fingerprint == received_fingerprint {
                    tracing::debug!("Host key verified for {}:{}", self.host, self.port);
                    Ok(true)
                } else {
                    tracing::error!(
                        "Host key mismatch for {}:{} - expected: {}, received: {}",
                        self.host,
                        self.port,
                        expected_fingerprint,
                        received_fingerprint
                    );
                    Err(russh::Error::from(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!(
                            "Host key verification failed for {}:{}\nExpected: {}\nReceived: {}",
                            self.host, self.port, expected_fingerprint, received_fingerprint
                        ),
                    )))
                }
            }
            Ok(None) => {
                // First connection - trust and save the key
                tracing::info!(
                    "First connection to {}:{} - accepting and storing host key: {}",
                    self.host,
                    self.port,
                    received_fingerprint
                );

                if let Err(e) = self.append_known_host(&received_fingerprint).await {
                    tracing::warn!(
                        "Failed to save host key for {}:{}: {}",
                        self.host,
                        self.port,
                        e
                    );
                    // Still allow the connection even if we can't save the key
                }

                Ok(true)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to read known_hosts for {}:{}: {} - allowing connection",
                    self.host,
                    self.port,
                    e
                );
                // If we can't read the known_hosts file, allow the connection
                // but don't save the key
                Ok(true)
            }
        }
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

        let handler = SshHandler::new(config.host.clone(), config.port);

        let mut handle = client::connect(
            Arc::new(ssh_config),
            (config.host.as_str(), config.port),
            handler,
        )
        .await
        .map_err(|e| {
            // Check if this is a host key mismatch error
            let err_msg = e.to_string();
            if err_msg.contains("Host key verification failed") {
                // Parse the error message to extract fingerprints if possible
                if let (Some(exp_start), Some(rec_start)) =
                    (err_msg.find("Expected: "), err_msg.find("Received: "))
                {
                    let exp_start = exp_start + 10;
                    let exp_end = err_msg[exp_start..]
                        .find('\n')
                        .unwrap_or(err_msg.len() - exp_start);
                    let expected = err_msg[exp_start..exp_start + exp_end].trim().to_string();

                    let rec_start = rec_start + 10;
                    let rec_end = err_msg[rec_start..]
                        .find('\n')
                        .unwrap_or(err_msg.len() - rec_start);
                    let received = err_msg[rec_start..rec_start + rec_end].trim().to_string();

                    return SftpError::HostKeyMismatch {
                        host: config.host.clone(),
                        port: config.port,
                        expected,
                        received,
                    };
                }
                SftpError::HostKeyError(err_msg)
            } else {
                SftpError::ConnectionFailed(e.to_string())
            }
        })?;

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
    use tempfile::TempDir;

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

        let err = SftpError::HostKeyMismatch {
            host: "example.com".to_string(),
            port: 22,
            expected: "SHA256:old_key".to_string(),
            received: "SHA256:new_key".to_string(),
        };
        let err_str = err.to_string();
        assert!(err_str.contains("host key verification failed"));
        assert!(err_str.contains("SHA256:old_key"));
        assert!(err_str.contains("SHA256:new_key"));
    }

    #[tokio::test]
    async fn test_known_hosts_first_connection() {
        let temp_dir = TempDir::new().unwrap();
        let known_hosts_path = temp_dir.path().join("known_hosts");

        // Create handler with custom known_hosts path
        let handler = SshHandler::new_with_known_hosts(
            "test.example.com".to_string(),
            22,
            known_hosts_path.clone(),
        );

        // First connection - should return None (unknown host)
        let stored = handler.read_known_host().await.unwrap();
        assert_eq!(stored, None);

        // Simulate saving a host key
        let fingerprint = "SHA256:test_fingerprint_abc123";
        handler.append_known_host(fingerprint).await.unwrap();

        // Verify it was saved
        let stored = handler.read_known_host().await.unwrap();
        assert_eq!(stored, Some(fingerprint.to_string()));

        // Verify file format
        assert!(known_hosts_path.exists());

        let content = std::fs::read_to_string(&known_hosts_path).unwrap();
        assert!(content.contains("test.example.com:22 SHA256:test_fingerprint_abc123"));

        // Verify permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&known_hosts_path).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o600);
        }
    }

    #[tokio::test]
    async fn test_known_hosts_multiple_entries() {
        let temp_dir = TempDir::new().unwrap();
        let known_hosts_path = temp_dir.path().join("known_hosts");

        let handler1 = SshHandler::new_with_known_hosts(
            "host1.example.com".to_string(),
            22,
            known_hosts_path.clone(),
        );
        let handler2 = SshHandler::new_with_known_hosts(
            "host2.example.com".to_string(),
            2222,
            known_hosts_path.clone(),
        );

        // Save two different hosts
        handler1
            .append_known_host("SHA256:fingerprint1")
            .await
            .unwrap();
        handler2
            .append_known_host("SHA256:fingerprint2")
            .await
            .unwrap();

        // Verify each can read its own entry
        let stored1 = handler1.read_known_host().await.unwrap();
        assert_eq!(stored1, Some("SHA256:fingerprint1".to_string()));

        let stored2 = handler2.read_known_host().await.unwrap();
        assert_eq!(stored2, Some("SHA256:fingerprint2".to_string()));
    }

    #[tokio::test]
    async fn test_known_hosts_custom_port() {
        let temp_dir = TempDir::new().unwrap();
        let known_hosts_path = temp_dir.path().join("known_hosts");

        let handler_default = SshHandler::new_with_known_hosts(
            "example.com".to_string(),
            22,
            known_hosts_path.clone(),
        );
        let handler_custom = SshHandler::new_with_known_hosts(
            "example.com".to_string(),
            2222,
            known_hosts_path.clone(),
        );

        // Save keys for same host but different ports
        handler_default
            .append_known_host("SHA256:default_port")
            .await
            .unwrap();
        handler_custom
            .append_known_host("SHA256:custom_port")
            .await
            .unwrap();

        // Verify they are treated as different hosts
        let stored_default = handler_default.read_known_host().await.unwrap();
        assert_eq!(stored_default, Some("SHA256:default_port".to_string()));

        let stored_custom = handler_custom.read_known_host().await.unwrap();
        assert_eq!(stored_custom, Some("SHA256:custom_port".to_string()));
    }

    #[test]
    fn test_compute_fingerprint_consistency() {
        // This test just verifies that fingerprint computation is deterministic
        // We can't test actual SSH keys without a real connection, but we can
        // verify the format is correct
        use russh_keys::key::KeyPair;

        // Generate a test keypair
        let keypair = KeyPair::generate_ed25519();

        // clone_public_key returns Result
        if let Ok(public_key) = keypair.clone_public_key() {
            let fp1 = SshHandler::compute_fingerprint(&public_key);
            let fp2 = SshHandler::compute_fingerprint(&public_key);

            // Should be deterministic
            assert_eq!(fp1, fp2);
            // Should have correct format
            assert!(fp1.starts_with("SHA256:"));
        } else {
            // If we can't extract the public key, skip the test
            eprintln!("Skipping test - cannot extract public key");
        }
    }
}

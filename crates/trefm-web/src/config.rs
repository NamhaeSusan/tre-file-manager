use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct UserConfig {
    pub username: String,
    pub password_hash: String,
    pub root: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: SocketAddr,
    #[serde(default)]
    pub filesystem: FilesystemConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub users: Vec<UserConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilesystemConfig {
    #[serde(default = "default_root")]
    pub root: PathBuf,
    #[serde(default = "default_max_upload_size_mb")]
    pub max_upload_size_mb: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub jwt_secret: String,
    #[serde(default)]
    pub password_hash: String,
    #[serde(default = "default_jwt_ttl_hours")]
    pub jwt_ttl_hours: u64,
    #[serde(default)]
    pub discord_webhook_url: Option<String>,
    #[serde(default = "default_otp_ttl_seconds")]
    pub otp_ttl_seconds: u64,
    #[serde(default = "default_webauthn_rp_id")]
    pub webauthn_rp_id: String,
    #[serde(default)]
    pub webauthn_rp_origin: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_login_rpm")]
    pub login_requests_per_minute: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: String::new(),
            password_hash: String::new(),
            jwt_ttl_hours: 24,
            discord_webhook_url: None,
            otp_ttl_seconds: 300,
            webauthn_rp_id: "localhost".to_string(),
            webauthn_rp_origin: None,
        }
    }
}

fn default_jwt_ttl_hours() -> u64 { 24 }
fn default_otp_ttl_seconds() -> u64 { 300 }
fn default_webauthn_rp_id() -> String { "localhost".to_string() }
fn default_login_rpm() -> u32 { 5 }

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self { login_requests_per_minute: default_login_rpm() }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TlsConfig {
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

fn default_bind_addr() -> SocketAddr {
    "0.0.0.0:9090".parse().unwrap()
}

fn default_max_upload_size_mb() -> usize { 100 }

fn default_root() -> PathBuf {
    dirs_home().unwrap_or_else(|| PathBuf::from("/"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        Self {
            root: default_root(),
            max_upload_size_mb: default_max_upload_size_mb(),
        }
    }
}

impl ServerConfig {
    pub fn find_user(&self, username: &str) -> Option<&UserConfig> {
        self.users.iter().find(|u| u.username == username)
    }

    pub fn is_multi_user(&self) -> bool {
        !self.users.is_empty()
    }

    pub fn resolve_root(&self, username: &str) -> &PathBuf {
        if self.is_multi_user() {
            if let Some(user) = self.find_user(username) {
                return &user.root;
            }
        }
        &self.filesystem.root
    }

    /// Returns `true` if any form of authentication is configured.
    pub fn has_auth(&self) -> bool {
        self.is_multi_user() || !self.auth.password_hash.is_empty()
    }

    pub fn load() -> anyhow::Result<Self> {
        let config_path = std::env::var("TREFM_WEB_CONFIG")
            .map(PathBuf::from)
            .ok();

        let mut config = if let Some(path) = config_path {
            let contents = std::fs::read_to_string(&path)?;
            toml::from_str(&contents)?
        } else {
            ServerConfig {
                bind_addr: default_bind_addr(),
                filesystem: FilesystemConfig::default(),
                auth: AuthConfig::default(),
                rate_limit: RateLimitConfig::default(),
                tls: TlsConfig::default(),
                users: Vec::new(),
            }
        };

        if let Ok(secret) = std::env::var("TREFM_JWT_SECRET") {
            config.auth.jwt_secret = secret;
        }
        if config.auth.jwt_secret.is_empty() {
            config.auth.jwt_secret = uuid::Uuid::new_v4().to_string();
            tracing::warn!(
                "No JWT secret configured. Generated random secret (will change on restart)."
            );
        }

        if let Ok(hash) = std::env::var("TREFM_PASSWORD_HASH") {
            config.auth.password_hash = hash;
        }

        if let Ok(url) = std::env::var("TREFM_DISCORD_WEBHOOK_URL") {
            config.auth.discord_webhook_url = Some(url);
        }
        if let Ok(rp_id) = std::env::var("TREFM_WEBAUTHN_RP_ID") {
            config.auth.webauthn_rp_id = rp_id;
        }
        if let Ok(origin) = std::env::var("TREFM_WEBAUTHN_RP_ORIGIN") {
            config.auth.webauthn_rp_origin = Some(origin);
        }

        if let Ok(root) = std::env::var("TREFM_ROOT") {
            config.filesystem.root = PathBuf::from(root);
        }

        if let Ok(val) = std::env::var("TREFM_MAX_UPLOAD_SIZE_MB") {
            if let Ok(mb) = val.parse::<usize>() {
                config.filesystem.max_upload_size_mb = mb;
            }
        }

        if let Ok(addr) = std::env::var("TREFM_BIND_ADDR") {
            config.bind_addr = addr.parse()?;
        }

        if let Ok(cert) = std::env::var("TREFM_TLS_CERT") {
            config.tls.cert_path = Some(cert);
        }
        if let Ok(key) = std::env::var("TREFM_TLS_KEY") {
            config.tls.key_path = Some(key);
        }

        // Security: validate JWT secret strength when auth is enabled
        if config.has_auth() {
            const WEAK_SECRETS: &[&str] = &[
                "change-me-to-a-random-secret",
                "secret",
                "password",
                "jwt-secret",
            ];
            if WEAK_SECRETS.iter().any(|&w| config.auth.jwt_secret == w) {
                anyhow::bail!(
                    "JWT secret matches a known weak/placeholder value. \
                     Set a strong random secret via TREFM_JWT_SECRET environment variable."
                );
            }
            if config.auth.jwt_secret.len() < 32 {
                tracing::warn!(
                    "JWT secret is shorter than 32 characters. \
                     Consider using a stronger secret via TREFM_JWT_SECRET."
                );
            }
        }

        // Security: restrict binding when no auth is configured
        if !config.has_auth() && config.bind_addr.ip().is_unspecified() {
            if std::env::var("TREFM_INSECURE").is_ok() {
                tracing::warn!(
                    "Running WITHOUT authentication on all interfaces ({}). \
                     Anyone on the network can access the terminal!",
                    config.bind_addr
                );
            } else {
                let safe_addr: SocketAddr =
                    ([127, 0, 0, 1], config.bind_addr.port()).into();
                tracing::warn!(
                    "No authentication configured. Binding to {} instead of {} for safety. \
                     Set TREFM_INSECURE=1 to override (NOT RECOMMENDED).",
                    safe_addr, config.bind_addr
                );
                config.bind_addr = safe_addr;
            }
        }

        Ok(config)
    }
}

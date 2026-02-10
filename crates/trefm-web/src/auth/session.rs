use std::time::{Duration, Instant};

use dashmap::DashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthStep {
    PasswordVerified,
    WebAuthnVerified,
    Complete,
}

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub username: String,
    pub step: AuthStep,
    pub created_at: Instant,
    pub otp_code: Option<String>,
    pub otp_sent_at: Option<Instant>,
    pub webauthn_state: Option<String>,
}

pub struct SessionStore {
    sessions: DashMap<String, AuthSession>,
    ttl: Duration,
}

impl SessionStore {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            sessions: DashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub fn create(&self, session_id: String, session: AuthSession) {
        tracing::debug!("Session created: id={session_id}, step={:?}", session.step);
        self.sessions.insert(session_id, session);
        tracing::debug!("Session store size: {}", self.sessions.len());
    }

    pub fn get(&self, session_id: &str) -> Option<AuthSession> {
        tracing::debug!(
            "Session lookup: id={session_id}, store_size={}",
            self.sessions.len()
        );
        let entry = self.sessions.get(session_id)?;
        if entry.created_at.elapsed() > self.ttl {
            tracing::debug!("Session expired: id={session_id}");
            drop(entry);
            self.sessions.remove(session_id);
            return None;
        }
        tracing::debug!("Session found: id={session_id}, step={:?}", entry.step);
        Some(entry.clone())
    }

    pub fn update(&self, session_id: &str, session: AuthSession) {
        self.sessions.insert(session_id.to_string(), session);
    }

    pub fn remove(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    pub fn cleanup_expired(&self) {
        let ttl = self.ttl;
        self.sessions.retain(|_, session| session.created_at.elapsed() <= ttl);
    }
}

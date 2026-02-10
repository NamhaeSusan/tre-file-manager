use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    pub username: Option<String>,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum AuthStepResponse {
    #[serde(rename = "next_step")]
    NextStep {
        session_id: String,
        next_step: String,
    },
    #[serde(rename = "complete")]
    Complete {
        token: String,
        expires_at: u64,
    },
}

#[derive(Debug, Deserialize)]
pub struct SessionRequest {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct OtpVerifyRequest {
    pub session_id: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct WebAuthnVerifyRequest {
    pub session_id: String,
    pub credential: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct FileEntryDto {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ListDirQuery {
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListDirResponse {
    pub entries: Vec<FileEntryDto>,
    pub current_path: String,
}

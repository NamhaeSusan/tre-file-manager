use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub fn create_token(jwt_secret: &str, ttl_hours: u64, username: &str) -> anyhow::Result<(String, u64)> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let expires_at = now + ttl_hours * 3600;

    let claims = Claims {
        sub: username.to_string(),
        exp: expires_at as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )?;

    Ok((token, expires_at))
}

pub fn verify_token(jwt_secret: &str, token: &str) -> anyhow::Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

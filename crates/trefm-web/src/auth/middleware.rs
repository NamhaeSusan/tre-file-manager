use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::AppError;
use crate::state::AppState;

pub struct AuthUser {
    pub sub: String,
    #[allow(dead_code)]
    pub jti: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // If no auth is configured, skip auth (dev mode)
        let has_auth = state.config.is_multi_user() || !state.config.auth.password_hash.is_empty();
        if !has_auth {
            return Ok(AuthUser {
                sub: "anonymous".to_string(),
                jti: String::new(),
            });
        }

        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Auth("Missing authorization header".to_string()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Auth("Invalid authorization header format".to_string()))?;

        let claims = super::jwt::verify_token(&state.config.auth.jwt_secret, token)
            .map_err(|_| AppError::Auth("Invalid or expired token".to_string()))?;

        if state.revoked_tokens.contains_key(&claims.jti) {
            return Err(AppError::Auth("Token has been revoked".to_string()));
        }

        Ok(AuthUser {
            sub: claims.sub,
            jti: claims.jti,
        })
    }
}

use std::time::Instant;

use axum::extract::State;
use axum::Json;

use crate::auth::jwt;
use crate::auth::middleware::AuthUser;
use crate::auth::session::{AuthSession, AuthStep};
use crate::dto::*;
use crate::error::AppError;
use crate::state::AppState;

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthStepResponse>, AppError> {
    let username: String;

    if state.config.is_multi_user() {
        // Multi-user mode
        let uname = body
            .username
            .clone()
            .ok_or_else(|| AppError::Auth("Username is required".to_string()))?;

        let user_config = state
            .config
            .find_user(&uname)
            .ok_or_else(|| AppError::Auth("Invalid credentials".to_string()))?
            .clone();

        let password = body.password.clone();
        let hash = user_config.password_hash.clone();

        let valid = tokio::task::spawn_blocking(move || {
            crate::auth::password::verify_password(&hash, &password)
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;

        if !valid {
            tracing::warn!("Failed login attempt for user: {uname}");
            return Err(AppError::Auth("Invalid credentials".to_string()));
        }

        username = uname;
    } else {
        // Single-user mode (legacy)
        if state.config.auth.password_hash.is_empty() {
            // Dev mode: skip all auth
            let (token, expires_at) = jwt::create_token(
                &state.config.auth.jwt_secret,
                state.config.auth.jwt_ttl_hours,
                "user",
            )?;
            return Ok(Json(AuthStepResponse::Complete { token, expires_at }));
        }

        let hash = state.config.auth.password_hash.clone();
        let password = body.password.clone();

        let valid = tokio::task::spawn_blocking(move || {
            crate::auth::password::verify_password(&hash, &password)
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;

        if !valid {
            tracing::warn!("Failed login attempt: invalid password");
            return Err(AppError::Auth("Invalid credentials".to_string()));
        }

        username = body.username.unwrap_or_else(|| "user".to_string());
    }

    tracing::info!("Password verified successfully for user: {username}");

    // Determine next step
    let has_webauthn = state
        .webauthn
        .as_ref()
        .is_some_and(|w| w.has_credentials_for(&username));
    let has_discord = state.config.auth.discord_webhook_url.is_some();

    if !has_webauthn && !has_discord {
        // No additional factors configured -> issue token directly
        let (token, expires_at) = jwt::create_token(
            &state.config.auth.jwt_secret,
            state.config.auth.jwt_ttl_hours,
            &username,
        )?;
        return Ok(Json(AuthStepResponse::Complete { token, expires_at }));
    }

    // Create session for multi-step auth
    let session_id = uuid::Uuid::new_v4().to_string();

    if !has_webauthn && has_discord {
        // Skip WebAuthn, go straight to OTP
        let otp = crate::auth::discord_otp::generate_otp();
        let webhook_url = state
            .config
            .auth
            .discord_webhook_url
            .as_ref()
            .unwrap()
            .clone();
        let otp_clone = otp.clone();
        let username_clone = username.clone();

        tokio::spawn(async move {
            if let Err(e) =
                crate::auth::discord_otp::send_otp_to_discord(&webhook_url, &otp_clone, &username_clone).await
            {
                tracing::error!("Failed to send OTP to Discord: {e}");
            }
        });

        let session = AuthSession {
            username: username.clone(),
            step: AuthStep::PasswordVerified,
            created_at: Instant::now(),
            otp_code: Some(otp),
            otp_sent_at: Some(Instant::now()),
            webauthn_state: None,
        };

        state.session_store.create(session_id.clone(), session);

        return Ok(Json(AuthStepResponse::NextStep {
            session_id,
            next_step: "otp".to_string(),
        }));
    }

    let session = AuthSession {
        username: username.clone(),
        step: AuthStep::PasswordVerified,
        created_at: Instant::now(),
        otp_code: None,
        otp_sent_at: None,
        webauthn_state: None,
    };

    state.session_store.create(session_id.clone(), session);

    let next_step = if has_webauthn { "webauthn" } else { "otp" };

    Ok(Json(AuthStepResponse::NextStep {
        session_id,
        next_step: next_step.to_string(),
    }))
}

pub async fn webauthn_challenge(
    State(state): State<AppState>,
    Json(body): Json<SessionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = state
        .session_store
        .get(&body.session_id)
        .ok_or_else(|| AppError::Auth("Invalid or expired session".to_string()))?;

    if session.step != AuthStep::PasswordVerified {
        return Err(AppError::Auth("Invalid auth step".to_string()));
    }

    let username = session.username.clone();

    let webauthn = state
        .webauthn
        .as_ref()
        .ok_or_else(|| AppError::Internal("WebAuthn not configured".to_string()))?;

    let (challenge, auth_state) = webauthn
        .start_authentication(&username)
        .map_err(|e| AppError::Internal(format!("WebAuthn challenge failed: {e}")))?;

    // Store auth state in session (serialize it)
    let state_json = serde_json::to_string(&auth_state)
        .map_err(|e| AppError::Internal(format!("Failed to serialize state: {e}")))?;

    let updated_session = AuthSession {
        webauthn_state: Some(state_json),
        ..session
    };
    state.session_store.update(&body.session_id, updated_session);

    let challenge_value = serde_json::to_value(challenge)
        .map_err(|e| AppError::Internal(format!("Failed to serialize challenge: {e}")))?;

    Ok(Json(challenge_value))
}

pub async fn webauthn_verify(
    State(state): State<AppState>,
    Json(body): Json<WebAuthnVerifyRequest>,
) -> Result<Json<AuthStepResponse>, AppError> {
    let session = state
        .session_store
        .get(&body.session_id)
        .ok_or_else(|| AppError::Auth("Invalid or expired session".to_string()))?;

    if session.step != AuthStep::PasswordVerified {
        return Err(AppError::Auth("Invalid auth step".to_string()));
    }

    let username = session.username.clone();

    let webauthn = state
        .webauthn
        .as_ref()
        .ok_or_else(|| AppError::Internal("WebAuthn not configured".to_string()))?;

    let auth_state_json = session
        .webauthn_state
        .as_ref()
        .ok_or_else(|| AppError::Auth("No WebAuthn challenge in progress".to_string()))?;

    let auth_state: webauthn_rs::prelude::PasskeyAuthentication =
        serde_json::from_str(auth_state_json)
            .map_err(|e| AppError::Internal(format!("Failed to deserialize state: {e}")))?;

    let credential: webauthn_rs::prelude::PublicKeyCredential =
        serde_json::from_value(body.credential)
            .map_err(|e| AppError::Auth(format!("Invalid credential: {e}")))?;

    webauthn
        .finish_authentication(&username, &credential, &auth_state)
        .map_err(|e| {
            tracing::warn!("WebAuthn verification failed: {e}");
            AppError::Auth("WebAuthn verification failed".to_string())
        })?;

    tracing::info!("WebAuthn verified successfully for user: {username}");

    let has_discord = state.config.auth.discord_webhook_url.is_some();

    if has_discord {
        // Send OTP
        let otp = crate::auth::discord_otp::generate_otp();
        let webhook_url = state
            .config
            .auth
            .discord_webhook_url
            .as_ref()
            .unwrap()
            .clone();
        let otp_clone = otp.clone();
        let username_clone = username.clone();

        tokio::spawn(async move {
            if let Err(e) =
                crate::auth::discord_otp::send_otp_to_discord(&webhook_url, &otp_clone, &username_clone).await
            {
                tracing::error!("Failed to send OTP to Discord: {e}");
            }
        });

        let updated_session = AuthSession {
            username: username.clone(),
            step: AuthStep::WebAuthnVerified,
            otp_code: Some(otp),
            otp_sent_at: Some(Instant::now()),
            webauthn_state: None,
            ..session
        };
        state.session_store.update(&body.session_id, updated_session);

        return Ok(Json(AuthStepResponse::NextStep {
            session_id: body.session_id,
            next_step: "otp".to_string(),
        }));
    }

    // No Discord -> complete
    state.session_store.remove(&body.session_id);

    let (token, expires_at) = jwt::create_token(
        &state.config.auth.jwt_secret,
        state.config.auth.jwt_ttl_hours,
        &username,
    )?;

    Ok(Json(AuthStepResponse::Complete { token, expires_at }))
}

pub async fn otp_verify(
    State(state): State<AppState>,
    Json(body): Json<OtpVerifyRequest>,
) -> Result<Json<AuthStepResponse>, AppError> {
    let session = state
        .session_store
        .get(&body.session_id)
        .ok_or_else(|| AppError::Auth("Invalid or expired session".to_string()))?;

    // Accept OTP after password-verified or webauthn-verified
    if session.step != AuthStep::WebAuthnVerified && session.step != AuthStep::PasswordVerified {
        return Err(AppError::Auth("Invalid auth step".to_string()));
    }

    let username = session.username.clone();

    let expected_otp = session
        .otp_code
        .as_ref()
        .ok_or_else(|| AppError::Auth("No OTP pending".to_string()))?;

    // Check OTP expiry
    if let Some(sent_at) = session.otp_sent_at {
        let ttl = std::time::Duration::from_secs(state.config.auth.otp_ttl_seconds);
        if sent_at.elapsed() > ttl {
            state.session_store.remove(&body.session_id);
            return Err(AppError::Auth("OTP expired".to_string()));
        }
    }

    if !constant_time_eq(body.code.as_bytes(), expected_otp.as_bytes()) {
        tracing::warn!("Invalid OTP attempt for user: {username}");
        return Err(AppError::Auth("Invalid OTP code".to_string()));
    }

    tracing::info!("OTP verified successfully for user: {username}");
    state.session_store.remove(&body.session_id);

    let (token, expires_at) = jwt::create_token(
        &state.config.auth.jwt_secret,
        state.config.auth.jwt_ttl_hours,
        &username,
    )?;

    Ok(Json(AuthStepResponse::Complete { token, expires_at }))
}

/// Constant-time byte comparison to prevent timing side-channel attacks on OTP.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Logout handler that accepts the token from either the Authorization header
/// or the JSON body `{ "token": "..." }`. This dual approach supports both
/// regular fetch (with Authorization header) and navigator.sendBeacon (body only).
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    // Try Authorization header first
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| {
            // Fallback: read token from JSON body
            serde_json::from_slice::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("token")?.as_str().map(String::from))
        });

    let token = match token {
        Some(t) => t,
        None => return Ok(Json(serde_json::json!({ "success": true }))),
    };

    if let Ok(claims) = crate::auth::jwt::verify_token(&state.config.auth.jwt_secret, &token) {
        state
            .revoked_tokens
            .insert(claims.jti.clone(), Instant::now());
        tracing::info!(
            "Token revoked for user: {} (jti: {})",
            claims.sub,
            claims.jti
        );
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// WebAuthn registration (requires JWT auth)
pub async fn webauthn_register_start(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let username = &user.sub;

    let webauthn = state
        .webauthn
        .as_ref()
        .ok_or_else(|| AppError::Internal("WebAuthn not configured".to_string()))?;

    let (challenge, reg_state) = webauthn
        .start_registration(username)
        .map_err(|e| AppError::Internal(format!("Registration start failed: {e}")))?;

    // Store reg state in a temporary session
    let session_id = uuid::Uuid::new_v4().to_string();
    let state_json = serde_json::to_string(&reg_state)
        .map_err(|e| AppError::Internal(format!("Failed to serialize: {e}")))?;

    let session = AuthSession {
        username: username.clone(),
        step: AuthStep::Complete,
        created_at: Instant::now(),
        otp_code: None,
        otp_sent_at: None,
        webauthn_state: Some(state_json),
    };
    state.session_store.create(session_id.clone(), session);

    let mut response = serde_json::to_value(challenge)
        .map_err(|e| AppError::Internal(format!("Failed to serialize challenge: {e}")))?;
    response["session_id"] = serde_json::Value::String(session_id);

    Ok(Json(response))
}

pub async fn webauthn_register_finish(
    user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let username = &user.sub;

    let session_id = body["session_id"]
        .as_str()
        .ok_or_else(|| AppError::Auth("Missing session_id".to_string()))?;

    let session = state
        .session_store
        .get(session_id)
        .ok_or_else(|| AppError::Auth("Invalid or expired session".to_string()))?;

    let webauthn = state
        .webauthn
        .as_ref()
        .ok_or_else(|| AppError::Internal("WebAuthn not configured".to_string()))?;

    let reg_state_json = session
        .webauthn_state
        .as_ref()
        .ok_or_else(|| AppError::Auth("No registration in progress".to_string()))?;

    let reg_state: webauthn_rs::prelude::PasskeyRegistration =
        serde_json::from_str(reg_state_json)
            .map_err(|e| AppError::Internal(format!("Failed to deserialize: {e}")))?;

    let credential: webauthn_rs::prelude::RegisterPublicKeyCredential =
        serde_json::from_value(body["credential"].clone())
            .map_err(|e| AppError::Auth(format!("Invalid credential: {e}")))?;

    webauthn
        .finish_registration(username, &credential, &reg_state)
        .map_err(|e| AppError::Internal(format!("Registration failed: {e}")))?;

    state.session_store.remove(session_id);

    tracing::info!("Passkey registered successfully for user: {username}");

    Ok(Json(serde_json::json!({ "success": true })))
}

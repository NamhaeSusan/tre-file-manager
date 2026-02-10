mod api;
mod auth;
mod config;
mod dto;
mod error;
mod middleware;
mod state;
mod static_files;
mod ws;

use std::sync::Arc;

use axum::middleware::from_fn;
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer};
use axum::http::{header, Method};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::ServerConfig;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "trefm_web=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = ServerConfig::load()?;
    let bind_addr = config.bind_addr;
    let tls_config = config.tls.clone();
    let tls_enabled = tls_config.cert_path.is_some() && tls_config.key_path.is_some();
    let rate_limit_rpm = config.rate_limit.login_requests_per_minute;

    // Session store
    let session_store = Arc::new(crate::auth::session::SessionStore::new(600));

    // WebAuthn (optional)
    let webauthn = if !config.auth.webauthn_rp_id.is_empty() {
        let origin = config.auth.webauthn_rp_origin.clone()
            .unwrap_or_else(|| format!("https://{}", config.auth.webauthn_rp_id));
        match crate::auth::webauthn_manager::WebAuthnManager::new(
            &config.auth.webauthn_rp_id,
            &origin,
        ) {
            Ok(wm) => {
                tracing::info!("WebAuthn enabled (rp_id: {})", config.auth.webauthn_rp_id);
                Some(Arc::new(wm))
            }
            Err(e) => {
                tracing::warn!("WebAuthn initialization failed: {e}");
                None
            }
        }
    } else {
        None
    };

    let ws_tickets = Arc::new(dashmap::DashMap::new());

    let state = AppState {
        config: Arc::new(config),
        session_store: session_store.clone(),
        webauthn,
        ws_tickets: ws_tickets.clone(),
    };

    // Session + ticket cleanup task
    let cleanup_store = session_store.clone();
    let cleanup_tickets = ws_tickets.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            cleanup_store.cleanup_expired();
            // Remove expired WebSocket tickets (>30s)
            cleanup_tickets.retain(|_, t: &mut crate::state::WsTicket| {
                t.created_at.elapsed() < std::time::Duration::from_secs(30)
            });
        }
    });

    // CORS: same-origin only by default (no cross-origin requests allowed)
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    // Rate limit config (per-IP)
    let period_per_request = 60 / rate_limit_rpm.max(1);
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(period_per_request.into())
            .burst_size(rate_limit_rpm)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("Failed to build rate limit config"),
    );

    // Rate limit + bot guard only on auth routes (not static files, API, or WebSocket)
    let auth_routes = api::auth_router()
        .layer(from_fn(middleware::bot_guard::bot_guard))
        .layer(GovernorLayer::<_, _, axum::body::Body>::new(governor_config));

    let base_router = axum::Router::new()
        .nest("/api", auth_routes.merge(api::protected_router()))
        .nest("/ws", ws::router())
        .fallback(static_files::static_handler);

    let app = if tls_enabled {
        base_router
            .layer(from_fn(middleware::security_headers::security_headers_with_hsts))
            .layer(RequestBodyLimitLayer::new(1024 * 1024))
            .layer(cors)
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    } else {
        base_router
            .layer(from_fn(middleware::security_headers::security_headers))
            .layer(RequestBodyLimitLayer::new(1024 * 1024))
            .layer(cors)
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    };

    if let (Some(cert), Some(key)) = (&tls_config.cert_path, &tls_config.key_path) {
        use axum_server::tls_rustls::RustlsConfig;
        let rustls_config = RustlsConfig::from_pem_file(cert, key).await?;
        tracing::info!("trefm-web listening on https://{}", bind_addr);
        axum_server::bind_rustls(bind_addr, rustls_config)
            .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        tracing::info!("trefm-web listening on http://{}", bind_addr);
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await?;
    }

    Ok(())
}

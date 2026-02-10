use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

const BOT_PATTERNS: &[&str] = &[
    "bot", "crawl", "spider", "scrape", "curl", "wget", "python-requests", "httpie", "go-http",
];

pub async fn bot_guard(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match user_agent {
        None => {
            tracing::warn!("Blocked request without User-Agent");
            Err(StatusCode::FORBIDDEN)
        }
        Some(ua) => {
            let ua_lower = ua.to_lowercase();
            for pattern in BOT_PATTERNS {
                if ua_lower.contains(pattern) {
                    tracing::warn!("Blocked bot request: {ua}");
                    return Err(StatusCode::FORBIDDEN);
                }
            }
            Ok(next.run(req).await)
        }
    }
}

use axum::body::Body;
use axum::http::{Request, header::HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

fn add_common_headers(response: &mut Response) {
    let headers = response.headers_mut();

    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'self'; connect-src 'self' wss: ws:; style-src 'self' 'unsafe-inline'",
        ),
    );
}

pub async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    add_common_headers(&mut response);
    response
}

pub async fn security_headers_with_hsts(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    add_common_headers(&mut response);
    response.headers_mut().insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    response
}

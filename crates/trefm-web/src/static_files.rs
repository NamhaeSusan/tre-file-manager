use axum::http::{header, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "web/dist/"]
struct Assets;

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try exact file match
    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            content.data.into_owned(),
        )
            .into_response()
    } else if let Some(content) = Assets::get("index.html") {
        // SPA fallback
        Html(String::from_utf8_lossy(&content.data).to_string()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

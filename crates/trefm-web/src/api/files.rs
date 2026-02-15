use axum::extract::{Multipart, Query, State};
use axum::response::Response;
use axum::Json;

use crate::auth::middleware::AuthUser;
use crate::dto::{DownloadQuery, FileEntryDto, ListDirQuery, ListDirResponse, UploadResponse};
use crate::error::AppError;
use crate::state::AppState;

pub async fn list_directory(
    user: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListDirQuery>,
) -> Result<Json<ListDirResponse>, AppError> {
    let root = state.config.resolve_root(&user.sub);

    let target = match &query.path {
        Some(p) => std::path::PathBuf::from(p),
        None => root.clone(),
    };

    // Path traversal protection
    let canonical = target
        .canonicalize()
        .map_err(|_| AppError::NotFound(format!("Path not found: {}", target.display())))?;

    let canonical_root = root
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Failed to resolve root path: {e}")))?;

    if !canonical.starts_with(&canonical_root) {
        return Err(AppError::NotFound("Access denied".to_string()));
    }

    if !canonical.is_dir() {
        return Err(AppError::NotFound(format!(
            "Not a directory: {}",
            canonical.display()
        )));
    }

    let read_dir = std::fs::read_dir(&canonical)
        .map_err(|e| AppError::Internal(format!("Failed to read directory: {e}")))?;

    let mut entries: Vec<FileEntryDto> = Vec::new();

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().to_string_lossy().to_string();

        let metadata = entry.path().symlink_metadata();
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let is_hidden = name.starts_with('.');
        let is_symlink = metadata
            .as_ref()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        let size = if is_dir {
            None
        } else {
            metadata.as_ref().map(|m| m.len()).ok()
        };

        entries.push(FileEntryDto {
            name,
            path,
            is_dir,
            is_hidden,
            is_symlink,
            size,
        });
    }

    // Sort: directories first, then alphabetical by name
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(Json(ListDirResponse {
        entries,
        current_path: canonical.to_string_lossy().to_string(),
    }))
}

pub async fn download_file(
    user: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, AppError> {
    let root = state.config.resolve_root(&user.sub);
    let target = std::path::PathBuf::from(&query.path);

    let canonical = target.canonicalize()
        .map_err(|_| AppError::NotFound("Path not found".to_string()))?;
    let canonical_root = root.canonicalize()
        .map_err(|e| AppError::Internal(format!("Failed to resolve root: {e}")))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(AppError::NotFound("Access denied".to_string()));
    }
    if canonical.is_dir() {
        return Err(AppError::NotFound("Cannot download a directory".to_string()));
    }

    let metadata = tokio::fs::metadata(&canonical).await
        .map_err(|_| AppError::NotFound("File not found".to_string()))?;
    let mime = mime_guess::from_path(&canonical).first_or_octet_stream();
    let filename = canonical.file_name()
        .unwrap_or_default().to_string_lossy().to_string();

    let file = tokio::fs::File::open(&canonical).await
        .map_err(|e| AppError::Internal(format!("Failed to open file: {e}")))?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    Ok(Response::builder()
        .header("Content-Type", mime.as_ref())
        .header("Content-Length", metadata.len())
        .header("Content-Disposition", format!("attachment; filename=\"{}\"", filename))
        .body(body)
        .unwrap())
}

pub async fn upload_file(
    user: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    let root = state.config.resolve_root(&user.sub);
    let mut target_dir: Option<String> = None;
    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::Internal(format!("Multipart error: {e}")))? {
        match field.name() {
            Some("path") => {
                target_dir = Some(field.text().await
                    .map_err(|e| AppError::Internal(format!("Read error: {e}")))?);
            }
            Some("file") => {
                let filename = field.file_name()
                    .unwrap_or("upload").to_string();
                let data = field.bytes().await
                    .map_err(|e| AppError::Internal(format!("Read error: {e}")))?;
                file_data = Some((filename, data.to_vec()));
            }
            _ => {}
        }
    }

    let (raw_filename, data) = file_data
        .ok_or_else(|| AppError::Internal("Missing file field".to_string()))?;

    let sanitized = sanitize_filename(&raw_filename);
    if sanitized.is_empty() {
        return Err(AppError::Internal("Invalid filename".to_string()));
    }

    let dir = match &target_dir {
        Some(p) => std::path::PathBuf::from(p),
        None => root.clone(),
    };
    let canonical_dir = dir.canonicalize()
        .map_err(|_| AppError::NotFound("Target directory not found".to_string()))?;
    let canonical_root = root.canonicalize()
        .map_err(|e| AppError::Internal(format!("Failed to resolve root: {e}")))?;
    if !canonical_dir.starts_with(&canonical_root) {
        return Err(AppError::NotFound("Access denied".to_string()));
    }
    if !canonical_dir.is_dir() {
        return Err(AppError::NotFound("Target is not a directory".to_string()));
    }

    let dest = canonical_dir.join(&sanitized);
    let size = data.len() as u64;
    tokio::fs::write(&dest, &data).await
        .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;

    Ok(Json(UploadResponse {
        success: true,
        path: dest.to_string_lossy().to_string(),
        filename: sanitized,
        size,
    }))
}

fn sanitize_filename(name: &str) -> String {
    let name = name.replace(['/', '\\', '\0'], "");
    let name = name.trim_start_matches('.');
    let name: String = name.chars()
        .filter(|c| !c.is_control())
        .take(255)
        .collect();
    name
}

use axum::extract::{Query, State};
use axum::Json;

use crate::auth::middleware::AuthUser;
use crate::dto::{FileEntryDto, ListDirQuery, ListDirResponse};
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

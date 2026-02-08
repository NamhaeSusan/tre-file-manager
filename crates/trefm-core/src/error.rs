//! Error types for `trefm-core`.
//!
//! All fallible operations in the core library return [`CoreResult<T>`],
//! which is an alias for `Result<T, CoreError>`.

use std::path::PathBuf;

/// Unified error type for all core operations.
///
/// Each variant captures just enough context for the caller to display
/// a meaningful message or take corrective action.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// The target path does not exist.
    #[error("path not found: {0}")]
    NotFound(PathBuf),

    /// The process lacks permission to access the path.
    #[error("permission denied: {0}")]
    PermissionDenied(PathBuf),

    /// A directory was expected but the path points to a file.
    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),

    /// A file or directory name is invalid (empty, contains path separators, etc.).
    #[error("invalid name: {0}")]
    InvalidName(String),

    /// Failed to parse a TOML configuration file.
    #[error("config parse error: {0}")]
    ConfigParse(String),

    /// The user cancelled an interactive operation.
    #[error("operation cancelled")]
    Cancelled,

    /// A git operation failed.
    #[error("git error: {0}")]
    Git(String),

    /// A remote (SSH/SFTP) operation failed.
    #[error("remote error: {0}")]
    Remote(String),

    /// An I/O error that doesn't fit a more specific variant.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Convenience alias used throughout `trefm-core`.
pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn not_found_displays_path() {
        let err = CoreError::NotFound(PathBuf::from("/missing/file"));
        assert_eq!(err.to_string(), "path not found: /missing/file");
    }

    #[test]
    fn permission_denied_displays_path() {
        let err = CoreError::PermissionDenied(PathBuf::from("/secret"));
        assert_eq!(err.to_string(), "permission denied: /secret");
    }

    #[test]
    fn not_a_directory_displays_path() {
        let err = CoreError::NotADirectory(PathBuf::from("/some/file.txt"));
        assert_eq!(err.to_string(), "not a directory: /some/file.txt");
    }

    #[test]
    fn invalid_name_displays_message() {
        let err = CoreError::InvalidName("bad/name".to_string());
        assert_eq!(err.to_string(), "invalid name: bad/name");
    }

    #[test]
    fn config_parse_displays_message() {
        let err = CoreError::ConfigParse("unexpected token".to_string());
        assert_eq!(err.to_string(), "config parse error: unexpected token");
    }

    #[test]
    fn cancelled_displays_message() {
        let err = CoreError::Cancelled;
        assert_eq!(err.to_string(), "operation cancelled");
    }

    #[test]
    fn io_error_from_std() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let core_err: CoreError = io_err.into();
        assert!(matches!(core_err, CoreError::Io(_)));
        assert!(core_err.to_string().contains("gone"));
    }

    #[test]
    fn core_result_ok() {
        let result: CoreResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn core_result_err() {
        let result: CoreResult<i32> = Err(CoreError::Cancelled);
        assert!(result.is_err());
    }

    #[test]
    fn remote_error_displays_message() {
        let err = CoreError::Remote("connection refused".to_string());
        assert_eq!(err.to_string(), "remote error: connection refused");
    }

    #[test]
    fn error_is_debug() {
        let err = CoreError::NotFound(PathBuf::from("/test"));
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }
}

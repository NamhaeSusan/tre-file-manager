//! Image preview caching and protocol state management.
//!
//! Manages terminal graphics protocol detection and caches
//! encoded images to avoid re-encoding on every render frame.

use std::path::{Path, PathBuf};

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

/// Cache key: file path + render area dimensions.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ImageCacheKey {
    path: PathBuf,
    width: u16,
    height: u16,
}

/// Holds the picker and cached encoded image protocol.
pub struct ImagePreviewState {
    picker: Picker,
    cached_protocol: Option<StatefulProtocol>,
    cache_key: Option<ImageCacheKey>,
}

impl ImagePreviewState {
    pub fn new(picker: Picker) -> Self {
        Self {
            picker,
            cached_protocol: None,
            cache_key: None,
        }
    }

    /// Returns `&mut StatefulProtocol` (cached or newly encoded).
    ///
    /// Re-encodes only when the file path or render area size changes.
    pub fn get_or_encode(
        &mut self,
        path: &Path,
        width: u16,
        height: u16,
    ) -> Option<&mut StatefulProtocol> {
        let new_key = ImageCacheKey {
            path: path.to_path_buf(),
            width,
            height,
        };

        let needs_encode = self
            .cache_key
            .as_ref()
            .is_none_or(|existing| *existing != new_key);

        if needs_encode {
            let dyn_img = match image::ImageReader::open(path) {
                Ok(reader) => match reader.decode() {
                    Ok(img) => img,
                    Err(e) => {
                        tracing::debug!("Image decode failed for {}: {e}", path.display());
                        self.cached_protocol = None;
                        self.cache_key = None;
                        return None;
                    }
                },
                Err(e) => {
                    tracing::debug!("Image open failed for {}: {e}", path.display());
                    self.cached_protocol = None;
                    self.cache_key = None;
                    return None;
                }
            };

            let protocol = self.picker.new_resize_protocol(dyn_img);
            self.cached_protocol = Some(protocol);
            self.cache_key = Some(new_key);
        }

        self.cached_protocol.as_mut()
    }

    /// Invalidates the cache (e.g. when file watcher detects changes).
    pub fn invalidate(&mut self) {
        self.cached_protocol = None;
        self.cache_key = None;
    }
}

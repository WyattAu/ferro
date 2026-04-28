use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::path::{Path as StdPath, PathBuf};
use tracing::{debug, warn};

use crate::AppState;

const SUPPORTED_IMAGE_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
];

const FILE_ICON_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><rect width="128" height="128" rx="12" fill="#e2e8f0"/><path d="M35 25h40l18 18v60a6 6 0 0 1-6 6H35a6 6 0 0 1-6-6V31a6 6 0 0 1 6-6z" fill="#94a3b8"/><path d="M75 25v18h18" fill="#64748b"/></svg>"##;

#[derive(Debug, Clone)]
pub struct ThumbnailService {
    data_dir: PathBuf,
    max_size: u32,
}

impl ThumbnailService {
    pub fn new(data_dir: &str, max_size: u32) -> Self {
        let thumb_dir = StdPath::new(data_dir).join("thumbnails");
        std::fs::create_dir_all(&thumb_dir).ok();
        Self {
            data_dir: StdPath::new(data_dir).to_path_buf(),
            max_size: max_size.clamp(64, 1024),
        }
    }

    fn thumb_dir(&self) -> PathBuf {
        self.data_dir.join("thumbnails")
    }

    fn cache_path(&self, hash: &str) -> PathBuf {
        self.thumb_dir().join(format!("{}.jpg", hash))
    }

    fn cache_key(path: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    pub fn is_supported(mime_type: &str) -> bool {
        SUPPORTED_IMAGE_TYPES.contains(&mime_type)
    }

    pub async fn get_or_generate(
        &self,
        path: &str,
        mime_type: &str,
        content: Bytes,
    ) -> (&'static str, Bytes) {
        let key = Self::cache_key(path);
        let cache_file = self.cache_path(&key);

        if let Ok(data) = tokio::fs::read(&cache_file).await {
            debug!("Thumbnail cache hit: {}", path);
            return ("image/jpeg", Bytes::from(data));
        }

        let result = if Self::is_supported(mime_type) {
            self.generate_image_thumbnail(&content).await
        } else {
            Ok(Bytes::from_static(FILE_ICON_SVG))
        };

        match result {
            Ok(data) => {
                let cache_path = cache_file.clone();
                let data_clone = data.clone();
                tokio::spawn(async move {
                    let _ = tokio::fs::write(&cache_path, &data_clone).await;
                });
                if Self::is_supported(mime_type) {
                    ("image/jpeg", data)
                } else {
                    ("image/svg+xml", data)
                }
            }
            Err(e) => {
                warn!("Thumbnail generation failed for {}: {}", path, e);
                ("image/svg+xml", Bytes::from_static(FILE_ICON_SVG))
            }
        }
    }

    async fn generate_image_thumbnail(&self, content: &Bytes) -> Result<Bytes, String> {
        let content = content.clone();
        let max_size = self.max_size;
        tokio::task::spawn_blocking(move || {
            let img = image::load_from_memory(&content)
                .map_err(|e| format!("Failed to load image: {}", e))?;

            let thumbnail = img.thumbnail(max_size, max_size);

            let mut buf = Vec::with_capacity(64 * 1024);
            let mut cursor = std::io::Cursor::new(&mut buf);
            thumbnail
                .write_to(&mut cursor, image::ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to encode JPEG: {}", e))?;

            Ok(Bytes::from(buf))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    pub async fn purge(&self, path: &str) {
        let key = Self::cache_key(path);
        let cache_file = self.cache_path(&key);
        tokio::fs::remove_file(cache_file).await.ok();
    }
}

pub async fn get_thumbnail(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let meta = match state.storage.head(&path).await {
        Ok(m) => m,
        Err(_) => return (StatusCode::NOT_FOUND, "File not found").into_response(),
    };

    if meta.is_collection {
        return (StatusCode::BAD_REQUEST, "Cannot thumbnail a collection").into_response();
    }

    let content = match state.storage.get(&path).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, "File not found").into_response(),
    };

    let mime = if meta.mime_type.is_empty() {
        "application/octet-stream"
    } else {
        &meta.mime_type
    };
    let data_dir = state.data_dir.as_deref().unwrap_or("/tmp/ferro");
    let service = ThumbnailService::new(data_dir, state.thumbnail_size);

    let (content_type, data) = service.get_or_generate(&path, mime, content).await;

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        data,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported() {
        assert!(ThumbnailService::is_supported("image/jpeg"));
        assert!(ThumbnailService::is_supported("image/png"));
        assert!(ThumbnailService::is_supported("image/gif"));
        assert!(ThumbnailService::is_supported("image/webp"));
        assert!(!ThumbnailService::is_supported("application/pdf"));
        assert!(!ThumbnailService::is_supported("text/plain"));
    }

    #[test]
    fn test_cache_key_deterministic() {
        let key1 = ThumbnailService::cache_key("/photos/cat.jpg");
        let key2 = ThumbnailService::cache_key("/photos/cat.jpg");
        let key3 = ThumbnailService::cache_key("/photos/dog.jpg");
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_generate_jpeg_thumbnail() {
        let service = ThumbnailService::new("/tmp/ferro-thumb-test", 128);
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            10,
            10,
            image::Rgb([255, 0, 0]),
        ));
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Jpeg).unwrap();
        let content = Bytes::from(buf);

        let (mime, thumb) = service
            .get_or_generate("/test.jpg", "image/jpeg", content)
            .await;
        assert_eq!(mime, "image/jpeg");
        assert!(thumb.len() > 0);

        let _ = tokio::fs::remove_dir_all("/tmp/ferro-thumb-test").await;
    }

    #[tokio::test]
    async fn test_non_image_returns_icon() {
        let service = ThumbnailService::new("/tmp/ferro-thumb-test2", 128);
        let content = Bytes::from("not an image");

        let (mime, thumb) = service
            .get_or_generate("/test.txt", "text/plain", content)
            .await;
        assert_eq!(mime, "image/svg+xml");
        assert!(thumb.len() > 0);

        let _ = tokio::fs::remove_dir_all("/tmp/ferro-thumb-test2").await;
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let service = ThumbnailService::new("/tmp/ferro-thumb-test3", 64);
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            5,
            5,
            image::Rgb([0, 0, 255]),
        ));
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
        let content = Bytes::from(buf);

        let (_, t1) = service
            .get_or_generate("/cached.png", "image/png", content.clone())
            .await;
        let (_, t2) = service
            .get_or_generate("/cached.png", "image/png", content)
            .await;
        assert_eq!(t1, t2);

        let _ = tokio::fs::remove_dir_all("/tmp/ferro-thumb-test3").await;
    }
}

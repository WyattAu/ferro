use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::path::{Path as StdPath, PathBuf};
use tracing::{debug, warn};

use crate::StorageUtilsState;

const SUPPORTED_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

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
        hex::encode(result)
    }

    pub fn is_supported(mime_type: &str) -> bool {
        SUPPORTED_IMAGE_TYPES.contains(&mime_type) || mime_type == "application/pdf"
    }

    pub async fn get_or_generate(&self, path: &str, mime_type: &str, content: Bytes) -> (&'static str, Bytes) {
        let key = Self::cache_key(path);
        let cache_file = self.cache_path(&key);

        let content_type = if mime_type == "application/pdf" {
            "image/svg+xml"
        } else if Self::is_supported(mime_type) {
            "image/jpeg"
        } else {
            "image/svg+xml"
        };

        if let Ok(data) = tokio::fs::read(&cache_file).await {
            debug!("Thumbnail cache hit: {}", path);
            return (content_type, Bytes::from(data));
        }

        let result = if mime_type == "application/pdf" {
            self.generate_pdf_thumbnail(&content).await
        } else if Self::is_supported(mime_type) {
            self.generate_image_thumbnail(&content).await
        } else {
            Ok(Bytes::from_static(FILE_ICON_SVG))
        };

        match result {
            Ok(data) => {
                let cache_path = cache_file.clone();
                let data_clone = data.clone();
                tokio::spawn(async move {
                    if let Err(e) = ferro_core::fs_util::atomic_write_async(cache_path, data_clone.to_vec()).await {
                        tracing::error!("Background thumbnail write failed: {e}");
                    }
                });
                (content_type, data)
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
            let img = image::load_from_memory(&content).map_err(|e| format!("Failed to load image: {}", e))?;

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

    async fn generate_pdf_thumbnail(&self, content: &Bytes) -> Result<Bytes, String> {
        let content = content.clone();
        tokio::task::spawn_blocking(move || {
            let file = pdf::file::FileOptions::cached()
                .load(content.to_vec())
                .map_err(|e| format!("Failed to parse PDF: {}", e))?;
            let pages = file.num_pages();

            let title = file
                .trailer
                .info_dict
                .as_ref()
                .and_then(|info| info.title.as_ref())
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "PDF Document".to_string());

            let creation_date = file
                .trailer
                .info_dict
                .as_ref()
                .and_then(|info| info.creation_date.as_ref())
                .map(|d| format!("{}-{:02}-{:02}", d.year, d.month, d.day));

            let file_size = content.len();
            let size_str = if file_size > 1_048_576 {
                format!("{:.1} MB", file_size as f64 / 1_048_576.0)
            } else {
                format!("{} KB", file_size / 1024)
            };

            let display_title = if title.len() > 30 {
                format!("{}...", &title[..27])
            } else {
                title.clone()
            };

            let date_str = creation_date
                .as_deref()
                .unwrap_or("");

            let svg = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
                <rect width="256" height="256" rx="12" fill="#fee2e2"/>
                <rect x="40" y="30" width="176" height="196" rx="4" fill="#ffffff" stroke="#ef4444" stroke-width="2"/>
                <rect x="56" y="46" width="144" height="10" rx="2" fill="#fca5a5"/>
                <rect x="56" y="66" width="120" height="10" rx="2" fill="#fecaca"/>
                <rect x="56" y="86" width="130" height="10" rx="2" fill="#fecaca"/>
                <rect x="56" y="106" width="100" height="10" rx="2" fill="#fecaca"/>
                <text x="128" y="175" text-anchor="middle" font-family="system-ui" font-size="16" font-weight="bold" fill="#dc2626">{}</text>
                <text x="128" y="195" text-anchor="middle" font-family="system-ui" font-size="11" fill="#b91c1c">{} pages · {}</text>
                <text x="128" y="212" text-anchor="middle" font-family="system-ui" font-size="10" fill="#9ca3af">{}</text>
            </svg>"##,
                display_title,
                pages,
                size_str,
                if !date_str.is_empty() {
                    format!("{} · {}", title, date_str)
                } else {
                    title
                },
            );
            Ok(Bytes::from(svg))
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

pub async fn get_thumbnail<S: StorageUtilsState>(
    State(state): State<S>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let meta = match state.storage().head(&path).await {
        Ok(m) => m,
        Err(_) => return (StatusCode::NOT_FOUND, "File not found").into_response(),
    };

    if meta.is_collection {
        return (StatusCode::BAD_REQUEST, "Cannot thumbnail a collection").into_response();
    }

    let mime = if meta.mime_type.is_empty() {
        "application/octet-stream"
    } else {
        &meta.mime_type
    };

    if let Some((data, cached_mime)) = state.thumbnail_cache().get(&path) {
        debug!("Thumbnail cache hit (LRU): {}", path);
        let content_type = if cached_mime == "image/svg+xml" {
            "image/svg+xml"
        } else {
            "image/jpeg"
        };
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, "public, max-age=86400"),
            ],
            Bytes::from(data),
        )
            .into_response();
    }

    let content = match state.storage().get(&path).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, "File not found").into_response(),
    };

    let data_dir = state.data_dir().unwrap_or("/tmp/ferro");
    let service = ThumbnailService::new(data_dir, state.thumbnail_size());

    let (content_type, data) = service.get_or_generate(&path, mime, content).await;

    state.thumbnail_cache().put(&path, data.to_vec(), content_type);

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
        assert!(ThumbnailService::is_supported("application/pdf"));
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
        assert!(!thumb.is_empty());

        let _ = tokio::fs::remove_dir_all("/tmp/ferro-thumb-test").await;
    }

    #[tokio::test]
    async fn test_non_image_returns_icon() {
        let _ = tokio::fs::remove_dir_all("/tmp/ferro-thumb-test2").await;
        let service = ThumbnailService::new("/tmp/ferro-thumb-test2", 128);
        let content = Bytes::from("not an image");

        let (mime, thumb) = service
            .get_or_generate("/test.txt", "text/plain", content)
            .await;
        assert_eq!(mime, "image/svg+xml");
        assert!(!thumb.is_empty());

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

    #[tokio::test]
    async fn test_generate_pdf_thumbnail() {
        let service = ThumbnailService::new("/tmp/ferro-thumb-test4", 128);
        let minimal_pdf = Bytes::from_static(
            br#"%PDF-1.0
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>
endobj
xref
0 4
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
trailer
<< /Size 4 /Root 1 0 R >>
startxref
186
%%EOF"#,
        );

        let (mime, thumb) = service
            .get_or_generate("/test.pdf", "application/pdf", minimal_pdf)
            .await;
        assert_eq!(mime, "image/svg+xml");
        let svg = String::from_utf8(thumb.to_vec()).unwrap();
        assert!(svg.contains("PDF"), "SVG should contain 'PDF': {}", svg);
        assert!(
            svg.contains("1 pages"),
            "SVG should contain '1 pages': {}",
            svg
        );
        assert!(svg.contains("KB"), "SVG should contain file size: {}", svg);

        let _ = tokio::fs::remove_dir_all("/tmp/ferro-thumb-test4").await;
    }
}

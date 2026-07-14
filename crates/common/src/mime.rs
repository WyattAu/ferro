/// Detect MIME type from the first bytes of content using magic bytes.
#[must_use]
pub fn sniff_content_type(data: &[u8], path: &str) -> String {
    if let Some(mime) = mime_guess::from_path(path).first() {
        let mime_str = mime.essence_str();
        if mime_str != "application/octet-stream" {
            return mime_str.to_string();
        }
    }

    if data.len() >= 4 {
        match &data[..4] {
            b"%PDF" => return "application/pdf".to_string(),
            b"\x89PNG" => return "image/png".to_string(),
            b"GIF8" => return "image/gif".to_string(),
            _ => {}
        }
    }
    if data.len() >= 3 && &data[..3] == b"\xff\xd8\xff" {
        return "image/jpeg".to_string();
    }
    if data.len() >= 5 && &data[..5] == b"<?xml" {
        return "application/xml".to_string();
    }
    if data.len() >= 2 && &data[..2] == b"PK" {
        return "application/zip".to_string();
    }
    if data.len() >= 6 && &data[..6] == b"Rar!\x1a\x07" {
        return "application/vnd.rar".to_string();
    }
    if data.len() >= 4 && &data[..4] == b"OggS" {
        return "audio/ogg".to_string();
    }
    if data.len() >= 12 && &data[8..12] == b"WEBP" {
        return "image/webp".to_string();
    }
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        return "video/mp4".to_string();
    }

    "application/octet-stream".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sniff_empty() {
        assert_eq!(sniff_content_type(&[], "test.txt"), "text/plain");
    }

    #[test]
    fn test_sniff_pdf() {
        assert_eq!(sniff_content_type(b"%PDF-1.4", "doc.pdf"), "application/pdf");
    }

    #[test]
    fn test_sniff_png() {
        assert_eq!(sniff_content_type(b"\x89PNG\r\n\x1a\n", "img.png"), "image/png");
    }

    #[test]
    fn test_sniff_gif() {
        assert_eq!(sniff_content_type(b"GIF89a", "img.gif"), "image/gif");
    }

    #[test]
    fn test_sniff_jpeg() {
        assert_eq!(sniff_content_type(b"\xff\xd8\xff\xe0", "img.jpg"), "image/jpeg");
    }

    #[test]
    fn test_sniff_xml() {
        assert_eq!(sniff_content_type(b"<?xml version=\"1.0\"?>", "data.xml"), "text/xml");
    }

    #[test]
    fn test_sniff_zip() {
        assert_eq!(sniff_content_type(b"PK\x03\x04", "archive.zip"), "application/zip");
    }

    #[test]
    fn test_sniff_rar() {
        assert_eq!(
            sniff_content_type(b"Rar!\x1a\x07\x00", "archive.rar"),
            "application/x-rar-compressed"
        );
    }

    #[test]
    fn test_sniff_ogg() {
        assert_eq!(sniff_content_type(b"OggS\x00\x02", "audio.ogg"), "audio/ogg");
    }

    #[test]
    fn test_sniff_webp() {
        let mut data = vec![0u8; 12];
        data[8..12].copy_from_slice(b"WEBP");
        assert_eq!(sniff_content_type(&data, "img.webp"), "image/webp");
    }

    #[test]
    fn test_sniff_mp4() {
        let mut data = vec![0u8; 8];
        data[4..8].copy_from_slice(b"ftyp");
        assert_eq!(sniff_content_type(&data, "video.mp4"), "video/mp4");
    }

    #[test]
    fn test_sniff_fallback() {
        assert_eq!(
            sniff_content_type(b"\x00\x00\x00\x00", "unknown"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_sniff_unknown_binary() {
        assert_eq!(
            sniff_content_type(b"\x01\x02\x03\x04", "data.bin"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_sniff_html_by_extension() {
        assert_eq!(sniff_content_type(b"", "index.html"), "text/html");
    }

    #[test]
    fn test_sniff_css_by_extension() {
        assert_eq!(sniff_content_type(b"", "style.css"), "text/css");
    }

    #[test]
    fn test_sniff_js_by_extension() {
        assert_eq!(sniff_content_type(b"", "app.js"), "text/javascript");
    }

    #[test]
    fn test_sniff_json_by_extension() {
        assert_eq!(sniff_content_type(b"", "data.json"), "application/json");
    }

    #[test]
    fn test_sniff_png_by_extension() {
        assert_eq!(sniff_content_type(b"", "image.png"), "image/png");
    }

    #[test]
    fn test_sniff_jpg_by_extension() {
        assert_eq!(sniff_content_type(b"", "photo.jpg"), "image/jpeg");
    }

    #[test]
    fn test_sniff_gif_by_extension() {
        assert_eq!(sniff_content_type(b"", "animation.gif"), "image/gif");
    }

    #[test]
    fn test_sniff_mp3_by_extension() {
        assert_eq!(sniff_content_type(b"", "song.mp3"), "audio/mpeg");
    }

    #[test]
    fn test_sniff_wav_by_extension() {
        assert_eq!(sniff_content_type(b"", "sound.wav"), "audio/wav");
    }

    #[test]
    fn test_sniff_svg_by_extension() {
        assert_eq!(sniff_content_type(b"", "icon.svg"), "image/svg+xml");
    }

    #[test]
    fn test_sniff_tiff_by_extension() {
        assert_eq!(sniff_content_type(b"", "photo.tiff"), "image/tiff");
    }

    #[test]
    fn test_sniff_webm_by_extension() {
        assert_eq!(sniff_content_type(b"", "video.webm"), "video/webm");
    }

    #[test]
    fn test_sniff_mkv_by_extension() {
        assert_eq!(sniff_content_type(b"", "video.mkv"), "video/x-matroska");
    }

    #[test]
    fn test_sniff_avi_by_extension() {
        assert_eq!(sniff_content_type(b"", "video.avi"), "video/x-msvideo");
    }

    #[test]
    fn test_sniff_mov_by_extension() {
        assert_eq!(sniff_content_type(b"", "video.mov"), "video/quicktime");
    }

    #[test]
    fn test_sniff_txt_by_extension() {
        assert_eq!(sniff_content_type(b"", "readme.txt"), "text/plain");
    }

    #[test]
    fn test_sniff_md_by_extension() {
        assert_eq!(sniff_content_type(b"", "README.md"), "text/markdown");
    }

    #[test]
    fn test_sniff_csv_by_extension() {
        assert_eq!(sniff_content_type(b"", "data.csv"), "text/csv");
    }

    #[test]
    fn test_sniff_xml_by_extension() {
        assert_eq!(sniff_content_type(b"", "data.xml"), "text/xml");
    }

    #[test]
    fn test_sniff_pdf_by_extension() {
        assert_eq!(sniff_content_type(b"", "doc.pdf"), "application/pdf");
    }

    #[test]
    fn test_sniff_zip_by_extension() {
        assert_eq!(sniff_content_type(b"", "archive.zip"), "application/zip");
    }

    #[test]
    fn test_sniff_gz_by_extension() {
        assert_eq!(sniff_content_type(b"", "archive.gz"), "application/gzip");
    }

    #[test]
    fn test_sniff_tar_by_extension() {
        assert_eq!(sniff_content_type(b"", "archive.tar"), "application/x-tar");
    }

    #[test]
    fn test_sniff_exe_by_extension() {
        let result = sniff_content_type(b"", "program.exe");
        assert!(!result.is_empty());
    }
}

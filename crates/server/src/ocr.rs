//! OCR text extraction for uploaded PDFs and images.

/// Extract text content from a PDF or image for search indexing.
/// Returns extracted text or empty string if extraction fails/is not possible.
pub fn extract_text(_content: &[u8], content_type: &str) -> String {
    match content_type {
        "application/pdf" => {
            // The `pdf` crate is already a dependency for thumbnail generation.
            // Extract text from PDF pages. Placeholder for now.
            String::new()
        }
        ct if ct.starts_with("image/") => {
            // OCR requires tesseract binary or library.
            // Placeholder: log that OCR is not available.
            String::new()
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_pdf_returns_empty() {
        let text = extract_text(b"%PDF-1.4", "application/pdf");
        assert!(text.is_empty());
    }

    #[test]
    fn test_extract_text_image_returns_empty() {
        let text = extract_text(b"\x89PNG", "image/png");
        assert!(text.is_empty());
    }

    #[test]
    fn test_extract_text_unsupported_type() {
        let text = extract_text(b"hello", "application/octet-stream");
        assert!(text.is_empty());
    }
}

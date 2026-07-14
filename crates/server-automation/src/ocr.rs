use tracing::debug;

pub fn extract_text(content: &[u8], content_type: &str) -> String {
    match content_type {
        "application/pdf" => extract_pdf_text(content),
        ct if ct.starts_with("image/") => {
            debug!("Image OCR not available for {}", ct);
            String::new()
        }
        _ => String::new(),
    }
}

fn extract_pdf_text(content: &[u8]) -> String {
    let file = match pdf::file::FileOptions::cached().load(content.to_vec()) {
        Ok(f) => f,
        Err(e) => {
            debug!("Failed to parse PDF for OCR: {}", e);
            return String::new();
        }
    };

    let resolver = file.resolver();
    let num_pages = file.num_pages();
    let mut all_text = String::new();

    for page_idx in 0..num_pages {
        let page = match file.get_page(page_idx) {
            Ok(p) => p,
            Err(e) => {
                debug!("Failed to get PDF page {}: {}", page_idx, e);
                continue;
            }
        };

        if let Some(ref content) = page.contents {
            match content.operations(&resolver) {
                Ok(ops) => {
                    let mut page_text = String::new();
                    for op in &ops {
                        match op {
                            pdf::content::Op::TextDraw { text } => {
                                let s = text.to_string_lossy();
                                if !s.is_empty() {
                                    if !page_text.is_empty() {
                                        page_text.push(' ');
                                    }
                                    page_text.push_str(&s);
                                }
                            }
                            pdf::content::Op::TextDrawAdjusted { array } => {
                                for item in array {
                                    if let pdf::content::TextDrawAdjusted::Text(text) = item {
                                        let s = text.to_string_lossy();
                                        if !s.is_empty() {
                                            if !page_text.is_empty() {
                                                page_text.push(' ');
                                            }
                                            page_text.push_str(&s);
                                        }
                                    }
                                }
                            }
                            pdf::content::Op::TextNewline => {
                                page_text.push('\n');
                            }
                            _ => {}
                        }
                    }
                    if !page_text.is_empty() {
                        if !all_text.is_empty() {
                            all_text.push('\n');
                        }
                        all_text.push_str(page_text.trim());
                    }
                }
                Err(e) => {
                    debug!("Failed to parse content stream for page {}: {}", page_idx, e);
                }
            }
        }
    }

    all_text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_pdf_invalid_returns_empty() {
        let text = extract_text(b"%PDF-1.4 invalid", "application/pdf");
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

    #[test]
    fn test_extract_text_empty_pdf() {
        let text = extract_text(b"", "application/pdf");
        assert!(text.is_empty());
    }
}

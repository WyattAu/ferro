//! Search indexer that runs asynchronously.
//!
//! Design decision: Search index failures are logged but never propagated to the caller.
//! This ensures that file operations (PUT, DELETE, etc.) never fail due to search issues.
//! The index will eventually heal itself on the next successful operation.

use crate::AppState;
use common::metadata::FileMetadata;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};

/// Index a single file's metadata immediately after PUT/create.
/// This is called by the WebDAV handler on successful write operations.
pub async fn index_file(state: &AppState, metadata: &FileMetadata) {
    let Some(search_lock) = &state.search else {
        return;
    };

    if metadata.is_collection {
        return; // Don't index collections
    }

    let mut engine = search_lock.write().await;
    if let Err(e) = engine.index_metadata(metadata) {
        warn!("Auto-index: failed to index {}: {}", metadata.path, e);
        return;
    }
    if let Err(e) = engine.commit() {
        warn!("Auto-index: failed to commit after indexing {}: {}", metadata.path, e);
        return;
    }
    debug!("Auto-indexed: {}", metadata.path);
}

/// Index a single file with content immediately after PUT.
/// For text files, we index the content; for binary, just metadata.
/// When dedup is enabled, computes perceptual/content hash.
/// For PDFs/images, attempts OCR text extraction for search indexing.
/// Extract searchable text from an XLSX workbook: sheet names plus the text
/// form of every string/numeric/boolean cell. Returns `None` when the bytes
/// are not a parseable workbook.
#[must_use]
pub fn extract_xlsx_text(content: &[u8]) -> Option<String> {
    let wb = sheet_xlsx::read_xlsx(content).ok()?;
    let mut text = String::new();
    for sheet in &wb.sheets {
        text.push_str(&sheet.name);
        text.push('\n');
        for cell in sheet.cells.values() {
            let line = match &cell.value {
                sheet_xlsx::XlsxValue::InlineString(s) => Some(s.clone()),
                sheet_xlsx::XlsxValue::SharedString(idx) => wb.shared_text(*idx).map(str::to_string),
                sheet_xlsx::XlsxValue::Number(n) => Some(n.to_string()),
                sheet_xlsx::XlsxValue::Boolean(b) => Some(b.to_string()),
                sheet_xlsx::XlsxValue::FormulaString(s) if !s.is_empty() => Some(s.clone()),
                _ => None,
            };
            if let Some(line) = line {
                text.push_str(&line);
                text.push('\n');
            }
        }
    }
    if text.is_empty() { None } else { Some(text) }
}

/// Extract searchable text from ID3v2 tags (MP3): title, artist, album,
/// genre, year. Returns `None` when the bytes carry no parseable ID3 tag
/// (R7 estate dogfood: id3-core; mirrors the XLSX path).
#[must_use]
pub fn extract_id3_text(content: &[u8]) -> Option<String> {
    let tag = id3_core::Tag::parse(content).ok()?;
    let mut parts: Vec<String> = Vec::new();
    for field in [
        id3_core::TextField::Title,
        id3_core::TextField::Artist,
        id3_core::TextField::Album,
        id3_core::TextField::Genre,
        id3_core::TextField::Year,
    ] {
        if let Some(v) = tag.text_field(field) {
            let v = v.trim();
            if !v.is_empty() {
                parts.push(v.to_string());
            }
        }
    }
    if parts.is_empty() { None } else { Some(parts.join("\n")) }
}

/// Extract searchable text from MP4/M4A/M4B iTunes metadata (ilst):
/// title, artist, album (R7 estate dogfood: mp4-core).
#[must_use]
pub fn extract_mp4_text(content: &[u8]) -> Option<String> {
    let top = mp4_core::container::parse_boxes(content).ok()?;
    let moov = top.iter().find(|b| b.box_type == *b"moov")?;
    let udta = moov.children.iter().find(|b| b.box_type == *b"udta")?;
    let meta = udta.children.iter().find(|b| b.box_type == *b"meta")?;
    let md = mp4_core::metadata::parse_meta(meta);
    let md = md?;
    let mut parts: Vec<String> = Vec::new();
    for key in [
        mp4_core::metadata::atoms::TITLE,
        mp4_core::metadata::atoms::ARTIST,
        mp4_core::metadata::atoms::ALBUM,
    ] {
        if let Some(v) = md.text(key) {
            let v = v.trim();
            if !v.is_empty() {
                parts.push(v.to_string());
            }
        }
    }
    if parts.is_empty() { None } else { Some(parts.join("\n")) }
}

pub async fn index_file_with_content(state: &AppState, metadata: &FileMetadata, content: &[u8]) {
    let Some(search_lock) = &state.search else {
        return;
    };

    if metadata.is_collection {
        return;
    }

    let mut engine = search_lock.write().await;

    let mut content_to_index: Option<&str> = None;
    let mut content_extracted = false;

    // Only index content for text-like MIME types
    let is_text = metadata.mime_type.starts_with("text/")
        || metadata.mime_type == "application/json"
        || metadata.mime_type == "application/xml"
        || metadata.mime_type == "application/javascript"
        || metadata.mime_type == "application/x-yaml";

    if is_text {
        if let Ok(content_str) = std::str::from_utf8(content) {
            let truncated = &content_str[..content_str.len().min(1_000_000)];
            if let Err(e) = engine.index_content(metadata, truncated) {
                warn!("Auto-index: failed to index content for {}: {}", metadata.path, e);
                return;
            }
            content_to_index = Some(truncated);
        } else if let Err(e) = engine.index_metadata(metadata) {
            warn!("Auto-index: failed to index {}: {}", metadata.path, e);
            return;
        }
    } else if let Err(e) = engine.index_metadata(metadata) {
        warn!("Auto-index: failed to index {}: {}", metadata.path, e);
        return;
    }

    // XLSX text extraction (R5-D, sheet-xlsx): index cell text of
    // spreadsheets so they are searchable like text documents.
    if content_to_index.is_none()
        && (metadata.mime_type == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            || metadata.path.to_lowercase().ends_with(".xlsx"))
        && let Some(text) = extract_xlsx_text(content)
    {
        let truncated = &text[..text.len().min(1_000_000)];
        if let Err(e) = engine.index_content(metadata, truncated) {
            warn!(error = %e, path = %metadata.path, "XLSX content indexing failed");
        } else {
            content_extracted = true;
        }
    }

    // ID3 metadata extraction for MP3s (R7, id3-core): tags make audio
    // files searchable by title/artist/album like text documents.
    if content_to_index.is_none()
        && (metadata.mime_type == "audio/mpeg" || metadata.path.to_lowercase().ends_with(".mp3"))
        && let Some(text) = extract_id3_text(content)
    {
        let truncated = &text[..text.len().min(1_000_000)];
        if let Err(e) = engine.index_content(metadata, truncated) {
            warn!(error = %e, path = %metadata.path, "ID3 content indexing failed");
        } else {
            content_extracted = true;
        }
    }

    // iTunes metadata extraction for MP4/M4A/M4B (R7, mp4-core).
    if content_to_index.is_none()
        && (matches!(
            metadata.mime_type.as_str(),
            "audio/mp4" | "audio/x-m4a" | "audio/x-m4b" | "video/mp4"
        ) || metadata.path.to_lowercase().ends_with(".m4a")
            || metadata.path.to_lowercase().ends_with(".m4b"))
        && let Some(text) = extract_mp4_text(content)
    {
        let truncated = &text[..text.len().min(1_000_000)];
        if let Err(e) = engine.index_content(metadata, truncated) {
            warn!(error = %e, path = %metadata.path, "MP4 content indexing failed");
        } else {
            content_extracted = true;
        }
    }

    // OCR text extraction for PDFs and images
    if !content_extracted
        && content_to_index.is_none()
        && (metadata.mime_type == "application/pdf" || metadata.mime_type.starts_with("image/"))
    {
        let ocr_text = crate::ocr::extract_text(content, &metadata.mime_type);
        if !ocr_text.is_empty() {
            let truncated = &ocr_text[..ocr_text.len().min(1_000_000)];
            if let Err(e) = engine.index_content(metadata, truncated) {
                warn!(error = %e, path = %metadata.path, "OCR content indexing failed");
            }
        }
    }

    // Smart deduplication: compute perceptual/content hash
    if state.dedup_enabled {
        let dedup_hash = crate::dedup::compute_dedup_hash(content, &metadata.mime_type);
        debug!(
            "Dedup hash for {}: {}",
            metadata.path,
            &dedup_hash[..dedup_hash.len().min(16)]
        );
    }

    if let Err(e) = engine.commit() {
        warn!("Auto-index: failed to commit after indexing {}: {}", metadata.path, e);
        return;
    }
    debug!("Auto-indexed with content: {}", metadata.path);
}

/// Remove a file from the search index immediately after DELETE.
pub async fn remove_file(state: &AppState, path: &str) {
    let Some(search_lock) = &state.search else {
        return;
    };

    let mut engine = search_lock.write().await;
    if let Err(e) = engine.remove(path) {
        warn!("Auto-index: failed to remove {}: {}", path, e);
        return;
    }
    if let Err(e) = engine.commit() {
        warn!("Auto-index: failed to commit after removing {}: {}", path, e);
        return;
    }
    debug!("Auto-removed from index: {}", path);
}

pub fn spawn_indexer(state: Arc<AppState>, interval_secs: u64, cancel: tokio_util::sync::CancellationToken) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(interval_secs));

        tokio::time::sleep(Duration::from_secs(5)).await;
        if !cancel.is_cancelled() {
            index_storage(&state).await;
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !cancel.is_cancelled() {
                        index_storage(&state).await;
                    }
                }
                _ = cancel.cancelled() => {
                    tracing::info!("Background indexer shutting down");
                    break;
                }
            }
        }
    });

    info!("Background indexer started (interval: {}s)", interval_secs);
}

async fn index_storage(state: &AppState) {
    let Some(search_lock) = &state.search else {
        return;
    };

    let entries = match state.storage.list_all("/", 100).await {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Indexer: failed to list storage: {}", e);
            return;
        }
    };

    let mut engine = search_lock.write().await;

    for meta in &entries {
        if let Err(e) = engine.index_metadata(meta) {
            debug!("Indexer: failed to index {}: {}", meta.path, e);
        }
    }

    if let Err(e) = engine.commit() {
        warn!("Indexer: failed to commit: {}", e);
    }

    debug!("Indexer: processed {} entries", entries.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_xlsx_text_roundtrip() {
        let mut wb = sheet_xlsx::XlsxWorkbook::single_sheet();
        wb.sheets[0].name = "Budget".into();
        wb.sheets[0].cells.insert(
            (0, 0),
            sheet_xlsx::XlsxCell::new(sheet_xlsx::XlsxValue::InlineString("quarterly revenue".into())),
        );
        wb.sheets[0].cells.insert(
            (1, 0),
            sheet_xlsx::XlsxCell::new(sheet_xlsx::XlsxValue::Number(42250.0)),
        );
        let bytes = sheet_xlsx::write_xlsx(&wb).unwrap();

        let text = extract_xlsx_text(&bytes).expect("text");
        assert!(text.contains("Budget"));
        assert!(text.contains("quarterly revenue"));
        assert!(text.contains("42250"));
    }

    #[test]
    fn test_extract_xlsx_rejects_non_xlsx() {
        assert!(extract_xlsx_text(b"not a workbook").is_none());
        assert!(extract_xlsx_text(b"").is_none());
    }
}

#[cfg(test)]
mod id3_tests {
    use super::*;

    #[test]
    fn test_extract_id3_roundtrip() {
        let mut tag = id3_core::Tag::new();
        tag.set_text(id3_core::TextField::Title, id3_core::TextEncoding::Utf8, "Dune");
        tag.set_text(
            id3_core::TextField::Artist,
            id3_core::TextEncoding::Utf8,
            "The Sisterhood",
        );
        tag.set_text(id3_core::TextField::Album, id3_core::TextEncoding::Utf8, "Soundtracks");
        let bytes = tag.to_bytes().unwrap();
        let text = extract_id3_text(&bytes).expect("text");
        assert!(text.contains("Dune"));
        assert!(text.contains("The Sisterhood"));
        assert!(text.contains("Soundtracks"));
    }

    #[test]
    fn test_extract_id3_rejects_non_id3() {
        assert!(extract_id3_text(b"not an mp3").is_none());
        assert!(extract_id3_text(b"").is_none());
    }
}

#[cfg(test)]
mod mp4_tests {
    use super::*;

    fn box_bytes(t: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(t);
        v.extend_from_slice(body);
        v
    }

    fn ilst_text_atom(name: &[u8; 4], value: &str) -> Vec<u8> {
        // data box payload: version(1) + flags(3) + type flag(4; 1 =
        // UTF-8, at offset 4) + locale(4) + text.
        let mut data_body = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0];
        data_body.extend_from_slice(value.as_bytes());
        box_bytes(name, &box_bytes(b"data", &data_body))
    }

    #[test]
    fn test_extract_mp4_roundtrip() {
        let ilst_body = [
            ilst_text_atom(b"\xa9nam", "Neuromancer"),
            ilst_text_atom(b"\xa9ART", "William Gibson"),
        ]
        .concat();
        // meta carries a 4-byte version/flags prefix before its children.
        let mut meta_body = vec![0, 0, 0, 0];
        meta_body.extend_from_slice(&box_bytes(b"ilst", &ilst_body));
        let moov = box_bytes(b"moov", &box_bytes(b"udta", &box_bytes(b"meta", &meta_body)));

        let text = extract_mp4_text(&moov).expect("text");
        assert!(text.contains("Neuromancer"));
        assert!(text.contains("William Gibson"));
    }

    #[test]
    fn test_extract_mp4_rejects_non_mp4() {
        assert!(extract_mp4_text(b"not an mp4").is_none());
        assert!(extract_mp4_text(b"").is_none());
    }
}

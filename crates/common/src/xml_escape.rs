use crate::simd::compare::contains_simd;
use std::borrow::Cow;

/// Escape special XML characters in a string.
/// Returns a borrowed reference if no escaping is needed (zero-copy).
/// Uses SIMD acceleration on x86_64 for large ASCII strings.
#[must_use]
pub fn escape_xml(s: &str) -> Cow<'_, str> {
    if !needs_escaping(s) {
        return Cow::Borrowed(s);
    }
    let mut result = String::with_capacity(s.len() + s.len() / 4);
    for ch in s.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(ch),
        }
    }
    Cow::Owned(result)
}

/// Check if a string contains characters that need XML escaping.
/// Uses SIMD acceleration on x86_64 for large ASCII strings.
#[must_use]
fn needs_escaping(s: &str) -> bool {
    // For ASCII-only strings, use SIMD-accelerated byte search
    if s.is_ascii() {
        #[cfg(target_arch = "x86_64")]
        {
            contains_simd(s, "&")
                || contains_simd(s, "<")
                || contains_simd(s, ">")
                || contains_simd(s, "\"")
                || contains_simd(s, "'")
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            s.contains('&') || s.contains('<') || s.contains('>') || s.contains('"') || s.contains('\'')
        }
    } else {
        // For non-ASCII strings, use standard library (handles UTF-8 correctly)
        s.contains('&') || s.contains('<') || s.contains('>') || s.contains('"') || s.contains('\'')
    }
}

/// Unescape XML character entities back to their original characters.
/// Returns a borrowed reference if no unescaping is needed (zero-copy).
#[must_use]
pub fn unescape_xml(s: &str) -> Cow<'_, str> {
    if !s.contains("&amp;")
        && !s.contains("&lt;")
        && !s.contains("&gt;")
        && !s.contains("&quot;")
        && !s.contains("&apos;")
    {
        return Cow::Borrowed(s);
    }
    let mut result = String::with_capacity(s.len());
    let mut remaining = s;
    while let Some(pos) = remaining.find('&') {
        result.push_str(&remaining[..pos]);
        let entity = &remaining[pos..];
        if let Some(end) = entity.find(';') {
            let ent = &entity[..=end];
            match ent {
                "&amp;" => result.push('&'),
                "&lt;" => result.push('<'),
                "&gt;" => result.push('>'),
                "&quot;" => result.push('"'),
                "&apos;" => result.push('\''),
                _ => result.push_str(ent),
            }
            remaining = &entity[end + 1..];
        } else {
            result.push_str(entity);
            break;
        }
    }
    result.push_str(remaining);
    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_unescape_roundtrip() {
        let inputs = vec![
            "hello world",
            "<tag>value</tag>",
            "a & b",
            "\"quoted\"",
            "'single'",
            "",
            "&<>\"'",
        ];
        for input in inputs {
            let escaped = escape_xml(input);
            let unescaped = unescape_xml(&escaped);
            assert_eq!(input, unescaped.as_ref(), "roundtrip failed for {:?}", input);
        }
    }

    #[test]
    fn test_escaping_removes_dangerous_chars() {
        let escaped = escape_xml("<script>alert('xss')</script>");
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(!escaped.contains('\''));
    }

    #[test]
    fn test_escape_xml_zero_copy() {
        let safe = "hello world";
        let result = escape_xml(safe);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), safe);

        let needs_escaping = "a & b";
        let result = escape_xml(needs_escaping);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.as_ref(), "a &amp; b");
    }

    #[test]
    fn test_unescape_xml_zero_copy() {
        let safe = "hello world";
        let result = unescape_xml(safe);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), safe);

        let needs_unescaping = "a &amp; b";
        let result = unescape_xml(needs_unescaping);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.as_ref(), "a & b");
    }

    #[test]
    fn test_escape_xml_special_chars() {
        assert_eq!(escape_xml("<").as_ref(), "&lt;");
        assert_eq!(escape_xml(">").as_ref(), "&gt;");
        assert_eq!(escape_xml("&").as_ref(), "&amp;");
        assert_eq!(escape_xml("\"").as_ref(), "&quot;");
        assert_eq!(escape_xml("'").as_ref(), "&apos;");
    }

    #[test]
    fn test_unescape_xml_entities() {
        assert_eq!(unescape_xml("&lt;").as_ref(), "<");
        assert_eq!(unescape_xml("&gt;").as_ref(), ">");
        assert_eq!(unescape_xml("&amp;").as_ref(), "&");
        assert_eq!(unescape_xml("&quot;").as_ref(), "\"");
        assert_eq!(unescape_xml("&apos;").as_ref(), "'");
    }

    #[test]
    fn test_unescape_xml_unknown_entity() {
        let input = "&unknown;";
        let result = unescape_xml(input);
        assert_eq!(result.as_ref(), "&unknown;");
    }

    #[test]
    fn test_escape_xml_non_ascii() {
        let input = "Hello \u{00e9}m\u{00e9}di";
        let result = escape_xml(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    #[test]
    fn test_unescape_xml_multiple_entities() {
        let input = "&lt;div class=&quot;test&quot;&gt;Hello &amp; World&lt;/div&gt;";
        let result = unescape_xml(input);
        assert_eq!(result.as_ref(), "<div class=\"test\">Hello & World</div>");
    }

    #[test]
    fn test_escape_xml_empty() {
        let result = escape_xml("");
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), "");
    }

    #[test]
    fn test_unescape_xml_empty() {
        let result = unescape_xml("");
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), "");
    }

    #[test]
    fn test_unescape_xml_partial_entity() {
        let input = "&amp incomplete";
        let result = unescape_xml(input);
        assert_eq!(result.as_ref(), "&amp incomplete");
    }
}

pub mod device;
pub mod touch_gestures;

pub fn percent_encode(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => vec![c],
            _ => format!("%{:02X}", c as u32).chars().collect(),
        })
        .collect()
}

pub fn percent_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            result.push(byte);
            i += 3;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).unwrap_or_default()
}

pub fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_encode_safe_chars() {
        assert_eq!(percent_encode("abcABC123-_.~"), "abcABC123-_.~");
    }

    #[test]
    fn test_percent_encode_special_chars() {
        let encoded = percent_encode("hello world");
        assert_eq!(encoded, "hello%20world");

        let encoded = percent_encode("/");
        assert_eq!(encoded, "%2F");

        let encoded = percent_encode("a+b=c");
        assert_eq!(encoded, "a%2Bb%3Dc");
    }

    #[test]
    fn test_percent_encode_empty() {
        assert_eq!(percent_encode(""), "");
    }

    #[test]
    fn test_percent_decode_basic() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("%2Fpath"), "/path");
    }

    #[test]
    fn test_urlencoding_decode_plus() {
        assert_eq!(urlencoding_decode("a+b"), "a b");
    }

    #[test]
    fn test_urlencoding_decode_percent() {
        assert_eq!(urlencoding_decode("a%20b"), "a b");
    }
}

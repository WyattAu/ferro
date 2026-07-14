use std::cmp::Ordering;

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn strcmp_avx2_inner(a: &[u8], b: &[u8]) -> Ordering {
    use std::arch::x86_64::*;

    let mut i = 0;

    while i + 32 <= a.len() && i + 32 <= b.len() {
        unsafe {
            let a_chunk = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
            let b_chunk = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);

            let cmp = _mm256_cmpeq_epi8(a_chunk, b_chunk);
            let mask = _mm256_movemask_epi8(cmp) as u32;

            if mask != 0xFFFFFFFF {
                let pos = mask.trailing_ones() as usize;
                return a[i + pos].cmp(&b[i + pos]);
            }
        }
        i += 32;
    }

    while i < a.len() && i < b.len() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
        i += 1;
    }

    a.len().cmp(&b.len())
}

#[cfg(target_arch = "x86_64")]
pub fn strcmp_simd(a: &str, b: &str) -> Ordering {
    if super::is_avx2_available() {
        unsafe { strcmp_avx2_inner(a.as_bytes(), b.as_bytes()) }
    } else {
        a.as_bytes().cmp(b.as_bytes())
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn strcmp_simd(a: &str, b: &str) -> Ordering {
    a.as_bytes().cmp(b.as_bytes())
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn contains_avx2_inner(haystack: &[u8], needle: &[u8]) -> bool {
    use std::arch::x86_64::*;

    if needle.len() > haystack.len() {
        return false;
    }

    if needle.is_empty() {
        return true;
    }

    let first_char = _mm256_set1_epi8(needle[0] as i8);

    let mut i = 0;
    while i + 32 <= haystack.len() {
        unsafe {
            let chunk = _mm256_loadu_si256(haystack.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(chunk, first_char);
            let mask = _mm256_movemask_epi8(cmp) as u32;

            let mut pos = mask;
            while pos != 0 {
                let offset = pos.trailing_ones() as usize;
                let start = i + offset;

                if start + needle.len() <= haystack.len() && &haystack[start..start + needle.len()] == needle {
                    return true;
                }

                pos &= pos - 1;
            }
        }
        i += 32;
    }

    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            return true;
        }
        i += 1;
    }

    false
}

#[cfg(target_arch = "x86_64")]
pub fn contains_simd(haystack: &str, needle: &str) -> bool {
    if super::is_avx2_available() {
        unsafe { contains_avx2_inner(haystack.as_bytes(), needle.as_bytes()) }
    } else {
        haystack.contains(needle)
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn contains_simd(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn find_byte_avx2_inner(data: &[u8], needle: u8) -> Option<usize> {
    use std::arch::x86_64::*;

    let pattern = _mm256_set1_epi8(needle as i8);

    let mut i = 0;
    while i + 32 <= data.len() {
        unsafe {
            let chunk = _mm256_loadu_si256(data.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(chunk, pattern);
            let mask = _mm256_movemask_epi8(cmp) as u32;

            if mask != 0 {
                return Some(i + mask.trailing_ones() as usize);
            }
        }
        i += 32;
    }

    while i < data.len() {
        if data[i] == needle {
            return Some(i);
        }
        i += 1;
    }

    None
}

#[cfg(target_arch = "x86_64")]
pub fn find_byte_simd(data: &[u8], needle: u8) -> Option<usize> {
    if super::is_avx2_available() {
        unsafe { find_byte_avx2_inner(data, needle) }
    } else {
        data.iter().position(|&b| b == needle)
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn find_byte_simd(data: &[u8], needle: u8) -> Option<usize> {
    data.iter().position(|&b| b == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strcmp_equal() {
        assert_eq!(strcmp_simd("hello", "hello"), Ordering::Equal);
    }

    #[test]
    fn test_strcmp_less() {
        assert_eq!(strcmp_simd("abc", "abd"), Ordering::Less);
    }

    #[test]
    fn test_strcmp_greater() {
        assert_eq!(strcmp_simd("abd", "abc"), Ordering::Greater);
    }

    #[test]
    fn test_strcmp_prefix() {
        assert_eq!(strcmp_simd("abc", "abcd"), Ordering::Less);
    }

    #[test]
    fn test_strcmp_empty() {
        assert_eq!(strcmp_simd("", ""), Ordering::Equal);
        assert_eq!(strcmp_simd("", "a"), Ordering::Less);
        assert_eq!(strcmp_simd("a", ""), Ordering::Greater);
    }

    #[test]
    fn test_contains_found() {
        assert!(contains_simd("hello world", "world"));
    }

    #[test]
    fn test_contains_not_found() {
        assert!(!contains_simd("hello", "world"));
    }

    #[test]
    fn test_contains_empty_needle() {
        assert!(contains_simd("hello", ""));
    }

    #[test]
    fn test_contains_needle_longer() {
        assert!(!contains_simd("hi", "hello"));
    }

    #[test]
    fn test_find_byte_found() {
        assert_eq!(find_byte_simd(b"hello", b'l'), Some(2));
    }

    #[test]
    fn test_find_byte_not_found() {
        assert_eq!(find_byte_simd(b"hello", b'x'), None);
    }

    #[test]
    fn test_find_byte_empty() {
        assert_eq!(find_byte_simd(b"", b'a'), None);
    }
}

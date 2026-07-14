#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn memcpy_avx2_inner(dst: &mut [u8], src: &[u8]) {
    use std::arch::x86_64::*;

    assert_eq!(dst.len(), src.len());

    let mut i = 0;

    while i + 32 <= src.len() {
        unsafe {
            let chunk = _mm256_loadu_si256(src.as_ptr().add(i) as *const __m256i);
            _mm256_storeu_si256(dst.as_mut_ptr().add(i) as *mut __m256i, chunk);
        }
        i += 32;
    }

    while i < src.len() {
        dst[i] = src[i];
        i += 1;
    }
}

#[cfg(target_arch = "x86_64")]
pub fn memcpy_simd(dst: &mut [u8], src: &[u8]) {
    if super::is_avx2_available() {
        unsafe { memcpy_avx2_inner(dst, src) }
    } else {
        dst.copy_from_slice(src);
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn memcpy_simd(dst: &mut [u8], src: &[u8]) {
    dst.copy_from_slice(src);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn memset_avx2_inner(dst: &mut [u8], val: u8) {
    use std::arch::x86_64::*;

    let fill = _mm256_set1_epi8(val as i8);
    let mut i = 0;

    while i + 32 <= dst.len() {
        unsafe {
            _mm256_storeu_si256(dst.as_mut_ptr().add(i) as *mut __m256i, fill);
        }
        i += 32;
    }

    while i < dst.len() {
        dst[i] = val;
        i += 1;
    }
}

#[cfg(target_arch = "x86_64")]
pub fn memset_simd(dst: &mut [u8], val: u8) {
    if super::is_avx2_available() {
        unsafe { memset_avx2_inner(dst, val) }
    } else {
        dst.fill(val);
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn memset_simd(dst: &mut [u8], val: u8) {
    dst.fill(val);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn memchr_avx2_inner(data: &[u8], needle: u8) -> Option<usize> {
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
pub fn memchr_simd(data: &[u8], needle: u8) -> Option<usize> {
    if super::is_avx2_available() {
        unsafe { memchr_avx2_inner(data, needle) }
    } else {
        data.iter().position(|&b| b == needle)
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn memchr_simd(data: &[u8], needle: u8) -> Option<usize> {
    data.iter().position(|&b| b == needle)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn memcmp_avx2_inner(a: &[u8], b: &[u8]) -> bool {
    use std::arch::x86_64::*;

    if a.len() != b.len() {
        return false;
    }

    let mut i = 0;
    while i + 32 <= a.len() {
        unsafe {
            let a_chunk = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
            let b_chunk = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);

            let cmp = _mm256_cmpeq_epi8(a_chunk, b_chunk);
            let mask = _mm256_movemask_epi8(cmp) as u32;

            if mask != 0xFFFFFFFF {
                return false;
            }
        }
        i += 32;
    }

    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }

    true
}

#[cfg(target_arch = "x86_64")]
pub fn memcmp_simd(a: &[u8], b: &[u8]) -> bool {
    if super::is_avx2_available() {
        unsafe { memcmp_avx2_inner(a, b) }
    } else {
        a == b
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn memcmp_simd(a: &[u8], b: &[u8]) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcpy_basic() {
        let src = b"hello, simd world!";
        let mut dst = vec![0u8; src.len()];
        memcpy_simd(&mut dst, src);
        assert_eq!(&dst, src);
    }

    #[test]
    fn test_memcpy_empty() {
        let mut dst = vec![0u8; 0];
        memcpy_simd(&mut dst, &[]);
    }

    #[test]
    fn test_memset_basic() {
        let mut buf = vec![0u8; 64];
        memset_simd(&mut buf, 0xFF);
        assert!(buf.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_memset_empty() {
        let mut buf = vec![0u8; 0];
        memset_simd(&mut buf, 0xAA);
    }

    #[test]
    fn test_memchr_found() {
        assert_eq!(memchr_simd(b"hello", b'l'), Some(2));
    }

    #[test]
    fn test_memchr_not_found() {
        assert_eq!(memchr_simd(b"hello", b'x'), None);
    }

    #[test]
    fn test_memchr_at_start() {
        assert_eq!(memchr_simd(b"hello", b'h'), Some(0));
    }

    #[test]
    fn test_memchr_at_end() {
        assert_eq!(memchr_simd(b"hello", b'o'), Some(4));
    }

    #[test]
    fn test_memcmp_equal() {
        assert!(memcmp_simd(b"hello", b"hello"));
    }

    #[test]
    fn test_memcmp_not_equal() {
        assert!(!memcmp_simd(b"hello", b"world"));
    }

    #[test]
    fn test_memcmp_different_length() {
        assert!(!memcmp_simd(b"hello", b"hello world"));
    }

    #[test]
    fn test_memcmp_empty() {
        assert!(memcmp_simd(b"", b""));
    }
}

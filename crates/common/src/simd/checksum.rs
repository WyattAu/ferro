#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn crc32_avx2_inner(data: &[u8]) -> u32 {
    use std::arch::x86_64::*;

    let mut crc: u32 = 0xFFFFFFFF;
    let mut i = 0;

    while i + 32 <= data.len() {
        unsafe {
            let chunk = _mm256_loadu_si256(data.as_ptr().add(i) as *const __m256i);
            let bytes: [u8; 32] = std::mem::transmute(chunk);

            for byte in bytes {
                crc = _mm_crc32_u8(crc, byte);
            }
        }
        i += 32;
    }

    while i < data.len() {
        crc = _mm_crc32_u8(crc, data[i]);
        i += 1;
    }

    crc ^ 0xFFFFFFFF
}

#[cfg(target_arch = "x86_64")]
pub fn crc32_simd(data: &[u8]) -> u32 {
    if super::is_avx2_available() {
        unsafe { crc32_avx2_inner(data) }
    } else {
        software_crc32(data)
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn crc32_simd(data: &[u8]) -> u32 {
    software_crc32(data)
}

fn software_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;

    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x82F63B78;
            } else {
                crc >>= 1;
            }
        }
    }

    crc ^ 0xFFFFFFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_empty() {
        assert_eq!(crc32_simd(&[]), 0x00000000);
    }

    #[test]
    fn test_crc32_known_values() {
        assert_eq!(crc32_simd(b"123456789"), 0xE3069283);
    }

    #[test]
    fn test_crc32_matches_software() {
        let data = b"Hello, SIMD world!";
        assert_eq!(crc32_simd(data), software_crc32(data));
    }

    #[test]
    fn test_crc32_large_data() {
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        assert_eq!(crc32_simd(&data), software_crc32(&data));
    }
}

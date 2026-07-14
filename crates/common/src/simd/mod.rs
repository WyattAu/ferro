pub mod bulk;
pub mod checksum;
pub mod compare;

#[cfg(target_arch = "x86_64")]
pub fn is_avx2_available() -> bool {
    std::is_x86_feature_detected!("avx2")
}

#[cfg(not(target_arch = "x86_64"))]
pub fn is_avx2_available() -> bool {
    false
}

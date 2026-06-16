fn main() {
    #[cfg(any(feature = "ios", feature = "android"))]
    tauri_build::build()
}

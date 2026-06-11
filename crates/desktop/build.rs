fn main() {
    #[cfg(any(feature = "tauri", feature = "mobile"))]
    tauri_build::build()
}

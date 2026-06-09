#[cfg(target_arch = "wasm32")]
const SAMPLE_FOLDERS: &[&str] = &["Documents", "Photos", "Videos", "Music", "Shared"];

#[cfg(target_arch = "wasm32")]
const SAMPLE_README_TEMPLATES: &[(&str, &str)] = &[
    (
        "Documents",
        "# Documents\n\nStore your documents, spreadsheets, and text files here.\n\nSupports: PDF, DOCX, TXT, CSV, and more.\n",
    ),
    (
        "Photos",
        "# Photos\n\nYour photo library. Organize by date, event, or project.\n\nSupports: JPG, PNG, RAW, HEIC, and more.\n",
    ),
    (
        "Videos",
        "# Videos\n\nVideo files and recordings.\n\nSupports: MP4, MOV, AVI, MKV, and more.\n",
    ),
    (
        "Music",
        "# Music\n\nAudio files and music collection.\n\nSupports: MP3, FLAC, WAV, OGG, and more.\n",
    ),
    (
        "Shared",
        "# Shared\n\nFiles shared with other users. Place items here for collaboration.\n",
    ),
];

#[cfg(target_arch = "wasm32")]
const WELCOME_TXT: &str = "# Welcome to Ferro!\n\nThis is your personal distributed storage solution.\n\n## Getting Started\n\n1. **Upload files** - Drag and drop onto this window or use the upload button\n2. **Create folders** - Organize your files with the new folder button\n3. **Navigate** - Use the sidebar and breadcrumbs to move between folders\n4. **Search** - Press / to open the command palette and search files\n5. **Shortcuts** - Press ? to see all keyboard shortcuts\n\n## Admin Dashboard\n\nAccess the admin panel at `/ui/admin` to:\n- Monitor storage usage and health\n- Manage users and permissions\n- View audit logs and activity\n- Configure server settings\n\n## Need Help?\n\n- Check the README in each folder for tips\n- Use the command palette (Ctrl+K) for quick actions\n- Report issues at the project repository\n\nHappy storing!\n";

pub fn create_sample_folders() {
    #[cfg(target_arch = "wasm32")]
    {
        for folder in SAMPLE_FOLDERS {
            let readme_key = format!("ferro_sample_{}_readme", folder.to_lowercase());
            let readme_content = SAMPLE_README_TEMPLATES
                .iter()
                .find(|(name, _)| *name == *folder)
                .map(|(_, content)| *content)
                .unwrap_or("");

            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item(&readme_key, readme_content);
                }
            }
        }

        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("ferro_sample_welcome", WELCOME_TXT);
                let _ = storage.set_item("ferro_sample_folders_created", "true");
            }
        }
    }
}

pub fn are_sample_folders_created() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(val)) = storage.get_item("ferro_sample_folders_created") {
                    return val == "true";
                }
            }
        }
    }
    false
}

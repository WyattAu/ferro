# Ferro Desktop — Install & Validation Checklist (v1.0.0 pre-tag)

Artifacts: GitHub → Actions → latest green "Desktop (All Platforms)" run → Artifacts section.
Each artifact is a ZIP containing the installer(s).

## Windows (NSIS)
1. Download `ferro-desktop-windows-installer` → unzip → run `Ferro_3.0.0_x64-setup.exe`
2. Launch "Ferro" from the Start menu
3. VALIDATE:
   - [ ] Window opens (tray icon present)
   - [ ] `ferro-desktop --login` from a terminal → browser opens → sign in with company account → "Signed in as …"
   - [ ] `ferro-desktop --server-url https://ferro.wyattau.com --sync-once --local-dir %USERPROFILE%\Ferro` → files appear
   - [ ] Create/edit/delete a file in the synced folder → re-run sync → changes propagate
   - [ ] Edit a >8 MB file → re-run sync → "uploading changed blocks" in log (block dedup path)
   - [ ] Reboot → tray auto-mount still works (if configured)

## Linux (deb / rpm / AppImage)
1. `sudo apt install ./ferro-desktop_3.0.0_amd64.deb` (or `dnf install ./…rpm`), or chmod+x the AppImage and run
2. Same VALIDATE list as Windows; use `--local-dir ~/Ferro`
3. Keychain: Secret Service prompt may appear on first login-token storage — allow it

## macOS (DMG)
- KNOWN FAILING: aarch64 compile errors (Cow/f32 trait bounds) — needs a dedicated fix session.
- Interim: build from source on a Mac (`cargo build --release -p ferro-desktop --features "tauri,sync"`).

## Notes
- Tokens are stored in the OS secret store (Windows Credential Manager / Secret Service).
  A `secrets.json` fallback (0600) is used when no keyring daemon exists.
- Access tokens expire after 15 min; the sync engine refreshes automatically.
- Headless servers: use `--login` once interactively, then `--sync-once` / `--sync-interval N`.

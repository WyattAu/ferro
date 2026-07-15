# UI/UX Feature Roadmap

**Version:** 1.0 | **Date:** 2026-07-15 | **Status:** ACTIVE

---

## Roadmap Overview

This roadmap addresses all UI/UX gaps identified in `docs/ui_ux_comparative_analysis.md`. Items are organized into 4 phases, ordered by priority, with specific acceptance criteria and effort estimates.

**Total estimated effort:** 20 weeks (single engineer) / 12 weeks (2 engineers)

**Current theme system:** 4 themes (Light, Dark, Midnight, System) using CSS custom properties in `crates/web/src/styles/dark_mode.rs` and `crates/web/style.css`.

---

## Phase 1: Multi-Theme System (Week 1-2)

**Goal:** Expand from 4 themes to 14 themes. All themes must be Tailwind-compatible using CSS custom properties.

**Priority:** P0
**Dependencies:** None
**Theme Impact:** Core — all new themes define the full 60+ CSS custom property set

### 1.1 Theme Architecture Refactor

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 2 days |
| Dependencies | None |

**Implementation:**
1. Refactor `crates/web/src/styles/dark_mode.rs` to support dynamic theme loading
2. Create a `ThemeDefinition` struct with all 60+ CSS custom properties
3. Add a `themes/` directory with individual theme TOML files
4. Implement theme validation (all required properties must be present)
5. Add theme import/export for custom themes

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined per theme
- [ ] Theme switching works in <100ms
- [ ] No visual flash on theme change (FOUC prevention)
- [ ] Theme preference persisted in localStorage

### 1.2 CSS Custom Property Set

Every theme must define these properties:

```
/* Core Colors */
--accent, --accent-dark, --accent-glow, --accent-subtle
--crimson

/* Surfaces */
--surface, --surface-alt, --surface-elevated, --surface-overlay
--surface-interactive, --surface-interactive-hover

/* Text */
--text-primary, --text-secondary, --text-tertiary, --text-inverse
--text-accent, --text-danger, --text-success, --text-warning

/* Borders */
--border, --border-strong, --border-focus, --border-subtle
--border-danger, --border-success, --border-warning

/* Interactive States */
--hover-bg, --active-bg, --focus-ring
--disabled-bg, --disabled-text

/* Feedback Colors */
--success, --success-bg, --warning, --warning-bg
--danger, --danger-bg, --info, --info-bg

/* Shadows */
--shadow-sm, --shadow-md, --shadow-lg, --shadow-xl

/* Typography */
--font-display, --font-body

/* Special */
--scrollbar-track, --scrollbar-thumb
--code-bg, --code-text
--selection-bg, --selection-text
```

### 1.3 New Theme: Solarized Light

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/solarized-light.toml`
2. Warm, low-contrast palette based on Ethan Schoonover's Solarized
3. Base: `#fdf6e3`, Text: `#657b83`, Accent: `#268bd2`
4. Must pass WCAG AA contrast ratios (4.5:1 minimum)

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AA compliant (4.5:1 contrast ratio)
- [ ] Consistent with official Solarized color values
- [ ] No harsh transitions between elements

### 1.4 New Theme: Solarized Dark

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/solarized-dark.toml`
2. Warm dark palette based on Solarized Dark
3. Base: `#002b36`, Text: `#839496`, Accent: `#b58900`
4. Must pass WCAG AA contrast ratios

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AA compliant
- [ ] Warm undertones preserved (not cold/blue)

### 1.5 New Theme: Nord

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/nord.toml`
2. Arctic, cool-toned palette from Nord theme
3. Base: `#2e3440`, Text: `#d8dee9`, Accent: `#88c0d0`
4. 7 color frost tones, 4 aurora accent colors

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AA compliant
- [ ] Cool tones maintained throughout

### 1.6 New Theme: Tokyo Night

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/tokyo-night.toml`
2. Deep blue/purple palette inspired by Tokyo's neon night
3. Base: `#1a1b26`, Text: `#a9b1d6`, Accent: `#7aa2f7`
4. Purple/pink secondary accents

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AA compliant
- [ ] Deep blue/purple atmosphere preserved

### 1.7 New Theme: Dracula

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/dracula.toml`
2. Purple/green/pink palette from Dracula theme
3. Base: `#282a36`, Text: `#f8f8f2`, Accent: `#bd93f9`
4. 5 official accent colors: cyan, green, orange, pink, purple

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AA compliant
- [ ] All 5 accent colors used appropriately

### 1.8 New Theme: High Contrast

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/high-contrast.toml`
2. WCAG AAA compliant (7:1 contrast ratio minimum)
3. Pure black/white base with vivid accent colors
4. Enhanced focus indicators (3px outlines)
5. Thicker borders for visibility

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AAA compliant (7:1 contrast ratio)
- [ ] Enhanced focus indicators for keyboard navigation
- [ ] Thicker borders (3px minimum)

### 1.9 New Theme: Sepia

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/sepia.toml`
2. Warm reading mode with reduced blue light
3. Base: `#f4ecd8`, Text: `#5b4636`, Accent: `#b8860b`
4. Reduced screen fatigue for long reading sessions

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AA compliant
- [ ] Blue light reduced by >50% vs default light theme

### 1.10 New Theme: Forest

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/forest.toml`
2. Green-toned dark theme inspired by dense forest
3. Base: `#1a2e1a`, Text: `#c8d6c0`, Accent: `#4ade80`
4. Multiple green shades for depth

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AA compliant
- [ ] Green tones consistent across all surfaces

### 1.11 New Theme: Ocean

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 0.5 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Create `themes/ocean.toml`
2. Blue-teal dark theme inspired by deep ocean
3. Base: `#0d1b2a`, Text: `#b8c5d6`, Accent: `#22d3ee`
4. Teal secondary accents

**Acceptance Criteria:**
- [ ] All 60+ CSS custom properties defined
- [ ] WCAG AA compliant
- [ ] Blue-teal atmosphere maintained

### 1.12 Custom Theme Builder

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 3 days |
| Dependencies | Theme architecture refactor |

**Implementation:**
1. Add `/settings/themes/custom` route
2. Color picker for each CSS custom property
3. Real-time preview panel
4. Import/export as JSON/TOML
5. Validate contrast ratios live (WCAG AA warning)
6. Save custom themes to localStorage + backend

**Acceptance Criteria:**
- [ ] Users can define all 60+ CSS custom properties
- [ ] Real-time preview updates in <50ms
- [ ] Export/import as shareable JSON file
- [ ] WCAG contrast warning shown when ratio <4.5:1
- [ ] Custom themes persist across sessions

### 1.13 Theme Management UI

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 2 days |
| Dependencies | All theme implementations |

**Implementation:**
1. Theme grid in Settings > Appearance (2-column grid with previews)
2. Theme preview cards showing color swatches
3. One-click theme switching
4. "System" option respects OS preference
5. Theme search/filter for 14+ themes
6. Theme preview animation on hover

**Acceptance Criteria:**
- [ ] All 14 themes displayed in grid layout
- [ ] Preview card shows primary colors + text
- [ ] Clicking a theme applies it immediately
- [ ] Current theme highlighted in grid
- [ ] Mobile-responsive layout

---

## Phase 2: Critical Gap Features (Week 3-6)

**Goal:** Close the most impactful competitive gaps that affect core usability.

### 2.1 Resumable Uploads (TUS Protocol)

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 1 week |
| Dependencies | Storage backend |
| Theme Impact | None |

**Implementation:**
1. Integrate TUS (resumable upload protocol) server-side
2. Add TUS client library to WASM frontend
3. Upload queue with pause/resume/cancel per file
4. Server-side upload state tracking (Redis)
5. Resume detection on page reload
6. Progress indicator with ETA calculation
7. Configurable chunk size (default 5MB)

**Acceptance Criteria:**
- [ ] Uploads resume after browser close/reopen
- [ ] Uploads resume after network interruption
- [ ] Progress indicator shows per-file progress
- [ ] Queue supports pause/resume/cancel per file
- [ ] Upload success rate >99% for files <10GB
- [ ] Works with S3, GCS, Azure, and local storage backends

### 2.2 PWA Support

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 3 days |
| Dependencies | None |
| Theme Impact | Uses `--accent` for theme_color in manifest |

**Implementation:**
1. Create `manifest.json` with app metadata
2. Generate service worker with workbox
3. Cache static assets (app shell strategy)
4. Offline fallback page
5. Push notification support (web push API)
6. Install prompt handling
7. Background sync for pending uploads

**Acceptance Criteria:**
- [ ] Lighthouse PWA audit score >90
- [ ] App installable on Chrome, Firefox, Safari
- [ ] Offline: cached assets load, API calls queue
- [ ] Push notifications work on Android/Chrome
- [ ] Background sync retries failed uploads
- [ ] Splash screen uses theme colors

### 2.3 ZIP Download

| Attribute | Value |
|-----------|-------|
| Priority | P0 |
| Effort | 2 days |
| Dependencies | None |
| Theme Impact | None |

**Implementation:**
1. Server-side streaming ZIP creation (no temp files)
2. Client-side selection UI (checkboxes in file browser)
3. "Download as ZIP" button in toolbar + context menu
4. Progress indicator for ZIP generation
5. Support for nested folder structures
6. Filename sanitization for ZIP entries

**Acceptance Criteria:**
- [ ] ZIP download works for 1-1000 files
- [ ] Nested folder structure preserved in ZIP
- [ ] Progress indicator during ZIP creation
- [ ] ZIP generation completes in <30s for 100 files
- [ ] Maximum ZIP size configurable (default 2GB)
- [ ] Works with file selection (bulk select + download)

### 2.4 Duplicate Files

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 1 day |
| Dependencies | None |
| Theme Impact | None |

**Implementation:**
1. Add "Duplicate" option to context menu
2. POST to `/api/files/{id}/duplicate`
3. Backend creates copy with " (copy)" suffix
4. Preserve metadata (tags, annotations)
5. Return new file ID for immediate navigation
6. Duplicate inherits parent folder permissions

**Acceptance Criteria:**
- [ ] Duplicate appears in same folder
- [ ] Filename gets " (copy)" suffix
- [ ] File content is identical (byte-for-byte)
- [ ] Metadata (tags, favorites) preserved
- [ ] Duplicate appears in <2 seconds
- [ ] Works for files up to 1GB

### 2.5 Saved Searches

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 1 day |
| Dependencies | Search infrastructure |
| Theme Impact | None |

**Implementation:**
1. Add "Save Search" button next to search bar
2. Store search parameters (query, filters, sort)
3. Saved searches in sidebar under "Searches"
4. Click to re-execute search
5. Rename/delete saved searches
6. Maximum 50 saved searches per user

**Acceptance Criteria:**
- [ ] Save search in 1 click
- [ ] Saved searches appear in sidebar
- [ ] Click executes search instantly
- [ ] Rename/delete via context menu
- [ ] Persists across sessions (backend storage)
- [ ] Maximum 50 saved searches enforced

### 2.6 Photo Gallery Timeline/Map View

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 1 week |
| Dependencies | EXIF extraction |
| Theme Impact | Uses theme colors for timeline markers |

**Implementation:**
1. Timeline view with date-based grouping (year > month > day)
2. Virtualized list for performance (10k+ photos)
3. Thumbnail grid with lazy loading
4. Map view using Leaflet/Mapbox integration
5. EXIF GPS extraction and geocoding
6. Cluster markers for dense areas
7. Photo metadata panel (EXIF, tags, location)

**Acceptance Criteria:**
- [ ] Timeline groups photos by date
- [ ] Smooth scrolling through 10k+ photos
- [ ] Map view displays photos at GPS coordinates
- [ ] Cluster markers for >5 photos in same area
- [ ] Click photo in map navigates to full view
- [ ] Map loads in <3 seconds for 1000 photos

### 2.7 Federated Sharing (OCM Protocol)

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 2 weeks |
| Dependencies | OCM protocol implementation |
| Theme Impact | None |

**Implementation:**
1. Implement OCM (Open Collaboration Metadata) protocol
2. OCS Discovery endpoint for server info
3. Share notification system between instances
4. Federated share acceptance/rejection UI
5. Federated share permissions (view/edit)
6. Trust management for known servers
7. Audit logging for federated shares

**Acceptance Criteria:**
- [ ] Share files with users on other Ferro/ownCloud instances
- [ ] OCS Discovery endpoint responds correctly
- [ ] Share notifications delivered reliably
- [ ] Accept/reject shares from remote users
- [ ] Permissions respected (view/edit)
- [ ] Trust list manageable in admin settings
- [ ] Federated shares appear in activity feed

---

## Phase 3: High-Priority Features (Week 7-12)

**Goal:** Add features that enhance collaboration, mobile experience, and media management.

### 3.1 File Requests (Upload-Only Public Links)

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 2 days |
| Dependencies | Notification system |
| Theme Impact | None |

**Implementation:**
1. Generate upload-only link with metadata (file type, size limit)
2. Public upload page (no Ferro account required)
3. Email notifications for new uploads
4. Webhook support for automation
5. Expiration and max upload settings
6. Upload queue with progress for external users

**Acceptance Criteria:**
- [ ] Upload-only link generates in <2 seconds
- [ ] External users can upload without account
- [ ] File type restrictions enforced
- [ ] Size limits enforced per upload
- [ ] Email notification sent on upload
- [ ] Expiration date enforced

### 3.2 QR Code Sharing

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 1 day |
| Dependencies | Public link system |
| Theme Impact | Uses `--accent` for QR foreground |

**Implementation:**
1. Generate QR code for any share link
2. Download QR as PNG/SVG
3. QR code updates when link changes
4. Customizable QR colors (foreground/background)
5. Embed in share dialog

**Acceptance Criteria:**
- [ ] QR code generated in <500ms
- [ ] Scannable from 10+ feet distance
- [ ] Download as PNG and SVG
- [ ] Colors customizable
- [ ] Works with all public link types

### 3.3 Group Management

| Attribute | Value |
|-----------|-------|
| Priority | P1 |
| Effort | 3 days |
| Dependencies | User management |
| Theme Impact | None |

**Implementation:**
1. Create/edit/delete groups in admin panel
2. Assign users to groups
3. Share files/folders with groups
4. Group-level permissions (RBAC)
5. LDAP group sync
6. Group activity feed

**Acceptance Criteria:**
- [ ] Groups CRUD in admin settings
- [ ] Users assignable to multiple groups
- [ ] File sharing with groups works
- [ ] Group permissions override individual
- [ ] LDAP groups synced automatically
- [ ] Group activity shows all member actions

### 3.4 EPUB Preview

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 2 days |
| Dependencies | None |
| Theme Impact | Uses `--surface`, `--text-primary` for reader chrome |

**Implementation:**
1. Integrate epub.js for in-browser EPUB rendering
2. Full-screen reading mode
3. Table of contents navigation
4. Font size adjustment
5. Night mode (theme-aware)
6. Bookmark/progress saving

**Acceptance Criteria:**
- [ ] EPUB files open in browser reader
- [ ] Full-screen mode works
- [ ] Table of contents navigation works
- [ ] Font size adjustable (12px-24px)
- [ ] Reading progress saved per file
- [ ] Works with EPUB 2 and EPUB 3

### 3.5 Background Audio Player

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 2 days |
| Dependencies | None |
| Theme Impact | Uses `--surface-elevated` for player bar |

**Implementation:**
1. Persistent audio player bar at bottom
2. Play/pause, next/previous, seek
3. Volume control
4. Playlist management
5. Continues playing when navigating away
6. Media session API integration (OS controls)

**Acceptance Criteria:**
- [ ] Audio plays across page navigation
- [ ] OS media controls work (play/pause/skip)
- [ ] Playlist persists during session
- [ ] Volume slider works
- [ ] Seek bar shows progress
- [ ] Player bar uses theme colors

### 3.6 Slideshow Mode

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 1 day |
| Dependencies | Photo gallery |
| Theme Impact | Uses `--surface` for fullscreen background |

**Implementation:**
1. Full-screen slideshow from photo gallery
2. Auto-advance with configurable timer (3s-30s)
3. Manual next/previous with arrow keys
4. Transition effects (fade, slide)
5. EXIF info overlay toggle
6. Exit with Escape key

**Acceptance Criteria:**
- [ ] Slideshow starts in <1 second
- [ ] Auto-advance works with configurable timer
- [ ] Arrow keys navigate
- [ ] EXIF info togglable
- [ ] Escape exits fullscreen
- [ ] Works with 1000+ photo albums

### 3.7 Photo Editing (Basic Crop/Rotate)

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 1 week |
| Dependencies | Canvas library (Konva.js) |
| Theme Impact | Uses `--surface-elevated` for editor chrome |

**Implementation:**
1. Integrate Konva.js for canvas manipulation
2. Crop tool with aspect ratio presets
3. Rotate (90° CW/CCW, free rotation)
4. Flip (horizontal/vertical)
5. Brightness/contrast sliders
6. Save as new version (preserve original)
7. Undo/redo stack

**Acceptance Criteria:**
- [ ] Crop with free and preset ratios
- [ ] Rotate 90° and free rotation
- [ ] Flip horizontal/vertical
- [ ] Brightness/contrast adjustable
- [ ] Original preserved as version
- [ ] Undo/redo works (10 levels)
- [ ] Save completes in <3 seconds

### 3.8 Camera Upload (Mobile Auto-Backup)

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 1 week |
| Dependencies | Mobile app or PWA |
| Theme Impact | None |

**Implementation:**
1. Camera API integration (PWA or native)
2. Background upload when on WiFi
3. Duplicate detection (hash-based)
4. Upload queue with retry
5. Battery-aware (pause when low)
6. Selective folder backup

**Acceptance Criteria:**
- [ ] Photos auto-upload when on WiFi
- [ ] Duplicate detection prevents re-upload
- [ ] Upload resumes after connection loss
- [ ] Battery-aware (pauses below 20%)
- [ ] Selective folder selection
- [ ] Progress indicator in notification shade

---

## Phase 4: Medium-Priority Features (Week 13-20)

**Goal:** Advanced features for power users, admins, and extensibility.

### 4.1 Map View for Photos

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 2 days |
| Dependencies | Leaflet/Mapbox, EXIF extraction |
| Theme Impact | Uses `--surface`, `--text-primary` for map controls |

**Implementation:**
1. Leaflet.js integration with tile server
2. Photo markers with thumbnail previews
3. Cluster markers for dense areas
4. Click to open photo detail
5. Filter by date range on map
6. Export map as image

**Acceptance Criteria:**
- [ ] Map loads in <3 seconds
- [ ] Photos displayed at GPS coordinates
- [ ] Clusters form for >5 nearby photos
- [ ] Click marker opens photo
- [ ] Date filter works
- [ ] Export as PNG works

### 4.2 Custom Folder Views

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 1 week |
| Dependencies | UI framework |
| Theme Impact | None |

**Implementation:**
1. User-defined view templates (list, grid, gallery)
2. Column configuration for list view
3. Sort preferences per folder
4. View persistence (remember per folder)
5. Default view setting
6. Share view configuration

**Acceptance Criteria:**
- [ ] Users can create custom view templates
- [ ] Columns configurable (name, size, date, tags, etc.)
- [ ] Sort preferences saved per folder
- [ ] Views persist across sessions
- [ ] Default view applies to new folders
- [ ] Views shareable with other users

### 4.3 Smart Collections

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 1 week |
| Dependencies | Search/filter infrastructure |
| Theme Impact | None |

**Implementation:**
1. Rule-based dynamic collections (by tag, date, type, size)
2. Auto-updating as files change
3. Collection editor UI (add/remove rules)
4. Nesting support (collection of collections)
5. Share collections with users/groups
6. Collection activity feed

**Acceptance Criteria:**
- [ ] Create collection with 1-10 rules
- [ ] Rules: tag, date range, file type, size, name pattern
- [ ] Collection updates in real-time
- [ ] Collections shareable
- [ ] Activity feed shows collection changes
- [ ] Works with 100k+ files

### 4.4 Remote Wipe

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 3 days |
| Dependencies | Device management |
| Theme Impact | None |

**Implementation:**
1. Device management panel in admin settings
2. Per-device wipe command
3. Bulk wipe for user
4. Audit log for wipe events
5. Confirmation dialog with warning
6. Wipe status indicator (pending/wiped)

**Acceptance Criteria:**
- [ ] Admin can wipe individual devices
- [ ] Admin can wipe all devices for a user
- [ ] Wipe command sent in <5 seconds
- [ ] Wipe status tracked in audit log
- [ ] Confirmation required before wipe
- [ ] Wiped device loses access immediately

### 4.5 Workflow Automation

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 2 weeks |
| Dependencies | Event system |
| Theme Impact | None |

**Implementation:**
1. Visual workflow builder (node-based)
2. Triggers: file upload, share, tag, rename, delete
3. Actions: move, copy, tag, notify, webhook, convert
4. Conditions: file type, size, name pattern, user
5. Workflow templates (common patterns)
6. Workflow execution log

**Acceptance Criteria:**
- [ ] Create workflow with drag-and-drop
- [ ] 5+ trigger types available
- [ ] 6+ action types available
- [ ] Conditions filter execution
- [ ] Workflows execute in <5 seconds
- [ ] Execution log shows status/errors

### 4.6 Plugin Marketplace

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 1 week |
| Dependencies | Extension system |
| Theme Impact | None |

**Implementation:**
1. Plugin registry (JSON manifest)
2. Install/uninstall/update from UI
3. Plugin sandbox (permissions model)
4. Plugin configuration UI
5. Plugin dependency resolution
6. Plugin health monitoring

**Acceptance Criteria:**
- [ ] Browse plugins in marketplace
- [ ] One-click install/uninstall
- [ ] Plugin permissions requested on install
- [ ] Plugin config in settings
- [ ] Dependencies resolved automatically
- [ ] Failed plugins isolated

### 4.7 Advanced Admin Compliance Tools

| Attribute | Value |
|-----------|-------|
| Priority | P2 |
| Effort | 1 week |
| Dependencies | Audit logging |
| Theme Impact | None |

**Implementation:**
1. Compliance dashboard (GDPR, HIPAA, SOC2)
2. Data retention policy management
3. eDiscovery export (PST, EML formats)
4. Sensitivity labels (auto-classification)
5. Legal hold management
6. Compliance report generation

**Acceptance Criteria:**
- [ ] Dashboard shows compliance status
- [ ] Retention policies configurable
- [ ] eDiscovery export works
- [ ] Sensitivity labels auto-classify
- [ ] Legal hold prevents deletion
- [ ] Reports exportable as PDF/CSV

---

## Execution Timeline

```
Week 1-2:   Phase 1 (Multi-Theme System) ── Theme architecture + 10 new themes
Week 3-4:   Phase 2 (Critical Gaps) ── Resumable uploads, PWA, ZIP download
Week 5-6:   Phase 2 (Critical Gaps) ── Duplicate, saved searches, photos, federation
Week 7-8:   Phase 3 (High-Priority) ── File requests, QR codes, groups, EPUB
Week 9-10:  Phase 3 (High-Priority) ── Audio player, slideshow, photo editing
Week 11-12: Phase 3 (High-Priority) ── Camera upload, polish
Week 13-14: Phase 4 (Medium-Priority) ── Map view, custom views, smart collections
Week 15-16: Phase 4 (Medium-Priority) ── Remote wipe, workflow automation
Week 17-18: Phase 4 (Medium-Priority) ── Plugin marketplace
Week 19-20: Phase 4 (Medium-Priority) ── Compliance tools, polish
```

**Parallel tracks:**
- Phase 1 is independent (can start immediately)
- Phase 2 items are independent of each other
- Phase 3 depends on Phase 2.6 (photo gallery for timeline)
- Phase 4 depends on Phase 3.3 (groups for admin tools)

---

## Success Criteria

| Metric | Current | Target | Timeline |
|--------|---------|--------|----------|
| Themes available | 4 | 14 | Week 2 |
| WCAG AAA themes | 0 | 1 | Week 2 |
| Custom theme builder | No | Yes | Week 2 |
| Resumable uploads | No | Yes | Week 4 |
| PWA score | N/A | >90 | Week 4 |
| ZIP download | No | Yes | Week 4 |
| File requests | No | Yes | Week 7 |
| Photo timeline | No | Yes | Week 6 |
| Federated sharing | No | Yes | Week 6 |
| Photo editing | No | Yes | Week 10 |
| Workflow automation | No | Yes | Week 16 |
| Plugin marketplace | No | Yes | Week 18 |
| Compliance tools | No | Yes | Week 20 |

---

## Resource Requirements

**Single Engineer:** 20 weeks total
**Two Engineers:** 12 weeks total (parallel work)

**Recommended allocation:**
- Engineer A: Phase 1 → Phase 2.1-2.3 → Phase 3.1-3.4 → Phase 4.1-4.3
- Engineer B: Phase 2.4-2.7 → Phase 3.5-3.8 → Phase 4.4-4.7

---

## Risk Mitigation

| Risk | Mitigation | Contingency |
|------|------------|-------------|
| TUS protocol complexity | Use tus-rs library | Implement custom resumable upload |
| PWA offline complexity | Start with app shell caching | Defer offline to Phase 4 |
| OCM protocol immaturity | Wait for oCIS reference impl | Implement basic federation first |
| Plugin security | Sandboxed execution | Defer marketplace to Phase 5 |
| Photo editing scope | Start with crop/rotate only | Defer filters/adjustments |

---

*Document generated: July 2026*
*Next review: October 2026*
*Owner: Ferro Engineering Team*

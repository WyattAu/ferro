# Ferro GUI Current State Audit

> **Date:** 2026-07-22
> **Scope:** Frontend crate (`crates/web/`) — Leptos 0.8 CSR WASM application
> **Purpose:** Establish a complete baseline of the current GUI state before refactor planning

---

## 1. Architecture Overview

| Aspect | Detail |
|---|---|
| **Framework** | Leptos 0.8 CSR (client-side rendered) compiled to WASM |
| **Build tool** | Trunk (`crates/web/Trunk.toml`), `public_url = "/ui/"` |
| **Desktop shell** | Tauri v2 wrapping webkit2gtk on Linux |
| **CSS approach** | Hand-written utility classes in `style.css` (NOT Tailwind). `dark_mode.rs` injects 14 themes via CSS custom properties at runtime. |
| **State management** | Leptos signals (`signal`, `create_signal`). No global store (e.g. `Store`). Each page instantiates its own ~15-30 signals. |
| **Routing** | `leptos_router` 0.8 with 16 routes defined in `app.rs` |
| **i18n** | Custom `t!()` macro over a static English-only translation table (`i18n/en.rs`). `Locale` enum with single variant `En`. |
| **Mount target** | `mount_to_body` — Leptos mounts to `<body>`, not `#app` |
| **WASM binary** | ~3.8 MB release build |
| **Dependency health** | Leptos 0.8 (current), Tauri 2.x (current), webkit2gtk 2.x (Linux) |

### Route Table

| Path | Component | Notes |
|---|---|---|
| `/` | `RootView` → `HomePage("/")` | Alias for `/ui/` |
| `/ui/` | `RootView` → `HomePage("/")` | Default entry |
| `/ui/dashboard` | `DashboardPage` | |
| `/ui/files/` | `RootView` → `HomePage("/")` | Root file browser |
| `/ui/files/*path` | `FileViewRoute` → `HomePage(path)` | Parameterized file browser |
| `/ui/trash` | `TrashPage` | |
| `/ui/settings` | `SettingsPage` | |
| `/ui/admin` | `AdminPage` | Enterprise admin |
| `/ui/calendar` | `CalendarPage` | CalDAV |
| `/ui/contacts` | `ContactsPage` | CardDAV |
| `/ui/notes` | `NotesPage` | |
| `/ui/tasks` | `TasksPage` | |
| `/ui/chat` | `ChatPage` | |
| `/ui/chat/*room_id` | `ChatPage` | Chat with room |
| `/ui/photos` | `PhotosPage` | |
| `/ui/mail` | `MailPage` | |
| `/ui/whiteboard` | `WhiteboardPage` | |
| `/ui/whiteboard/:id` | `WhiteboardPage` | |
| `/ui/analytics` | `AnalyticsPage` | |
| `/ui/auth/callback` | `AuthCallbackPage` | OAuth callback |
| `/ui/auth/login` | `LoginPage` | |

---

## 2. Feature Inventory by Page

### 2.1 File Browser (Home)

**Route:** `/ui/files/*path`
**Primary files:** `pages/home.rs`, `components/file_browser/`

#### API Endpoints

| Protocol | Endpoint | Purpose |
|---|---|---|
| WebDAV PROPFIND | `{path}` (Depth: 1) | List directory contents, XML parsed by hand |
| WebDAV PUT | `{path}` | Upload file (binary body) |
| WebDAV DELETE | `{path}` | Delete file or directory |
| WebDAV MKCOL | `{path}` | Create directory |
| REST GET | `/api/favorites` | List favorited paths |
| REST PUT | `/api/favorites` | Add favorite |
| REST DELETE | `/api/favorites` | Remove favorite |
| REST GET | `/api/recent` | Recently accessed files |
| REST GET | `/api/search?q={query}` | Full-text search with filters (type, sort, mime_type) |
| REST GET | `/api/locks` | List file locks |
| REST POST | `/api/locks/force-unlock` | Force-unlock a file |
| REST POST | `/api/files/move` | Move file/directory |
| REST POST | `/api/files/copy` | Copy file/directory |
| REST POST | `/api/bulk/delete` | Bulk delete multiple paths |
| REST POST | `/api/shares` | Create share link |
| REST GET | `/api/shares` | List share links |
| REST DELETE | `/api/shares/{token}` | Delete share link |
| REST GET | `/api/smart-collections` | List smart collections |
| REST POST | `/api/smart-collections` | Create smart collection |
| REST DELETE | `/api/smart-collections/{id}` | Delete smart collection |
| REST GET | `/api/files/{path}/versions` | List file versions |
| REST GET | `/api/files/{path}/versions/{id}` | Get version content |
| REST GET | `/api/files/{path}/diff?from={}&to={}` | Diff two versions |
| REST GET | `/api/quota` | Storage quota info |
| REST GET | `/api/activity` | Activity log |
| REST GET | `/api/branding` | Branding config |

#### User Interactions

- **View modes:** List, grid, graph (dependency graph), dual-pane
- **Navigation:** Breadcrumb path bar, sidebar panels
- **Selection:** Multi-select (Ctrl/Shift+click), select all
- **File operations:** Upload (drag-drop + button), download, delete, rename, move, copy, create directory
- **Context menu:** Right-click context menu with all file operations
- **Keyboard shortcuts:** Full keyboard navigation, `Ctrl+K` command palette
- **Share dialog:** Password protection, expiry, download tracking
- **Version history:** Visual diff viewer, version restore
- **Smart collections:** Rule-based saved searches
- **Sidebar panels:** Favorites, recent files, activity feed, locks, quota

#### State Signals (~30 per instance)

`entries`, `current_path`, `view_mode`, `sort_by`, `sort_order`, `selected`, `clipboard`, `clipboard_mode`, `show_upload`, `show_new_folder`, `show_share`, `show_version_history`, `show_path_dialog`, `path_dialog_mode`, `path_dialog_source`, `loading`, `error_msg`, `search_query`, `search_results`, `show_search`, `show_command_palette`, `favorites`, `recent_files`, `locks`, `show_sidebar`, `sidebar_panel`, `quota`, `activity_entries`, `smart_collections`, `drag_active`

---

### 2.2 Dashboard

**Route:** `/ui/dashboard`
**File:** `pages/dashboard.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/dashboard` | Dashboard summary (storage, recent files, shares, activity) |

#### User Interactions

- Quick action buttons (upload, new folder, share)
- Recent files display with file icons
- Shared links summary
- Activity feed preview
- Storage usage indicator

#### State

`loading`, `error_msg`, `dashboard_data` (storage_used, storage_total, file_count, recent_files, shared_files, activity)

---

### 2.3 Trash

**Route:** `/ui/trash`
**File:** `pages/trash.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/trash` | List trashed entries |
| POST | `/api/trash/restore` | Restore a trashed entry |
| DELETE | `/api/trash/purge` | Permanently delete one entry |
| DELETE | `/api/trash/empty` | Empty entire trash |

#### User Interactions

- List all trashed files with original path, delete date, size, MIME type
- Restore individual entries
- Purge individual entries (with confirmation)
- Empty entire trash (with confirmation dialog)

#### State

`loading`, `entries`, `show_empty_confirm`

---

### 2.4 Settings

**Route:** `/ui/settings`
**File:** `pages/settings.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/preferences` | Get user preferences |
| PUT | `/api/preferences` | Update user preferences |
| GET | `/api/config` | Get auth config |

#### Tabs

| Tab | Features |
|---|---|
| **Account** | Profile info, change password |
| **Preferences** | View mode, sort, items per page, show hidden files |
| **Notifications** | Browser notification permission request (local-only, not persisted to server) |
| **Appearance** | Theme selector (14 themes), font size |
| **Sync** | Placeholder — no offline implementation |

#### State

`loading`, `active_tab`, `preferences`, `show_password_change`

---

### 2.5 Admin (Enterprise)

**Route:** `/ui/admin`
**File:** `pages/admin.rs` (1063 lines)

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/storage/stats` | Storage statistics |
| GET | `/api/shares` | All share links |
| GET | `/api/audit` | Audit log entries |
| GET | `/api/admin/users` | List all users |
| GET | `/api/admin/users/{id}/devices` | User devices |
| POST | `/api/admin/users` | Create user |
| PUT | `/api/admin/users/{id}` | Update user |
| DELETE | `/api/admin/users/{id}` | Delete user |
| POST | `/api/admin/users/{id}/transfer` | Transfer ownership |
| GET | `/api/admin/dlp/policies` | List DLP policies |
| POST | `/api/admin/dlp/policies` | Create DLP policy |
| PUT | `/api/admin/dlp/policies/{id}` | Update DLP policy |
| DELETE | `/api/admin/dlp/policies/{id}` | Delete DLP policy |
| GET | `/api/admin/dlp/alerts` | List DLP alerts |
| POST | `/api/admin/antivirus/scan` | Trigger AV scan |
| GET | `/api/admin/antivirus/scans` | List AV scans |
| GET | `/api/admin/watermarks` | List watermark policies |
| POST | `/api/admin/watermarks` | Create watermark policy |
| PUT | `/api/admin/watermarks/{id}` | Update watermark policy |
| DELETE | `/api/admin/watermarks/{id}` | Delete watermark policy |
| GET | `/api/admin/notifications` | Notification preferences |
| PUT | `/api/admin/notifications` | Update notification prefs |

#### Tabs

| Tab | Features |
|---|---|
| **Overview** | Storage stats, share links, audit log |
| **Users** | CRUD users, role management, device management, ownership transfer |
| **DLP Policies** | CRUD data loss prevention policies |
| **DLP Alerts** | View DLP violation alerts |
| **Antivirus** | Trigger scans, view scan history |
| **Watermarks** | CRUD watermark overlay policies |
| **Notifications** | Admin notification preferences |

#### State (~50 signals)

`tab`, `loading`, `error_msg`, `storage_stats`, `share_links`, `audit_entries`, `users`, `show_create_user`, `new_user_*`, `editing_user`, `dlp_policies`, `dlp_alerts`, `show_create_policy`, `new_policy_*`, `av_scans`, `av_scan_running`, `watermark_policies`, `show_create_watermark`, `new_wm_*`, `notification_prefs`, `show_devices`, `selected_user_devices`, `transfer_source`, `transfer_target`

---

### 2.6 Calendar (CalDAV)

**Route:** `/ui/calendar`
**File:** `pages/calendar.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/calendar/events` | List events |
| POST | `/api/calendar/events` | Create event |
| PUT | `/api/calendar/events/{id}` | Update event |
| DELETE | `/api/calendar/events/{id}` | Delete event |

#### User Interactions

- Month, week, day views
- Create/edit/delete events
- Recurrence rules (RRULE)
- Event drag-resize
- All-day events

#### State

`loading`, `events`, `current_date`, `view_mode` (month/week/day), `selected_event`, `show_event_dialog`, `editing_event`

---

### 2.7 Contacts (CardDAV)

**Route:** `/ui/contacts`
**File:** `pages/contacts.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/contacts` | List contacts |
| POST | `/api/contacts` | Create contact |
| PUT | `/api/contacts/{id}` | Update contact |
| DELETE | `/api/contacts/{id}` | Delete contact |
| POST | `/api/contacts/import` | Import vCard |
| GET | `/api/contacts/export` | Export vCard |

#### User Interactions

- Contact list with search
- Create/edit/delete contacts
- vCard import/export
- Contact detail view

#### State

`loading`, `contacts`, `search_query`, `selected_contact`, `show_edit_dialog`

---

### 2.8 Notes

**Route:** `/ui/notes`
**File:** `pages/notes.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/notes` | List notes |
| POST | `/api/notes` | Create note |
| PUT | `/api/notes/{id}` | Update note |
| DELETE | `/api/notes/{id}` | Delete note |
| GET | `/api/notes/folders` | List folders |
| GET | `/api/notes/tags` | List tags |

#### User Interactions

- Markdown note editor
- Folder organization
- Tag system
- Search notes
- Preview rendered markdown

#### State

`loading`, `notes`, `folders`, `tags`, `selected_note`, `search_query`, `show_editor`, `editing_note`

---

### 2.9 Tasks

**Route:** `/ui/tasks`
**File:** `pages/tasks.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/tasks` | List tasks |
| POST | `/api/tasks` | Create task |
| PUT | `/api/tasks/{id}` | Update task |
| DELETE | `/api/tasks/{id}` | Delete task |

#### User Interactions

- Kanban board view (columns: todo, in-progress, done)
- Calendar view
- Drag-drop between columns
- Create/edit/delete tasks
- Task priority and due dates

#### State

`loading`, `tasks`, `view_mode` (kanban/calendar), `show_create_dialog`

---

### 2.10 Chat

**Route:** `/ui/chat`
**File:** `pages/chat.rs` (450 lines)

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/chat/rooms` | List chat rooms |
| POST | `/api/chat/rooms` | Create room |
| GET | `/api/chat/rooms/{id}/messages?limit=50` | Get message history |
| POST | `/api/chat/rooms/{id}/messages` | Send message |

#### User Interactions

- Room list sidebar
- Create new rooms
- Message history display
- Send messages
- @mentions

#### State

`loading`, `rooms`, `selected_room_id`, `messages`, `new_message`, `typing_users`, `show_create_room`, `new_room_name`, `error_msg`, `ws_connected`

#### BROKEN: WebSocket

- `ws_connected` signal is defined but never set to `true`
- No WebSocket connection code exists in the chat page
- Messages are fetched via REST polling only
- No real-time message delivery
- No typing indicator delivery

---

### 2.11 Photos

**Route:** `/ui/photos`
**File:** `pages/photos.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/photos` | List photos |
| GET | `/api/photos/{id}/exif` | Get EXIF metadata |
| GET | `/api/photos/albums` | List albums |

#### User Interactions

- Grid view with thumbnails
- Timeline view (grouped by date)
- Album organization
- Map view (location from EXIF)
- Lightbox viewer
- EXIF metadata display

#### State

`loading`, `photos`, `view_mode` (grid/timeline/album/map), `selected_photo`, `albums`

---

### 2.12 Mail

**Route:** `/ui/mail`
**File:** `pages/mail.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/mail/accounts` | List IMAP accounts |
| GET | `/api/mail/accounts/{id}/folders` | List folders |
| GET | `/api/mail/accounts/{id}/folders/{folder}/messages` | List messages |
| GET | `/api/mail/messages/{id}` | Get message |
| POST | `/api/mail/send` | Send message |
| POST | `/api/mail/accounts` | Add account |

#### User Interactions

- Account management (add/remove IMAP accounts)
- Folder navigation (INBOX, Sent, Drafts, Trash)
- Message list with preview
- Message detail view
- Compose new message
- Reply/forward

#### State

`loading`, `accounts`, `selected_account`, `folders`, `selected_folder`, `messages`, `selected_message`, `composing`, `compose_data`

---

### 2.13 Whiteboard

**Route:** `/ui/whiteboard`
**File:** `pages/whiteboard.rs` (725 lines)

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| POST | `/api/whiteboards` | Save whiteboard |
| GET | `/api/whiteboards/{id}` | Load whiteboard |

#### Drawing Tools

| Tool | Description |
|---|---|
| Pen | Freehand drawing |
| Line | Straight line |
| Rectangle | Rectangle shape |
| Circle | Circle/ellipse shape |
| Text | Text annotation |
| Eraser | Erase elements |

#### User Interactions

- Canvas drawing with multiple tools
- Color picker (12 preset colors)
- Stroke width selector (1, 2, 3, 5, 8, 12px)
- Undo/redo (local stack only)
- Pan and zoom viewport
- Save/load to server

#### State

`elements`, `current_tool`, `current_color`, `stroke_width`, `is_drawing`, `current_element`, `undo_stack`, `redo_stack`, `viewport`, `is_panning`, `pan_start`, `show_color_picker`, `show_stroke_picker`, `whiteboard_name`

#### BROKEN: No Collaborative Sync

- Whiteboard is single-user only
- No WebSocket or CRDT synchronization
- No presence indicators
- Changes are saved to server but not broadcast to other users

---

### 2.14 Analytics

**Route:** `/ui/analytics`
**File:** `pages/analytics.rs`

#### API Endpoints

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/api/shares` | Share link analytics |
| GET | `/api/activity` | Activity feed for charts |

#### User Interactions

- Share link analytics (views, downloads over time)
- Storage breakdown charts
- Activity timeline

#### State

`loading`, `share_data`, `activity_data`

---

## 3. What Works Well

| Area | Details |
|---|---|
| **Comprehensive file management** | Full CRUD via WebDAV + REST, multi-select, drag-drop upload, move/copy, bulk delete |
| **Enterprise admin** | 7-tab admin panel with DLP policies, antivirus, watermarks, user management, device management, audit logs |
| **14-theme design system** | Light, Dark, Midnight, System, Solarized Light/Dark, Nord, Tokyo Night, Dracula, High Contrast, Sepia, Forest, Ocean, Custom — all via CSS custom properties with smooth transitions |
| **Accessibility** | Skip navigation links, ARIA attributes, keyboard shortcuts throughout, FocusTrap component, `prefers-reduced-motion` support, high-contrast theme (WCAG AAA), touch target minimums (44px) |
| **Responsive design** | Mobile detection, responsive grid/list layouts, mobile-optimized spacing overrides at 640px breakpoint |
| **Version history** | Full version listing with visual diff (additions/deletions/unchanged), version restore |
| **Share links** | Password protection, expiry, download count tracking, allow-download/upload flags |
| **Smart collections** | Rule-based saved searches with auto-update |
| **Command palette** | `Ctrl+K` global command palette for quick navigation and actions |
| **Skeleton loading** | Shimmer animation skeleton states for perceived performance |
| **Branding** | Server-driven branding (logo, accent color, title, favicon, custom CSS) |
| **Toast notifications** | Toast notification system via Leptos context |
| **File previews** | EPUB preview, audio player, video player, markdown preview |
| **Dual-pane view** | Side-by-side file browser layout |
| **Graph view** | Dependency graph visualization for file relationships |
| **Custom views** | Configurable data table views |
| **Clipboard** | Copy/cut/paste file operations via clipboard context |
| **Onboarding** | First-run onboarding overlay and setup wizard |
| **Update check** | GitHub releases API integration for update notifications |
| **i18n infrastructure** | `t!()` macro with locale context, English only but extensible |
| **Activity sidebar** | Real-time activity feed in file browser sidebar |
| **Audio player** | Persistent audio player component across pages |
| **Photo map** | EXIF-based photo location map |
| **Lightbox** | Full-screen photo lightbox viewer |
| **Slideshow** | Photo slideshow functionality |

---

## 4. What's Broken

| Issue | Severity | Details |
|---|---|---|
| **Chat WebSocket never established** | Critical | `ws_connected` signal defined but never set to `true`. No WebSocket connection code exists. Messages fetched via REST polling only. No real-time delivery. |
| **Whiteboard has no sync mechanism** | Critical | Single-user only. No WebSocket or CRDT sync despite `ferro-crdt` crate being a dependency. No collaborative features. |
| **Settings notification prefs are local-only** | Medium | Notification permission request uses `Notification.requestPermission()` but preference is never persisted to server. |
| **Sync tab has no offline implementation** | Medium | Settings > Sync tab exists but has no actual offline/sync logic. Placeholder only. |
| **Many admin forms have incomplete submit handlers** | Medium | Several admin CRUD forms (DLP policies, watermarks) have create/edit dialogs but some submit handlers may be stubs. |
| **Photo map provider unclear** | Low | Map view exists but tile provider / API key configuration is not visible in code. |
| **Transcode API exists but no UI** | Medium | `api.rs` has `start_transcode`, `get_transcode_status`, `list_transcode_jobs` but no page or component calls them. |
| **File rename dialog unclear** | Low | Rename operation exists in file browser but the dialog UX path is not clearly wired in all cases. |
| **No actual offline support** | High | Service worker not registered. No cache-first strategies. No IndexedDB usage. The entire app requires network. |
| **Calendar recurrence value never included in iCal output** | Medium | Event creation form collects recurrence rules but the iCal serialization may not include RRULE in output. |
| **Notes tags not editable in UI** | Low | Tags exist in the data model but the UI has no tag editor component. |

---

## 5. CSS/Rendering Issues

| Issue | Impact | Details |
|---|---|---|
| **Missing CSS utility classes** | Visual bugs | Classes referenced in components but not defined in `style.css`: `table-fixed`, `whitespace-nowrap` (defined), `shadow-lg` (defined in theme CSS but not in `style.css`), `outline-none`, `pointer-events-none`, `select-none`, `overflow-hidden`, `border-collapse`, `table-auto`, `sticky`, `inset-0`, `z-50`, `z-10`, `absolute`, `relative`, `fixed`, `rounded-full`, `rounded-lg`, `divide-y`, `line-clamp-1`, `line-clamp-2`, `opacity-0`, `opacity-50`, `opacity-100`, `scale-95`, `scale-100`, `translate-x-0`, `-translate-y-1/2` |
| **No Tailwind — hand-written CSS with gaps** | Maintenance burden | `style.css` is 852 lines of manually maintained utility classes. Gaps between what components reference and what's defined. |
| **`#app` selector removed but Leptos mounts outside it** | Mount mismatch | `lib.rs` uses `mount_to_body` but some CSS may still reference `#app` |
| **Date column wrapping in table views** | Layout bug | Date/time columns wrap to multiple lines in narrow table viewports |
| **Table layout breaks with block display on tbody** | Layout bug | `tbody` elements with `display: block` break table column alignment |
| **Duplicate scroll containers in file browser** | Scroll issues | Multiple nested scroll containers create competing scroll behaviors |
| **Theme CSS duplication** | Bundle size | Each of 14 themes re-declares all spacing/typography tokens (~120 lines each) instead of inheriting from `:root` |
| **Style.css vs dark_mode.rs conflict** | Visual inconsistency | `style.css` defines its own color tokens (`:root { --accent: #E85D04 }`) that conflict with `dark_mode.rs` tokens (`--accent: #3b82f6`). Two competing design systems. |

---

## 6. Dependency Health

| Dependency | Version | Status |
|---|---|---|
| `leptos` | 0.8 (workspace) | Current |
| `leptos_router` | 0.8 | Current |
| `wasm-bindgen` | 0.2 | Current |
| `wasm-bindgen-futures` | 0.4 | Current |
| `web-sys` | 0.3 | Current (extensive feature flags) |
| `js-sys` | 0.3 | Current |
| `serde` / `serde_json` | workspace / 1 | Current |
| `chrono` | 0.4 (serde feature) | Current |
| `uuid` | 1 (v4, js features) | Current |
| `console_error_panic_hook` | 0.1 | Current |
| `ferro-common` | path dependency | Internal |
| `ferro-crdt` | path dependency (wasm feature) | Internal — imported but CRDT not used in frontend |
| Tauri | 2.x | Current |
| webkit2gtk | 2.x | Linux only |
| **TypeScript type safety** | None | No TS in the stack — all API contracts are Rust structs with serde |
| **WASM binary size** | ~3.8 MB | Acceptable for release, could be optimized |

---

## 7. Backend-Frontend Gap Analysis

The following backend features exist in the Ferro server crates but have **no corresponding frontend UI**:

### 7.1 API Protocols

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **GraphQL API** | `crates/graphql/` | Not used — frontend uses REST + WebDAV exclusively |
| **WOPI Office editing** | `crates/server-wopi/` | No UI for launching collaborative editing sessions |
| **WebRTC signaling** | `crates/server-webrtc/` | No peer-to-peer UI or signaling interface |

### 7.2 Media & Streaming

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **Video streaming/transcoding** | `crates/server-content/` (transcode routes) | API functions exist in `api.rs` but no UI page or component uses them |
| **Transcode job management** | Same | No progress monitoring UI |

### 7.3 Federation & Protocol Bridges

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **Federation (ActivityPub)** | `crates/server-activitypub/`, `crates/server-federation/` | No federation management UI |
| **CalDAV (full server)** | `crates/caldav/` | Frontend uses REST bridge (`/api/calendar/*`), not direct CalDAV |
| **CardDAV (full server)** | `crates/dav/` | Frontend uses REST bridge (`/api/contacts/*`), not direct CardDAV |

### 7.4 Sync & Offline

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **Selective sync profiles** | `crates/selective-sync/` | No UI for configuring sync rules |
| **Offline mode with conflict resolution** | `crates/offline/` | No offline support in frontend at all |
| **Sync protocol** | `crates/sync-protocol/` | No sync UI |
| **CRDT collaboration** | `crates/crdt/` | Imported as dependency but not wired to any frontend component |

### 7.5 Notifications & Push

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **Push notifications (FCM/APNs)** | Backend routes exist | Frontend only uses browser `Notification` API (local) |

### 7.6 File Management

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **File requests (upload-only links)** | Backend routes exist | No UI for creating file request links |
| **Remote mount proxy** | `crates/mount-nfs/` | No UI for managing remote mounts |
| **Presigned URL generation** | Backend routes exist | No UI for generating time-limited URLs |

### 7.7 Compliance & Security

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **Backup/restore UI** | Backend routes exist | No backup management interface |
| **GDPR export/erasure UI** | Backend routes exist | No data subject request interface |
| **Compliance reports** | `crates/server-compliance/` | No compliance report viewer |
| **WORM policies** | Backend routes exist | No write-once-read-many policy management UI |
| **Data retention policies** | Backend routes exist | No retention policy configuration UI |
| **E2EE key generation UI** | `crates/crypto/` | No end-to-end encryption key management UI |
| **LDAP configuration UI** | Backend routes exist | No LDAP/AD configuration interface |
| **SCIM provisioning** | `crates/scim/` | No SCIM provisioning management UI |

### 7.8 Enterprise & Administration

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **Plugin marketplace** | `crates/server-plugins/`, `crates/plugin/` | No plugin browsing/installation UI |
| **WASM worker upload** | `crates/wasm-host/` | No WASM plugin upload interface |
| **Event triggers/automation** | `crates/server-automation/` | No automation rule builder UI |
| **Tenant rate limiting UI** | `crates/rate-limiter/` | No rate limit configuration interface |
| **Branding customization UI** | Backend serves branding config | Frontend reads branding but no admin UI to customize it |
| **Device management UI** | Backend routes exist | Admin page has basic device listing but no full device management |
| **Group management UI** | `crates/server-user-mgmt/` | No group CRUD UI |
| **Prometheus metrics UI** | `crates/observability/` | No metrics dashboard |
| **OpenAPI/Swagger browsing** | Backend serves spec | No API documentation browser in frontend |
| **Circuit breaker status** | `crates/circuit-breaker/` | No circuit breaker monitoring UI |
| **Ransomware detection alerts** | Backend routes exist | No alert viewer or response UI |

### 7.9 Infrastructure

| Backend Feature | Backend Crate | Frontend Status |
|---|---|---|
| **Distributed consensus** | `crates/distributed/` | No cluster management UI |
| **Cache management** | `crates/cache/` | No cache invalidation UI |
| **Health checks** | `crates/health/`, `crates/server-health/` | No health dashboard |
| **Chaos engineering** | `crates/chaos/` | No chaos experiment UI |
| **FIPS compliance** | `crates/server-fips/` | No FIPS status UI |
| **SLO tracking** | `crates/server-slo/` | No SLO dashboard |

### 7.10 Summary Count

| Category | Backend Features | Frontend Coverage |
|---|---|---|
| Core file operations | ~15 endpoints | **Covered** |
| Enterprise admin | ~20 endpoints | **Partially covered** (7 tabs, some incomplete) |
| Calendar/Contacts | ~8 endpoints | **Covered** (via REST bridge) |
| Collaboration | ~6 features | **Not covered** (Chat broken, Whiteboard single-user, no CRDT UI) |
| Compliance/Security | ~10 features | **Not covered** |
| Media/Streaming | ~4 features | **Not covered** (API exists, no UI) |
| Federation/Protocol | ~3 features | **Not covered** |
| Sync/Offline | ~4 features | **Not covered** |
| Enterprise/Infra | ~12 features | **Not covered** |
| **Total gap** | **~35+ backend features** | **No frontend counterpart** |

---

## Appendix A: Component Inventory

| Component | File | Purpose |
|---|---|---|
| `FileBrowser` | `components/file_browser/` | Main file browser (directory listing, views) |
| `Header` | `components/header.rs` | Top navigation bar |
| `NavigationSidebar` | `components/navigation.rs` | Side navigation |
| `CommandPalette` | `components/command_palette.rs` | Ctrl+K command palette |
| `DataTable` | `components/data_table.rs` | Generic sortable/filterable table |
| `DualPane` | `components/dual_pane.rs` | Side-by-side layout |
| `GraphView` | `components/graph_view.rs` | Dependency graph visualization |
| `GridView` | `components/grid_view.rs` | Grid layout for files/photos |
| `ShareDialog` | `components/share_dialog.rs` | Share link creation |
| `VersionHistory` | `components/version_history.rs` | File version browser + diff |
| `Dialog` | `components/dialog.rs` | Modal dialog primitive |
| `FocusTrap` | `components/focus_trap.rs` | Keyboard focus management |
| `Toast` | `components/toast.rs` | Toast notification system |
| `Skeleton` | `components/skeleton.rs` | Loading skeleton |
| `Tooltip` | `components/tooltip.rs` | Tooltip component |
| `ThemeToggle` | `components/theme_toggle.rs` | Theme switcher + state provider |
| `AudioPlayer` | `components/audio_player.rs` | Persistent audio player |
| `VideoPlayer` | `components/video_player.rs` | Video player component |
| `FilePreview` | `components/file_preview.rs` | File content preview |
| `EpubPreview` | `components/epub_preview.rs` | EPUB reader |
| `MarkdownEditor` | `components/markdown_editor.rs` | Markdown note editor |
| `BlockEditor` | `components/block_editor.rs` | Block-based editor |
| `PhotoEditor` | `components/photo_editor.rs` | Photo editing tools |
| `PhotoMap` | `components/photo_map.rs` | EXIF-based photo map |
| `Slideshow` | `components/slideshow.rs` | Photo slideshow |
| `Clipboard` | `components/clipboard.rs` | Copy/cut/paste state |
| `BulkActionBar` | `components/bulk_action_bar.rs` | Multi-select action bar |
| `DeleteConfirm` | `components/delete_confirm.rs` | Delete confirmation dialog |
| `UploadDialog` | `components/upload_dialog.rs` | File upload dialog |
| `NewFolderDialog` | `components/new_folder_dialog.rs` | Create folder dialog |
| `PathDialog` | `components/path_dialog.rs` | Path input dialog (move/copy) |
| `SmartCollectionsSidebar` | `components/smart_collections_sidebar.rs` | Smart collections panel |
| `ActivitySidebar` | `components/activity_sidebar.rs` | Activity feed sidebar |
| `ErrorBoundary` | `components/error_boundary.rs` | Error boundary |
| `OnboardingOverlay` | `components/onboarding.rs` | First-run onboarding |
| `SetupWizard` | `components/setup_wizard.rs` | Initial setup wizard |
| `EmptyState` | `components/empty_state.rs` | Empty state placeholder |
| `DragHint` | `components/drag_hint.rs` | Drag-drop hint overlay |
| `ScrollSentinel` | `components/scroll_sentinel.rs` | Infinite scroll sentinel |
| `FileIcon` | `components/file_icon.rs` | File type icon |
| `FileRow` | `components/file_row.rs` | File list row |
| `Thumbnail` | `components/thumbnail.rs` | Image thumbnail |
| `Icons` | `components/icons.rs` | SVG icon library |
| `Chart` | `components/chart.rs` | Chart component |
| `Animate` | `components/animate.rs` | CSS animation helpers |
| `SampleFiles` | `components/sample_files.rs` | Demo/sample file generation |
| `CustomView` | `components/custom_view.rs` | Configurable custom views |
| `KeyboardShortcutsHelp` | `components/keyboard_shortcuts_help.rs` | Keyboard shortcuts reference |
| `Collaboration` | `components/collaboration.rs` | Collaboration indicators (unused) |
| `Primitives` | `components/primitives.rs` | Shared primitive components |

---

## Appendix B: Style System Architecture

The frontend has **two competing CSS systems**:

1. **`style.css`** (852 lines) — Hand-written utility classes inspired by Tailwind, with a custom "Spatial Materialism" design language. Defines its own color tokens at `:root` that conflict with the theme system.

2. **`dark_mode.rs` THEME_CSS** (~1600 lines injected at runtime) — 14 complete theme definitions using `[data-theme="..."]` selectors with comprehensive CSS custom properties. Includes utility classes, scrollbar styling, skeleton animations, focus rings, reduced motion, and responsive overrides.

**Conflict:** `style.css` line 8 sets `--accent: #E85D04` while `dark_mode.rs` sets `--accent: #3b82f6` for the light theme. Both inject into `:root`. The runtime injection order determines which wins.

---

*End of audit.*

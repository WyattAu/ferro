# GUI Rewrite Roadmap

> Complete implementation plan for the Ferro frontend rewrite. 24 weeks, 8 phases, from foundation to production-ready.

---

## Timeline Overview

```
Week  1──2   Phase 0: Foundation
Week  3──5   Phase 1: Core File Management
Week  6──8   Phase 2: Collaboration Features
Week  9──11  Phase 3: Media & Content
Week 12──14  Phase 4: Admin & Enterprise
Week 15──17  Phase 5: Advanced Features
Week 18──20  Phase 6: Desktop & Mobile
Week 21──24  Phase 7: Polish & Hardening
```

| Phase | Weeks | Endpoints Covered | Cumulative Coverage |
|-------|-------|-------------------|---------------------|
| 0     | 1-2   | 0 (foundation)    | 0%                  |
| 1     | 3-5   | ~40 (file APIs)   | 27%                 |
| 2     | 6-8   | ~35 (collab APIs) | 50%                 |
| 3     | 9-11  | ~20 (media APIs)  | 63%                 |
| 4     | 12-14 | ~25 (admin APIs)  | 80%                 |
| 5     | 15-17 | ~15 (advanced)    | 90%                 |
| 6     | 18-20 | 0 (desktop)       | 90%                 |
| 7     | 21-24 | ~15 (polish)      | 100%                |

**Note:** All API paths use the `/api/v1/` prefix as specified in GUI_ARCHITECTURE.md Section 5.2.

---

## Phase 0: Foundation (Week 1-2)

> Build the bedrock. Everything downstream depends on these decisions.

### Objectives
- Establish crate structure and build pipeline
- Define design system (tokens, utilities, theme)
- Generate type-safe API client
- Build WebSocket manager
- Create primitive component library
- Set up testing infrastructure

### Deliverables

| Deliverable                    | Details                                                | Acceptance Criteria                            |
|-------------------------------|--------------------------------------------------------|------------------------------------------------|
| Crate structure                | New `ferro-frontend` crate with module layout          | `cargo build --target wasm32-unknown-unknown` succeeds |
| Design tokens                  | CSS custom properties for colors, spacing, typography, shadows | Token file compiles, values match `dark_mode.rs` |
| Utility CSS system             | Complete utility classes (flex, grid, spacing, typography, state) | Zero missing classes on any layout pattern |
| Theme system                   | Dark/light mode toggle, CSS custom property swapping   | All tokens resolve correctly in both modes      |
| API client generation          | TOML schema → Rust client codegen via `build.rs`      | All 150+ endpoints have typed request/response  |
| HTTP client                    | Fetch-based with retry, timeout, CSRF token injection  | Handles 401, 429, 5xx correctly                |
| WebSocket manager              | Auto-reconnect, message queue, subscription model     | Survives disconnect/reconnect, delivers messages |
| Primitive components           | Button, Input, Select, Dialog, Toast, Tooltip, Badge, Avatar, Spinner, Divider | All pass accessibility checks |
| Layout components              | Shell, Sidebar, Header, ContentArea, SplitPane, Stack, Grid | Responsive from 320px to 4K                     |
| State architecture             | Global state provider, feature state pattern           | Context provider works, signals propagate       |
| Route system                   | LazyRoute with code splitting, route guards            | Routes load on demand, auth guard works         |
| Testing infrastructure         | `wasm-bindgen-test` setup, Playwright config, CI integration | Tests run in CI, screenshot comparison works    |
| Error boundary                 | Top-level error catch with fallback UI                 | Catches render errors, logs, shows recovery UI  |

### Week 1

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Create crate, set up Trunk, verify WASM build               |
| Tue   | Design tokens CSS file, theme provider component            |
| Wed   | Utility CSS system (flex, grid, spacing, typography)        |
| Thu   | TOML API schema for 5 sample endpoints, codegen pipeline    |
| Fri   | HTTP client with retry/timeout/CSRF, error handling         |

### Week 2

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | WebSocket manager with auto-reconnect                       |
| Tue   | Primitive components: Button, Input, Select                  |
| Wed   | Primitive components: Dialog, Toast, Tooltip                 |
| Thu   | Layout components: Shell, Sidebar, Header, ContentArea      |
| Fri   | State architecture, route system, error boundary, tests     |

### Milestone: Foundation Complete
- [ ] `trunk serve` starts dev server with theme toggle
- [ ] All primitives render correctly in both themes
- [ ] API client generates and compiles
- [ ] WebSocket connects and reconnects
- [ ] Unit tests pass for all primitives
- [ ] No accessibility violations in primitive components

---

## Phase 1: Core File Management (Week 3-5)

> The primary use case. Must feel fast and complete.

### Objectives
- Implement full file browser with all CRUD operations
- Upload/download with progress and drag-drop
- Search, favorites, recent files
- Version history with diff
- Share links, lock management
- Trash with restore/purge

### Deliverables

| Deliverable               | Endpoints | Details                                         |
|--------------------------|-----------|--------------------------------------------------|
| File browser (list view) | GET /api/v1/files | Breadcrumb nav, sorting, pagination              |
| File browser (grid view) | GET /api/v1/files | Thumbnail previews, selection                    |
| File upload              | POST /api/v1/files/upload | Drag-drop zone, progress bar, multi-file   |
| File download            | GET /api/v1/files/:id/download | Progress tracking, resume support          |
| Create folder            | POST /api/v1/files/mkdir | Inline naming, validation                       |
| Delete files             | DELETE /api/v1/files/:id | Confirmation dialog, soft delete to trash     |
| Move files               | POST /api/v1/files/move | Drag-drop or dialog destination picker          |
| Copy files               | POST /api/v1/files/copy | Duplicate with naming conflict resolution        |
| Rename files             | POST /api/v1/files/rename | Inline edit, validation                        |
| Search files             | GET /api/v1/files/search | Full-text, filters (type, date, size, owner)  |
| Favorites                | POST /api/v1/files/favorite | Toggle, filter by favorites                    |
| Recent files             | GET /api/v1/files/recent | Last 20 accessed files                          |
| Smart collections        | GET /api/v1/collections | Rule-based dynamic folders                      |
| Version history          | GET /api/v1/files/:id/versions | List versions, download any                  |
| Version diff             | GET /api/v1/files/:id/versions/:v/diff | Side-by-side diff                   |
| Share links              | POST /api/v1/files/:id/share | Generate link, expiry, password protection   |
| Lock management          | POST /api/v1/files/:id/lock | Lock/unlock, show lock status                  |
| Trash (list)             | GET /api/v1/trash | View deleted items                               |
| Trash (restore)          | POST /api/v1/trash/:id/restore | Restore to original path                    |
| Trash (purge)            | DELETE /api/v1/trash/:id | Permanent delete, confirmation                 |
| Trash (empty)            | DELETE /api/v1/trash | Empty all, confirmation                         |

### Week 3

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | File browser list view with columns (name, size, date, type) |
| Tue   | Breadcrumb navigation component                             |
| Wed   | Grid view with thumbnail placeholders                       |
| Thu   | Sort controls (name, date, size, type) + pagination         |
| Fri   | File selection (single, multi, shift-click, ctrl-click)     |

### Week 4

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Upload zone: drag-drop, button trigger, file validation     |
| Tue   | Upload progress bar, multi-file queue, cancel support       |
| Wed   | Download with progress tracking, resume                     |
| Thu   | Create folder, delete (with confirmation), move, copy       |
| Fri   | Rename (inline edit), context menu for all operations       |

### Week 5

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Search with full-text and filter UI                         |
| Tue   | Favorites toggle, recent files list, smart collections      |
| Wed   | Version history panel, diff viewer                          |
| Thu   | Share links, lock management                                |
| Fri   | Trash view, restore, purge, empty                           |

### Milestone: Core File Management Complete
- [ ] All file CRUD operations work end-to-end
- [ ] Upload supports drag-drop, multi-file, progress
- [ ] Search returns relevant results with filters
- [ ] Version history shows changes with diff
- [ ] Share links generate and expire correctly
- [ ] Trash restore/purge works with confirmation
- [ ] All operations have optimistic updates
- [ ] Keyboard shortcuts work for all operations
- [ ] Screen reader announces all actions

---

## Phase 2: Collaboration Features (Week 6-8)

> Notes, tasks, calendar, contacts, chat. The productivity layer.

### Objectives
- Markdown notes editor with live preview
- Kanban task board with drag-drop
- Calendar with event management
- Contact management with vCard
- Real-time chat with WebSocket
- File comments and tagging system

### Deliverables

| Deliverable               | Endpoints | Details                                         |
|--------------------------|-----------|--------------------------------------------------|
| Notes (list)             | GET /api/v1/notes | Folder tree, search, tags                        |
| Notes (editor)           | POST /api/v1/notes | Markdown editor, live preview, split view       |
| Notes (folders)          | CRUD /api/v1/notes/folders | Folder CRUD, drag to organize               |
| Notes (tags)             | CRUD /api/v1/notes/:id/tags | Tag assignment, filter by tag               |
| Tasks (list)             | GET /api/v1/tasks | Filter by status, assignee, priority, due date  |
| Tasks (Kanban)           | PUT /api/v1/tasks/:id | Drag-drop between columns                   |
| Tasks (calendar view)    | GET /api/v1/tasks | Tasks on calendar grid                          |
| Tasks (CRUD)             | CRUD /api/v1/tasks | Create, edit, delete, complete                 |
| Calendar (month)         | GET /api/v1/events | Month grid with event dots                      |
| Calendar (week)          | GET /api/v1/events | Week view with time slots                       |
| Calendar (day)           | GET /api/v1/events | Day view with detailed timeline                 |
| Calendar (CRUD)          | CRUD /api/v1/events | Create, edit, delete events                    |
| Contacts (list)          | GET /api/v1/contacts | Search, groups, sort                            |
| Contacts (detail)        | GET /api/v1/contacts/:id | Full vCard view, edit                          |
| Contacts (import/export) | POST /api/v1/contacts/import, GET /api/v1/contacts/export | vCard/CSV support         |
| Chat (rooms)             | WS /api/v1/chat | Room list, create, join                         |
| Chat (messages)          | WS /api/v1/chat | Real-time messages, @mentions, reactions        |
| Chat (history)           | GET /api/v1/chat/history | Load older messages, scroll back           |
| File comments            | CRUD /api/v1/files/:id/comments | Threaded comments, resolve               |
| Tags (system)            | CRUD /api/v1/tags | Create, rename, delete, assign to any entity    |

### Week 6

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Notes: folder tree, list view, create/edit/delete           |
| Tue   | Notes: markdown editor with syntax highlighting             |
| Wed   | Notes: live preview, split view toggle                      |
| Thu   | Notes: tags, search, sort                                   |
| Fri   | Tasks: list view, CRUD, status management                   |

### Week 7

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Tasks: Kanban board with drag-drop between columns          |
| Tue   | Tasks: calendar view showing task due dates                 |
| Wed   | Calendar: month view with event dots                        |
| Thu   | Calendar: week/day views, event CRUD                        |
| Fri   | Contacts: list, detail view, vCard rendering                |

### Week 8

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Contacts: import/export (vCard, CSV)                        |
| Tue   | Chat: WebSocket rooms, message send/receive                 |
| Wed   | Chat: @mentions, reactions, message history                  |
| Thu   | File comments: threaded comments, resolve                   |
| Fri   | Tags system: CRUD, assignment UI, filtering                 |

### Milestone: Collaboration Complete
- [ ] Notes editor saves automatically, preview renders correctly
- [ ] Kanban drag-drop works, persists column position
- [ ] Calendar shows events, CRUD works in all views
- [ ] Contacts import/export works with vCard standard
- [ ] Chat delivers messages in real-time via WebSocket
- [ ] File comments are threaded and resolvable
- [ ] Tags can be assigned to any entity and filtered

---

## Phase 3: Media & Content (Week 9-11)

> Photos, video, audio, whiteboard, file previews.

### Objectives
- Photo gallery with grid/timeline/album views
- EXIF data display and map view
- Video streaming with custom player
- Audio player with playlist
- Whiteboard canvas with drawing tools
- File preview for markdown, CSV, HTML, code
- Thumbnail generation and caching

### Deliverables

| Deliverable               | Endpoints | Details                                         |
|--------------------------|-----------|--------------------------------------------------|
| Photos (grid)            | GET /api/v1/photos | Masonry layout, lazy loading                     |
| Photos (timeline)        | GET /api/v1/photos | Chronological view with date headers            |
| Photos (albums)          | CRUD /api/v1/albums | Album CRUD, add/remove photos                 |
| Photos (lightbox)        | — | Full-screen view, swipe, keyboard nav            |
| Photos (EXIF)            | GET /api/v1/photos/:id/exif | Camera, date, GPS, settings display        |
| Photos (map)             | GET /api/v1/photos/map | Photos plotted by GPS coordinates             |
| Video player             | GET /api/v1/video/:id/stream | Range request streaming, custom controls  |
| Video playlist           | — | Queue, shuffle, repeat                           |
| Audio player             | GET /api/v1/audio/:id/stream | Persistent player, playlist, shuffle      |
| Audio waveform           | — | Waveform visualization for navigation            |
| Whiteboard (canvas)      | CRUD /api/v1/whiteboard | Drawing tools: pen, shapes, text, eraser    |
| Whiteboard (collab)      | WS /api/v1/whiteboard | Real-time cursor tracking, sync              |
| Whiteboard (export)      | GET /api/v1/whiteboard/:id/export | PNG, SVG, PDF export                 |
| Preview (markdown)       | — | Rendered markdown with syntax highlighting      |
| Preview (CSV)            | — | Scrollable table with sort                      |
| Preview (HTML)           | — | Sandboxed iframe rendering                      |
| Preview (code)           | — | Syntax highlighted code block                   |
| Thumbnails               | — | On-demand generation, cached, responsive sizes   |

### Week 9

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Photo grid: masonry layout, lazy loading, selection         |
| Tue   | Photo timeline: chronological grouping, scroll              |
| Wed   | Photo albums: CRUD, add/remove, album grid                  |
| Thu   | Photo lightbox: full-screen, swipe, keyboard navigation     |
| Fri   | Photo EXIF: metadata display, camera info                   |

### Week 10

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Photo map: GPS plotting, cluster markers                    |
| Tue   | Video player: range request streaming, custom controls      |
| Wed   | Video playlist, audio player with persistent bar            |
| Thu   | Audio waveform visualization, playlist management           |
| Fri   | Whiteboard: canvas with pen, shapes, text tools             |

### Week 11

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Whiteboard: real-time collaboration via WebSocket           |
| Tue   | Whiteboard: export to PNG/SVG/PDF                           |
| Wed   | File preview: markdown renderer                             |
| Thu   | File preview: CSV table, HTML sandbox, code highlighting    |
| Fri   | Thumbnail generation, caching, responsive sizes             |

### Milestone: Media Complete
- [ ] Photo gallery loads fast with lazy loading
- [ ] EXIF data displays correctly
- [ ] Video streams without buffering (range requests)
- [ ] Audio player persists across page navigation
- [ ] Whiteboard supports real-time collaboration
- [ ] All file previews render correctly
- [ ] Thumbnails generate on-demand and cache

---

## Phase 4: Admin & Enterprise (Week 12-14)

> User management, DLP, compliance, automation. The enterprise layer.

### Objectives
- User management with roles and permissions
- DLP policies and violation alerts
- Antivirus scanning UI
- Watermark policies
- Audit log viewer
- Compliance reports
- Backup/restore UI
- Branding customization
- Plugin marketplace
- WASM worker management
- Event triggers and automation
- Tenant management
- Device management
- GDPR export/erasure

### Deliverables

| Deliverable               | Endpoints | Details                                         |
|--------------------------|-----------|--------------------------------------------------|
| User management          | CRUD /api/v1/users | List, create, edit, deactivate, password reset |
| Roles & permissions      | CRUD /api/v1/roles | Role hierarchy, permission matrix              |
| DLP policies             | CRUD /api/v1/dlp | Create policies, view violations               |
| DLP alerts               | GET /api/v1/dlp/alerts | Real-time violation notifications             |
| Antivirus UI             | GET /api/v1/files/:id/scan | Scan status, results, re-scan                |
| Watermark policies       | CRUD /api/v1/watermarks | Text/image watermarks, preview              |
| Audit log viewer         | GET /api/v1/audit | Filterable, exportable, real-time streaming    |
| Compliance reports       | GET /api/v1/compliance | Generate PDF reports, schedule               |
| Backup/restore           | POST /api/v1/backup, POST /api/v1/restore | Full/incremental, progress UI         |
| Branding                 | PUT /api/v1/branding | Logo, colors, custom domain                    |
| Plugin marketplace       | GET /api/v1/plugins | Browse, install, configure, enable/disable     |
| WASM workers             | CRUD /api/v1/workers | Upload, start, stop, monitor, logs            |
| Event triggers           | CRUD /api/v1/triggers | Automation rules, conditions, actions        |
| Tenant management        | CRUD /api/v1/tenants | Multi-tenant admin, resource limits           |
| Device management        | GET /api/v1/devices | Connected devices, revoke, trust               |
| GDPR export              | POST /api/v1/gdpr/export | User data export, download                  |
| GDPR erasure             | POST /api/v1/gdpr/erasure | Account deletion, confirmation             |

### Week 12

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | User management: list, create, edit, deactivate             |
| Tue   | Roles & permissions: matrix UI, assignment                  |
| Wed   | DLP policies: create/edit policies, condition builder       |
| Thu   | DLP alerts: real-time notification, violation details       |
| Fri   | Antivirus: scan status, results, re-scan trigger            |

### Week 13

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Watermark policies: text/image config, preview              |
| Tue   | Audit log: filterable table, search, export                 |
| Wed   | Compliance reports: report generation, PDF preview          |
| Thu   | Backup/restore: full/incremental, progress UI               |
| Fri   | Branding: logo upload, color picker, custom domain          |

### Week 14

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Plugin marketplace: browse, install, configure              |
| Tue   | WASM workers: upload, monitor, logs                         |
| Wed   | Event triggers: rule builder, condition/action UI           |
| Thu   | Tenant management: resource limits, admin delegation        |
| Fri   | Device management, GDPR export/erasure                      |

### Milestone: Admin Complete
- [ ] User CRUD works with role-based access
- [ ] DLP policies enforce and alert on violations
- [ ] Audit log captures and displays all events
- [ ] Backup/restore works end-to-end
- [ ] Plugin marketplace installs plugins correctly
- [ ] GDPR export/erasure complies with regulations

---

## Phase 5: Advanced Features (Week 15-17)

> Offline mode, conflict resolution, notifications, remote mounts, and more.

### Objectives
- Offline mode with IndexedDB cache
- Conflict resolution UI
- Push notification preferences
- File requests (upload-only links)
- Remote mount management
- Selective sync profiles
- GraphQL explorer
- WebRTC video calls
- WOPI office editing integration
- Federation (ActivityPub) UI

### Deliverables

| Deliverable               | Details                                                 |
|--------------------------|---------------------------------------------------------|
| Offline mode             | IndexedDB cache, offline indicator, queue mutations     |
| Conflict resolution      | UI for resolving conflicting edits (side-by-side)       |
| Push notifications       | Preference center, browser notification API integration |
| File requests            | Generate upload-only links, track submissions           |
| Remote mounts            | Add/edit/remove remote storage connections              |
| Selective sync           | Choose which directories sync, bandwidth limits         |
| GraphQL explorer         | Interactive query builder, schema documentation         |
| WebRTC calls             | Peer-to-peer video calls, screen sharing                |
| WOPI integration         | Edit Office docs in-browser via WOPI protocol           |
| Federation (ActivityPub) | Share files across instances, follow users              |

### Week 15

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Offline mode: IndexedDB cache layer                         |
| Tue   | Offline mode: offline indicator, queue management           |
| Wed   | Conflict resolution: side-by-side diff UI                   |
| Thu   | Push notification preferences center                        |
| Fri   | File requests: generate link, track submissions             |

### Week 16

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Remote mounts: add/edit/remove storage connections          |
| Tue   | Selective sync: directory picker, bandwidth settings        |
| Wed   | GraphQL explorer: query builder, schema docs                |
| Thu   | WebRTC: peer-to-peer video call setup                       |
| Fri   | WebRTC: screen sharing, call controls                       |

### Week 17

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | WOPI: Office document editing integration                   |
| Tue   | Federation: ActivityPub share/follow UI                     |
| Wed   | Integration testing for all Phase 5 features                |
| Thu   | Bug fixes and edge cases                                    |
| Fri   | Phase 5 milestone review                                    |

### Milestone: Advanced Features Complete
- [ ] Offline mode caches and syncs correctly
- [ ] Conflict resolution shows diff and resolves
- [ ] Push notifications respect user preferences
- [ ] File requests generate working upload links
- [ ] Remote mounts connect and browse successfully
- [ ] WebRTC calls establish and maintain connection

---

## Phase 6: Desktop & Mobile (Week 18-20)

> Tauri integration, system tray, OS features, mobile optimization.

### Objectives
- Tauri integration testing across platforms
- System tray with context menu
- Mount/sync lifecycle management
- OS notifications
- Shell integration (context menu, autostart)
- Auto-update mechanism
- Mobile responsive optimization
- Platform-specific features (Android/iOS)

### Deliverables

| Deliverable               | Platform   | Details                                        |
|--------------------------|------------|------------------------------------------------|
| Tauri integration test   | All        | Verify all features work in desktop wrapper     |
| System tray              | All        | Minimize to tray, context menu, status icon     |
| Mount/sync lifecycle     | All        | Mount directories, sync status, conflict handling |
| OS notifications         | All        | Tauri notification plugin integration           |
| Context menu integration | All        | Right-click file → OS context menu              |
| Autostart                | All        | Launch on system boot, configurable             |
| Auto-update              | All        | Check for updates, download, prompt install     |
| File associations        | All        | Open files with Ferro from OS                   |
| Mobile responsive        | Web        | Touch-friendly UI, swipe gestures               |
| Android features         | Android    | Share intent, file picker, background sync      |
| iOS features             | iOS        | Share extension, file picker, background fetch  |

### Week 18

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Tauri integration: verify all features in desktop wrapper   |
| Tue   | System tray: minimize, restore, context menu                |
| Wed   | Mount/sync lifecycle: mount directories, status display     |
| Thu   | OS notifications: Tauri notification plugin                 |
| Fri   | Shell integration: context menu, autostart, file associations |

### Week 19

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Auto-update: check, download, prompt                        |
| Tue   | Mobile responsive: breakpoints, touch targets, swipe        |
| Wed   | Android: share intent, file picker, background sync         |
| Thu   | iOS: share extension, file picker, background fetch         |
| Fri   | Cross-platform testing: Linux, macOS, Windows               |

### Week 20

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Platform-specific bug fixes                                 |
| Tue   | Performance optimization for mobile                         |
| Wed   | Memory optimization for desktop                             |
| Thu   | Integration testing across all platforms                    |
| Fri   | Phase 6 milestone review                                    |

### Milestone: Desktop & Mobile Complete
- [ ] Desktop app works on Linux, macOS, Windows
- [ ] System tray functions correctly
- [ ] Auto-update downloads and installs
- [ ] Mobile layout is usable at 320px-768px
- [ ] Android share intent works
- [ ] iOS share extension works

---

## Phase 7: Polish & Hardening (Week 21-24)

> Accessibility, performance, security, testing, documentation, i18n.

### Objectives
- Accessibility audit (WCAG 2.1 AA)
- Performance optimization (Lighthouse scores)
- Security audit (OWASP ZAP)
- Visual regression testing
- E2E test suite completion
- Documentation
- Internationalization (5+ languages)
- Onboarding flow
- Keyboard shortcuts reference
- Dark/light theme testing across all components

### Deliverables

| Deliverable               | Details                                                 |
|--------------------------|---------------------------------------------------------|
| Accessibility audit      | axe-core scan, manual keyboard testing, screen reader   |
| WCAG compliance          | Fix all AA violations, document any AAA achievements    |
| Performance optimization | Lighthouse >90, bundle size targets met                 |
| Security audit           | OWASP ZAP scan, fix all high/medium findings            |
| Visual regression        | Screenshot tests for all components in both themes      |
| E2E test suite           | Playwright tests for all critical user journeys         |
| Documentation            | Component API docs, architecture guide, contributing    |
| i18n (5 languages)       | EN, ES, FR, DE, JA, ZH translations                    |
| Onboarding flow          | First-run experience, feature highlights, tooltips      |
| Keyboard shortcuts       | Reference panel, discoverable shortcuts                 |
| Theme testing            | All components verified in dark and light modes         |

### Week 21

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Accessibility: axe-core scan, identify violations           |
| Tue   | Accessibility: fix keyboard navigation issues               |
| Wed   | Accessibility: screen reader testing, ARIA labels           |
| Thu   | Accessibility: focus management, skip links                 |
| Fri   | Performance: Lighthouse audit, identify bottlenecks         |

### Week 22

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Performance: bundle optimization, code splitting            |
| Tue   | Performance: runtime optimization, memoization              |
| Wed   | Security: OWASP ZAP scan                                    |
| Thu   | Security: fix vulnerabilities, CSP hardening                |
| Fri   | Visual regression: screenshot baseline for all components   |

### Week 23

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | E2E tests: critical user journeys (login, upload, share)    |
| Tue   | E2E tests: collaboration features (notes, tasks, chat)      |
| Wed   | i18n: translation files for 5 languages                     |
| Thu   | i18n: RTL support, locale formatting                        |
| Fri   | Onboarding: first-run experience, tooltips                  |

### Week 24

| Day   | Task                                                        |
|-------|-------------------------------------------------------------|
| Mon   | Keyboard shortcuts: reference panel, shortcut binding       |
| Tue   | Theme testing: all components verified dark + light         |
| Wed   | Documentation: architecture guide, component API docs       |
| Thu   | Final integration testing, bug fixes                        |
| Fri   | Release candidate, stakeholder review                       |

### Milestone: Production Ready
- [ ] Zero critical accessibility violations
- [ ] Lighthouse performance >90
- [ ] Zero high/medium security findings
- [ ] All components pass visual regression
- [ ] E2E tests cover all critical journeys
- [ ] All 5 languages translated
- [ ] Onboarding flow complete
- [ ] Keyboard shortcuts documented
- [ ] All themes tested

---

## Success Metrics

| Metric                                        | Target                    | Measurement                           |
|----------------------------------------------|---------------------------|---------------------------------------|
| Backend endpoint coverage                     | 100% (150+ endpoints)     | API client generation report          |
| Interaction latency (p99)                     | < 100ms                   | Performance monitoring                |
| Critical accessibility violations             | 0                         | axe-core + manual audit               |
| Code coverage (critical paths)                | > 90%                     | wasm-bindgen-test coverage            |
| Lighthouse performance score                  | > 90                      | Lighthouse WASM audit                 |
| Cross-browser compatibility                   | Chrome, Firefox, Safari, Edge | Playwright multi-browser tests    |
| Desktop app (Linux, macOS, Windows)           | All functional            | Platform-specific testing             |
| Offline mode (core file operations)           | Fully functional          | Offline integration tests             |
| Visual regression failures                    | 0                         | Screenshot comparison                 |
| Security findings (high/medium)               | 0                         | OWASP ZAP scan                        |
| Bundle size (WASM + JS + CSS)                 | < 400KB compressed        | Trunk build output                    |
| Languages supported                           | 6 (EN, ES, FR, DE, JA, ZH) | i18n coverage report                |

---

## Risk Register

| Risk                                          | Phase | Likelihood | Impact | Mitigation                              |
|----------------------------------------------|-------|------------|--------|-----------------------------------------|
| Leptos 0.8 breaking changes                   | 0-7   | Low        | Medium | Pin version, vendor critical deps       |
| Scope creep beyond 24 weeks                   | 0-7   | High       | High   | Phase gates, MVP-first, strict prioritization |
| API schema changes during development         | 1-5   | Medium     | Low    | Backend stable, version API endpoints   |
| WASM bundle size exceeds budget               | 0-7   | Medium     | Medium | Code splitting from Phase 0, continuous monitoring |
| Accessibility requirements expand             | 7     | Low        | Medium | WCAG 2.1 AA baseline, audit every phase |
| Desktop platform issues (Linux/macOS/Windows) | 6     | Medium     | Medium | Test early, platform-specific workarounds |
| Translation quality for non-English           | 7     | High       | Low    | Native speaker review, community contribution |
| Performance regression under load             | 1-7   | Medium     | High   | Load testing, performance budgets       |

---

## Feature Flags

Feature flags enable gradual rollout and instant kill switches:

```toml
# feature-flags.toml
[flags.file_browser_v2]
phase = 1
default = false
description = "New file browser with list/grid views"

[flags.chat_websocket]
phase = 2
default = false
description = "WebSocket-based real-time chat"

[flags.offline_mode]
phase = 5
default = false
description = "IndexedDB offline cache"

[flags.admin_plugins]
phase = 4
default = false
description = "Plugin marketplace"
```

Flag evaluation happens client-side via `FeatureFlagProvider`. Flags are fetched from `/api/v1/feature-flags` on app load and cached locally.

---

## Rollback Strategy

Each phase has a defined rollback procedure:

| Phase | Rollback Trigger | Rollback Action |
|-------|-----------------|-----------------|
| 0 | Build fails, primitives broken | Revert to old frontend entirely |
| 1 | File operations broken | Disable v2 routes, serve old file browser |
| 2 | Collaboration features broken | Disable collaboration routes, keep file browser |
| 3 | Media features broken | Disable media routes, keep core + collab |
| 4 | Admin features broken | Disable admin routes, keep core + collab + media |
| 5 | Offline mode causes data loss | Disable offline flag, revert to online-only |
| 6 | Desktop integration broken | Ship web-only, defer desktop |
| 7 | Any critical issue found | Disable affected feature flag, ship without |

Rollback is implemented via feature flags + route-level code splitting. Old frontend remains deployable until Phase 7 completion.

---

## Dependencies

| Dependency                          | Phase | Required For                        |
|------------------------------------|-------|-------------------------------------|
| Leptos 0.8 stable                  | 0     | Everything                          |
| Trunk (Rust WASM bundler)          | 0     | Build pipeline                      |
| Tauri v2 stable                    | 6     | Desktop integration                 |
| Playwright                          | 0     | E2E testing                         |
| axe-core                            | 7     | Accessibility testing               |
| OWASP ZAP                           | 7     | Security testing                    |
| Translation contributors           | 7     | i18n                                |
| Backend API stability              | 1-5   | API client generation               |

---

## Out-of-Scope Features (Migration Required)

The following features exist in the current frontend (per GUI_AUDIT.md) but are NOT included in this rewrite. These require a separate migration plan:

| Feature | Current State | Migration Plan |
|---------|--------------|----------------|
| **Mail (IMAP)** | AUDIT Section 2.12 — 6 endpoints, account management, folder nav, compose | Defer to Phase 8 or separate project. Requires IMAP client architecture decision. |
| **Analytics** | AUDIT Section 2.14 — Share link analytics, storage charts, activity timeline | Defer to Phase 8. Low priority compared to core features. |
| **Graph view** | AUDIT Section 3 — Dependency graph visualization | Include in Phase 3 if time permits. Low priority. |
| **Custom views** | AUDIT Section 3 — Configurable data table views | Include in Phase 4 if time permits. Low priority. |
| **Photo editor** | AUDIT Appendix A — Crop, rotate, filters | Include in Phase 3 if time permits. Medium priority. |
| **Slideshow** | AUDIT Appendix A — Auto-advance, transitions | Include in Phase 3 if time permits. Low priority. |

**Decision required:** Product owner must explicitly approve deferral or assign these to a subsequent project.

---

## References

- [ADR-001: Complete GUI Rewrite](../02_architecture/ADR-001-GUI-REWRITE.md)
- [GUI Architecture Specification](../02_architecture/GUI_ARCHITECTURE.md)
- [Current Backend API Spec](../00_requirements/API_SPECIFICATION.md)
- [Security Specification](../03_security/SECURITY_SPEC.md)
- [Performance Specification](../04_performance/PERFORMANCE_SPEC.md)

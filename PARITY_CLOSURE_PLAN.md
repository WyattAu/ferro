# Ferro Parity Closure & Improvement Plan

**Date:** 2026-06-15
**Author:** Nexus (Principal Systems Architect)
**Baseline:** Competitive Parity Matrix (COMPETITIVE_PARITY_MATRIX.md)
**Target:** Close critical gaps while maintaining performance/security advantages

---

## Strategic Principles

1. **Performance first:** Never sacrifice <10ms P99 for features
2. **Security depth:** Maintain Cedar + SHA-256 audit chain advantages
3. **Self-hosted native:** Don't try to beat Google/Dropbox at cloud -- win at self-hosted
4. **Rust advantage:** Use Rust strengths (WASM, performance, safety) for unique features
5. **Incremental delivery:** Ship working features, iterate

---

## Priority Tiers

### TIER 1: CRITICAL (Must ship to be competitive)

#### P1-01: Native Desktop Client (Windows + macOS)
**Gap:** Tauri buildable but no installer, no Finder/Explorer integration
**Effort:** 4-6 weeks
**Approach:**
- Ship Tauri desktop as signed `.msi` (Windows) and `.dmg` (macOS)
- Implement Finder extension (macOS) for overlay icons and context menu
- Implement Explorer shell extension (Windows) for overlay icons and context menu
- Add auto-update via Tauri updater plugin
- System tray with sync status, pause/resume

**Milestones:**
1. Week 1-2: Tauri build pipeline + code signing (Windows + macOS)
2. Week 3-4: Finder/Explorer shell extensions
3. Week 5-6: Auto-update + system tray polish

**Acceptance criteria:**
- Windows: MSI installer, Explorer overlay icons, right-click context menu
- macOS: DMG installer, Finder integration, menu bar icon
- Both: Auto-update, system tray, selective sync UI

#### P1-02: Native Mobile Apps (iOS + Android)
**Gap:** Contract only, no real apps
**Effort:** 8-12 weeks (Tauri v2 mobile)
**Approach:**
- Use Tauri v2 iOS/Android targets (same Rust codebase)
- Camera auto-upload (background task)
- Offline file caching with selective sync
- Share sheet integration
- Push notifications (FCM/APNS)
- File provider (iOS) / SAF (Android)

**Milestones:**
1. Week 1-3: Tauri iOS project setup + basic file browsing
2. Week 4-6: Android setup + file operations
3. Week 7-9: Camera auto-upload + offline mode
4. Week 10-12: Push notifications + polish

**Acceptance criteria:**
- iOS: File browsing, upload/download, camera auto-upload, share sheet, offline files
- Android: File browsing, upload/download, camera auto-upload, SAF integration, offline files
- Both: Push notifications, background sync, biometric auth

#### P1-03: Offline Mode (Full)
**Gap:** `ferro-offline` crate exists but not wired to server or desktop
**Effort:** 3-4 weeks
**Approach:**
- Wire `ferro-offline` into desktop client (sync on reconnect)
- Add offline file cache with configurable size limit
- Conflict resolution on reconnect (vector clocks + CRDT)
- Visual indicator for offline/partially synced files
- Background sync when connection restored

**Milestones:**
1. Week 1: Wire offline crate into desktop client
2. Week 2: Add offline cache management UI
3. Week 3: Conflict resolution on reconnect
4. Week 4: Background sync + visual indicators

**Acceptance criteria:**
- Desktop: Files accessible offline, sync on reconnect, conflict resolution
- Visual: Sync status indicator per file/folder
- Config: Cache size limit, selective offline folders

#### P1-04: Version History & Trash UI
**Gap:** Backend exists but no dedicated UI
**Effort:** 2-3 weeks
**Approach:**
- Version history panel (per file): list versions, restore, compare
- Trash panel: browse deleted items, restore, empty trash
- Visual diff for text file versions (optional)
- Admin: configurable retention policies via UI

**Milestones:**
1. Week 1: Version history panel (web + desktop)
2. Week 2: Trash panel with restore/empty
3. Week 3: Admin retention policy UI

**Acceptance criteria:**
- Version history: List all versions, restore any version, show metadata
- Trash: Browse deleted, restore, empty trash, auto-purge indicator
- Admin: Set retention policies per user/space

#### P1-05: Share Dialog & Management
**Gap:** Backend supports sharing but no dedicated UI
**Effort:** 2-3 weeks
**Approach:**
- Share dialog: invite users/groups, set permissions, create share links
- Share management: view all shares, revoke, edit permissions
- Share link settings: expiry, password, download limit
- Federated sharing UI (ActivityPub)

**Milestones:**
1. Week 1: Share dialog (invite + permissions)
2. Week 2: Share link management + settings
3. Week 3: Federated sharing UI

**Acceptance criteria:**
- Share dialog: Invite by email/username, set view/edit/admin permissions
- Share links: Create, configure expiry/password, copy/revoke
- Management: View all shares per file/folder, bulk operations

---

### TIER 2: HIGH (Significant competitive advantage)

#### P2-01: Dashboard & Activity Timeline
**Gap:** No dashboard, no activity feed
**Effort:** 2-3 weeks
**Approach:**
- Dashboard: storage usage, recent files, shared with me, quick actions
- Activity timeline: file events (create/edit/delete/share) per user
- Notification center: share invites, mentions, system alerts
- Widget system for dashboard customization

**Milestones:**
1. Week 1: Dashboard with storage/recent/shared widgets
2. Week 2: Activity timeline + notification center
3. Week 3: Widget system + admin dashboard

#### P2-02: Multi-Language Support
**Gap:** EN only, i18n framework exists
**Effort:** 1-2 weeks per language (after framework wired)
**Approach:**
- Wire existing `t!()` macro into all UI components
- Ship with top 5 languages (EN, ES, FR, DE, ZH)
- Community translation platform (Weblate/Crowdin)
- RTL support for Arabic/Hebrew

**Milestones:**
1. Week 1: Wire i18n into all desktop components
2. Week 2-3: Ship with 5 languages
3. Week 4: Community translation platform

#### P2-03: Calendar/Contacts UI (CalDAV/CardDAV)
**Gap:** Backend works but no dedicated UI
**Effort:** 3-4 weeks
**Approach:**
- Calendar view: month/week/day, create/edit/delete events
- Contacts view: list, search, create/edit/delete
- Shared calendars with permission management
- Import/export (ICS, vCard)

**Milestones:**
1. Week 1: Calendar view (month/week/day)
2. Week 2: Contacts view (list + detail)
3. Week 3: Shared calendars + permissions
4. Week 4: Import/export + polish

#### P2-04: Photo Management
**Gap:** No photo browsing, no camera upload
**Effort:** 3-4 weeks
**Approach:**
- Photo gallery view: timeline, grid, lightbox
- EXIF metadata display (camera, date, location)
- Camera auto-upload (mobile)
- Photo search by date/location
- Album creation (virtual folders)

**Milestones:**
1. Week 1: Photo gallery view (grid + lightbox)
2. Week 2: EXIF metadata + timeline view
3. Week 3: Camera auto-upload (mobile)
4. Week 4: Albums + search

#### P2-05: Video Streaming
**Gap:** No video playback
**Effort:** 2-3 weeks
**Approach:**
- HTML5 video player with HLS transcoding
- Support: MP4, WebM, MKV (via transcoding)
- Seek/scrub, quality selection, subtitle support
- Mobile: native player integration

**Milestones:**
1. Week 1: HTML5 player with format detection
2. Week 2: HLS transcoding for non-native formats
3. Week 3: Mobile integration + subtitle support

#### P2-06: Virus Scanning (ClamAV)
**Gap:** Skeleton only, no real integration
**Effort:** 1-2 weeks
**Approach:**
- Wire ClamAV socket connection (TCP/Unix)
- Scan on upload, scan on demand
- Quarantine infected files
- Admin dashboard: scan status, infection history

**Milestones:**
1. Week 1: ClamAV socket connection + scan on upload
2. Week 2: Quarantine + admin dashboard

#### P2-07: DLP (Data Loss Prevention)
**Gap:** Not implemented
**Effort:** 3-4 weeks
**Approach:**
- File access policies: restrict by file type, size, user, group
- Content inspection: detect sensitive data patterns (credit cards, SSN, etc.)
- Block/allow list for external sharing
- Admin policy management UI

**Milestones:**
1. Week 1: File access policies (type/size/user restrictions)
2. Week 2: Content inspection (regex patterns)
3. Week 3: External sharing controls
4. Week 4: Admin policy UI

---

### TIER 3: MEDIUM (Ecosystem breadth)

#### P3-01: Chat/Messaging (Real-time)
**Gap:** No chat/video calls
**Effort:** 6-8 weeks
**Approach:**
- WebSocket-based messaging per file/folder
- Threaded conversations
- File sharing in chat
- @mentions with notifications
- Video calls (WebRTC, optional)

**Milestones:**
1. Week 1-2: WebSocket messaging infrastructure
2. Week 3-4: Chat UI (per-file threads, global chat)
3. Week 5-6: @mentions + notifications
4. Week 7-8: Video calls (WebRTC, optional)

#### P3-02: Tasks/Kanban
**Gap:** No task management
**Effort:** 3-4 weeks
**Approach:**
- Task lists per file/folder/project
- Kanban board view (todo/in-progress/done)
- Assignees, due dates, priority
- Calendar integration (CalDAV)
- Notifications on assignment/completion

**Milestones:**
1. Week 1: Task model + API
2. Week 2: Task list UI + Kanban board
3. Week 3: Calendar integration
4. Week 4: Notifications + polish

#### P3-03: Mail Integration
**Gap:** No email
**Effort:** 4-6 weeks
**Approach:**
- IMAP client for receiving
- SMTP client for sending
- Mail UI in web/desktop
- File attachments from Ferro
- Share link via email

**Milestones:**
1. Week 1-2: IMAP/SMTP infrastructure
2. Week 3-4: Mail UI
3. Week 5-6: File attachment + share via email

#### P3-04: Notes/Wiki
**Gap:** No notes or wiki
**Effort:** 3-4 weeks
**Approach:**
- Markdown editor with live preview
- Per-file/folder notes
- Wiki pages (hierarchical)
- Cross-linking between notes
- Search across all notes

**Milestones:**
1. Week 1: Markdown editor component
2. Week 2: Per-file notes + API
3. Week 3: Wiki pages (hierarchical)
4. Week 4: Cross-linking + search

#### P3-05: Whiteboard
**Gap:** No whiteboard
**Effort:** 2-3 weeks
**Approach:**
- Canvas-based drawing (shapes, text, lines)
- Collaborative editing (CRDT)
- Export to PNG/PDF
- Templates (flowchart, mindmap, wireframe)

**Milestones:**
1. Week 1: Canvas drawing engine
2. Week 2: Collaborative editing (CRDT sync)
3. Week 3: Export + templates

#### P3-06: Link Analytics
**Gap:** No share link analytics
**Effort:** 1-2 weeks
**Approach:**
- Track: views, downloads, unique visitors, referrers
- Dashboard per share link
- Export analytics data
- Admin: global link usage stats

**Milestones:**
1. Week 1: Analytics collection + storage
2. Week 2: Dashboard + export

#### P3-07: Watermarking
**Gap:** No watermarking
**Effort:** 1-2 weeks
**Approach:**
- Image watermarking (configurable text/logo)
- PDF watermarking
- Video watermarking (overlay)
- Admin: watermark policy per space/user

**Milestones:**
1. Week 1: Image + PDF watermarking
2. Week 2: Video overlay + admin policy

#### P3-08: Account Transfer & Remote Wipe
**Gap:** No account transfer, no remote wipe
**Effort:** 1-2 weeks
**Approach:**
- Admin: transfer user data to another user
- Admin: remotely wipe synced data on lost device
- User: self-service device management
- Audit log for transfer/wipe events

**Milestones:**
1. Week 1: Account transfer API + UI
2. Week 2: Remote wipe + device management

---

### TIER 4: LOW (Polish and ecosystem)

#### P4-01: Dark Mode Polish
- Consistent dark mode across all UI components
- System theme detection
- Manual toggle

#### P4-02: Keyboard Shortcuts Expansion
- Standard shortcuts (Ctrl+C, Ctrl+V, Ctrl+X, Del, F2 rename)
- Customizable shortcut map
- Shortcut help overlay

#### P4-03: Drag-and-Drop Enhancement
- Drag from desktop to browser (upload)
- Drag between folders (move)
- Drag to share dialog (share)

#### P4-04: Grid/List View Toggle
- Persistent preference per user
- Thumbnail size slider
- Sort options (name, date, size, type)

#### P4-05: Batch Operations
- Multi-select with Shift/Ctrl
- Batch move/copy/delete
- Batch share

#### P4-06: Search Improvements
- Filters (type, date, size, owner, shared)
- Search suggestions
- Recent searches
- Search within file contents (Tantivy)

#### P4-07: Notification Preferences
- Per-event notification settings
- Email notification digests
- Push notification customization

#### P4-08: Accessibility Audit
- Full WCAG 2.1 AA audit
- Screen reader testing
- High contrast mode
- Reduced motion

---

## Implementation Timeline

### Phase A: Critical Parity (Weeks 1-12)

| Week | P1-01 Desktop | P1-02 Mobile | P1-03 Offline | P1-04 UIs | P1-05 Shares |
|------|---------------|--------------|---------------|-----------|--------------|
| 1 | Build pipeline | iOS setup | Wire offline | Version UI | Share dialog |
| 2 | Code signing | iOS browsing | Cache mgmt | Trash UI | Share links |
| 3 | Finder ext | iOS ops | Conflict res | Retention UI | Federated |
| 4 | Explorer ext | Android setup | Background sync | | |
| 5 | Auto-update | Android ops | | | |
| 6 | System tray | Camera upload | | | |

### Phase B: Competitive Features (Weeks 13-24)

| Week | P2-01 Dashboard | P2-02 i18n | P2-03 CalDAV UI | P2-04 Photos | P2-05 Video | P2-06 ClamAV | P2-07 DLP |
|------|-----------------|------------|-----------------|--------------|-------------|--------------|-----------|
| 13 | Dashboard | Wire i18n | Calendar view | Photo grid | HTML5 player | ClamAV conn | Policies |
| 14 | Activity timeline | Ship 5 langs | Contacts view | EXIF + timeline | HLS transcode | Quarantine | Content insp |
| 15 | Notifications | Translation platform | Shared cals | Camera upload | Mobile player | Admin UI | Sharing ctrl |
| 16 | Widgets | | Import/export | Albums + search | Subtitles | | Admin UI |

### Phase C: Ecosystem (Weeks 25-40)

| Week | P3-01 Chat | P3-02 Tasks | P3-03 Mail | P3-04 Notes | P3-05 Whiteboard |
|------|-----------|-------------|-----------|-------------|------------------|
| 25-28 | Chat infra + UI | Task model + UI | IMAP/SMTP | Markdown editor | Canvas engine |
| 29-32 | Mentions + notif | Kanban + calendar | Mail UI | Wiki pages | CRDT collab |
| 33-36 | Video calls | Notifications | Attachments | Cross-linking | Export |
| 37-40 | | | | Search | Templates |

---

## Resource Estimates

| Phase | Duration | Focus | Risk |
|-------|----------|-------|------|
| Phase A | 12 weeks | Desktop + Mobile + Offline + UIs | Medium (Tauri mobile maturity) |
| Phase B | 12 weeks | Dashboard + Photos + Video + Security | Low (incremental) |
| Phase C | 16 weeks | Groupware (Chat/Tasks/Mail/Notes) | High (complex real-time features) |
| **Total** | **40 weeks** | **Full competitive parity** | |

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tauri v2 mobile unstable | HIGH | Fallback to React Native for mobile |
| CRDT chat complexity | HIGH | Start with WebSocket simple messages, add CRDT later |
| WASM plugin ecosystem too small | MEDIUM | Ship 5 killer plugins, community follows |
| Performance regression with new features | MEDIUM | Benchmark every PR, maintain P99 <10ms |
| Scope creep | HIGH | Strict priority tiers, ship incrementally |

---

## Success Metrics

| Metric | Current | Phase A | Phase B | Phase C |
|--------|---------|---------|---------|---------|
| Desktop clients | 1 (Linux CLI) | 3 (Win/Mac/Linux) | 3 | 3 |
| Mobile apps | 0 | 2 (iOS/Android) | 2 | 2 |
| Feature parity score | 45/100 | 65/100 | 80/100 | 95/100 |
| Plugin ecosystem | 43 crates | 43 | 50+ | 60+ |
| Languages | 1 | 1 | 6 | 10+ |
| P99 latency | <10ms | <10ms | <15ms | <20ms |
| Test count | 2500+ | 3000+ | 3500+ | 4000+ |

---

## Competitive Positioning After Closure

After completing all phases, Ferro will be:

1. **Fastest** self-hosted file sync server (<10ms P99)
2. **Most secure** (Cedar + SHA-256 audit chain + X25519 E2EE)
3. **Most extensible** (WASM plugins + ActivityPub federation)
4. **Most complete** (file sync + groupware + office + chat + tasks)
5. **Most deployable** (single binary, Docker, Kubernetes)

**Target:** Match Nextcloud's feature breadth while maintaining Ferro's performance and security advantages.

---

*This plan is a living document. Update as features ship and priorities shift.*

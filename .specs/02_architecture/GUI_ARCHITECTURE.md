# GUI Architecture Specification

> Complete architecture for the Ferro frontend rewrite. All design decisions are informed by high-frequency trading (HFT), FAANG-scale, defense-grade, and ECN-grade engineering principles.

---

## 1. Design Principles

| #  | Principle                      | Origin      | Requirement                                           |
|----|--------------------------------|-------------|-------------------------------------------------------|
| 1  | **Deterministic Rendering**    | HFT         | Zero layout thrash, virtual scrolling, memoized computations. Every render produces identical DOM output for identical state. |
| 2  | **Sub-100ms Interaction Latency** | HFT/FAANG | Optimistic updates, local-first state, background sync. User interactions must feel instantaneous. |
| 3  | **Defense-Grade Reliability**  | Defense     | Error boundaries at every level, graceful degradation, offline-first. No single failure brings down the app. |
| 4  | **Zero-Trust Security**        | Defense     | CSP strict, no inline scripts, CSRF tokens, input sanitization. Every input is untrusted until validated. |
| 5  | **Observability**              | FAANG       | Structured logging, performance metrics, error tracking. You cannot improve what you cannot measure. |
| 6  | **Accessibility**              | ECN/ADA     | WCAG 2.1 AA minimum, keyboard-first navigation, screen reader support. Accessible by default, not as an afterthought. |
| 7  | **Type Safety**                | FAANG       | End-to-end type safety from API schema to UI components. Compile-time guarantees over runtime checks. |
| 8  | **SOLID Compliance**           | Design Patterns | Single Responsibility, Open/Closed, Liskov Substitution, Interface Segregation, Dependency Inversion. |
| 9  | **Composition over Inheritance** | Design Patterns | Build complex components from simple primitives via composition, not class hierarchies. |
| 10 | **Strategy Pattern**           | Design Patterns | Algorithms (sort, search, conflict resolution) are interchangeable via trait objects. |

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Ferro Frontend                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   Routes     │  │   State      │  │   Styles             │  │
│  │  (LazyRoute) │  │  (Signals)   │  │  (CSS Custom Props)  │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘  │
│         │                 │                      │              │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌──────────▼───────────┐  │
│  │  Components  │  │  API Client  │  │  Utility System      │  │
│  │  (Leptos)    │  │  (Generated) │  │  (Complete Coverage) │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────────────┘  │
│         │                 │                                     │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌──────────────────────┐  │
│  │  WebSocket   │  │  Offline     │  │  Service Worker      │  │
│  │  Manager     │  │  Cache (IDB) │  │  (Asset Caching)     │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────────────┘  │
│         │                 │                                     │
└─────────┼─────────────────┼─────────────────────────────────────┘
          │                 │
    ┌─────▼─────┐     ┌────▼────┐
    │  Backend  │     │  CDN    │
    │  (Axum)   │     │ (Static)│
    └───────────┘     └─────────┘
```

### Technology Choices

| Layer               | Technology                              | Rationale                                        |
|---------------------|-----------------------------------------|--------------------------------------------------|
| Rendering Engine    | Leptos 0.8 CSR                          | Best Rust WASM framework, fine-grained reactivity, no virtual DOM |
| State Management    | Leptos signals + context                | Built-in, no extra dependencies, composable      |
| Styling             | CSS custom properties + hand-written utilities | Full control, no framework overhead, design token native |
| API Client          | Code-generated from TOML schema         | Type-safe, compile-time verified, zero manual parsing |
| Real-time           | WebSocket with auto-reconnect           | Native browser API, no polling overhead           |
| Offline             | IndexedDB via `idb` crate               | Structured storage, async, transactional          |
| Bundling            | Trunk                                    | Rust-native WASM bundler, handles CSS/JS/assets  |
| Testing             | `wasm-bindgen-test` + Playwright        | Unit tests in WASM, E2E in real browser           |

---

## 3. Component Architecture

### 3.1 Primitive Components

The foundation layer. Every other component is built from these.

| Component    | Props                                          | Accessibility                      |
|-------------|------------------------------------------------|------------------------------------|
| `Button`     | `variant`, `size`, `disabled`, `loading`, `icon` | `role="button"`, keyboard enter/space, aria-disabled |
| `Input`      | `type`, `placeholder`, `value`, `error`, `label` | `aria-label`, `aria-invalid`, `aria-describedby` |
| `Select`     | `options`, `value`, `placeholder`, `multiple`  | `role="listbox"`, arrow key navigation, `aria-activedescendant` |
| `Dialog`     | `open`, `title`, `on_close`, `size`           | `role="dialog"`, `aria-modal`, focus trap, Escape closes |
| `Toast`      | `variant`, `message`, `duration`, `action`     | `role="alert"`, `aria-live="polite"` |
| `Tooltip`    | `content`, `placement`, `delay`                | `aria-describedby`, hover/focus trigger |
| `Badge`      | `variant`, `count`                             | `aria-label` for screen readers    |
| `Avatar`     | `src`, `name`, `size`                          | `alt` text derived from `name`     |
| `Spinner`    | `size`, `label`                                | `role="status"`, `aria-label`      |
| `Divider`    | `orientation`, `label`                         | `role="separator"`                 |

### 3.2 Layout Components

The structural layer that defines page composition.

```
┌─ Shell ──────────────────────────────────────────────┐
│ ┌─ Header ─────────────────────────────────────────┐ │
│ │ [Logo] [Search] [Notifications] [User Menu]      │ │
│ └──────────────────────────────────────────────────┘ │
│ ┌─ Sidebar ─┐ ┌─ ContentArea ─────────────────────┐ │
│ │ [Nav]     │ │ [Breadcrumbs]                     │ │
│ │ [Nav]     │ │ [Page Content]                    │ │
│ │ [Nav]     │ │                                   │ │
│ │ [Nav]     │ │                                   │ │
│ └───────────┘ └───────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

| Component       | Responsibility                                         |
|-----------------|--------------------------------------------------------|
| `Shell`         | Top-level layout, orchestrates Sidebar + Header + Content |
| `Sidebar`       | Collapsible navigation, stores collapsed state         |
| `Header`        | Global search, notifications, user menu                |
| `ContentArea`   | Main content wrapper, handles breadcrumbs              |
| `SplitPane`     | Resizable two-pane layout for file preview             |
| `Stack`         | Vertical/horizontal stacking with consistent spacing   |
| `Grid`          | CSS Grid wrapper with responsive breakpoints           |

### 3.3 Domain Components

Feature-specific components built from primitives.

| Component            | Features                                          | Phase |
|----------------------|---------------------------------------------------|-------|
| `FileBrowser`        | List/grid view, breadcrumb nav, drag-drop, context menu | 1     |
| `UploadZone`         | Drag-drop area, progress bar, file validation      | 1     |
| `CommandPalette`     | Ctrl+K global command palette, fuzzy search        | 0     |
| `GraphView`          | Dependency graph visualization for file relationships | 3   |
| `CustomView`         | Configurable data table views                      | 4     |
| `NotesEditor`        | Markdown editing, live preview, folder tree        | 2     |
| `TaskBoard`          | Kanban columns, drag-drop cards, filters           | 2     |
| `CalendarGrid`       | Month/week/day views, event CRUD, drag-resize     | 2     |
| `ContactList`        | vCard rendering, search, import/export             | 2     |
| `ChatPanel`          | WebSocket real-time, rooms, @mentions, reactions   | 2     |
| `PhotoGrid`          | Masonry layout, lightbox, album management         | 3     |
| `PhotoEditor`        | Crop, rotate, filters, EXIF editing                | 3     |
| `Slideshow`          | Auto-advance, transitions, keyboard nav            | 3     |
| `VideoPlayer`        | Range request streaming, custom controls           | 3     |
| `AudioPlayer`        | Playlist, waveform visualization                   | 3     |
| `EpubPreview`        | EPUB reader with page navigation                   | 3     |
| `MarkdownPreview`    | Rendered markdown with syntax highlighting         | 3     |
| `WhiteboardCanvas`   | Drawing tools, real-time cursors, export            | 3     |
| `AdminDashboard`     | User management, DLP, audit logs                   | 4     |
| `AuditLogViewer`     | Filterable, exportable, real-time streaming         | 4     |
| `SettingsPanel`      | Tabbed settings, validation, save states            | 4     |
| `OnboardingOverlay`  | First-run experience, feature highlights           | 0     |
| `SetupWizard`        | Initial configuration wizard                       | 0     |

### 3.4 Infrastructure Components

Cross-cutting concerns applied at the app level.

| Component             | Responsibility                                     |
|-----------------------|----------------------------------------------------|
| `ErrorBoundary`       | Catches render errors, shows fallback UI, logs error |
| `Suspense`            | Shows loading skeleton while async data resolves    |
| `LazyRoute`           | Code-splits route chunks, shows loading indicator   |
| `WebSocketProvider`   | Manages WS connection, reconnect, message routing   |
| `OfflineIndicator`    | Shows online/offline status, queues mutations       |
| `ThemeProvider`        | Manages dark/light theme, applies CSS custom props  |
| `I18nProvider`        | Loads translations, provides t() function           |
| `AnalyticsProvider`   | Structured logging, performance metrics             |
| `FeatureFlagProvider` | Feature flag evaluation, gradual rollout            |
| `AuditLogger`         | Client-side action logging for audit trail          |
| `CircuitBreaker`      | API call failure detection, fallback triggering     |

---

## 4. State Architecture

### 4.1 Layer Model

```
┌─────────────────────────────────────────┐
│           Global State (1 instance)      │
│  Auth, Theme, WebSocket, Offline Queue  │
│  Scope: Entire application lifetime     │
├─────────────────────────────────────────┤
│           Feature State (per feature)    │
│  FileBrowser, Notes, Tasks, Calendar     │
│  Scope: Feature active lifetime         │
├─────────────────────────────────────────┤
│           Component State (per component)│
│  is_open, is_hovered, focused_index     │
│  Scope: Component mount lifetime        │
├─────────────────────────────────────────┤
│           Server State (cached)          │
│  TanStack Query-style cached/invalidated│
│  Scope: Stale-while-revalidate          │
└─────────────────────────────────────────┘
```

### 4.2 Global State

```rust
// state/global.rs
#[derive(Clone)]
pub struct GlobalState {
    pub auth: AuthState,
    pub theme: ThemeState,
    pub websocket: WebSocketState,
    pub offline: OfflineQueueState,
    pub notification: NotificationState,
}

#[derive(Clone)]
pub struct AuthState {
    pub user: Signal<Option<User>>,
    pub token: Signal<Option<String>>,
    pub is_authenticated: Signal<bool>,
}

#[derive(Clone)]
pub struct ThemeState {
    pub mode: Signal<ThemeMode>,           // Dark | Light | System
    pub accent_color: Signal<String>,
    pub font_scale: Signal<f32>,
}

#[derive(Clone)]
pub struct WebSocketState {
    pub status: Signal<ConnectionStatus>,  // Connected | Reconnecting | Disconnected
    pub last_message: Signal<Option<WsMessage>>,
    pub send: Callback<WsMessage>,
}

#[derive(Clone)]
pub struct OfflineQueueState {
    pub pending_mutations: Signal<Vec<PendingMutation>>,
    pub is_online: Signal<bool>,
    pub queue_mutation: Callback<PendingMutation>,
}
```

### 4.3 Feature State

Each feature owns its slice of state with clear boundaries:

```rust
// state/features/file_browser.rs
#[derive(Clone)]
pub struct FileBrowserState {
    pub current_path: Signal<PathBuf>,
    pub view_mode: Signal<ViewMode>,           // List | Grid
    pub sort_by: Signal<SortField>,
    pub sort_order: Signal<SortOrder>,
    pub selected: Signal<HashSet<FileId>>,
    pub search_query: Signal<String>,
    pub breadcrumbs: Signal<Vec<BreadcrumbItem>>,
    pub items: Resource<(), Vec<FileItem>>,
}
```

### 4.4 Server State (Cache Layer)

Inspired by TanStack Query — every server resource is cached with stale-while-revalidate semantics:

```rust
pub struct ServerCache<T> {
    pub data: Signal<Option<T>>,
    pub error: Signal<Option<ApiError>>,
    pub is_loading: Signal<bool>,
    pub is_stale: Signal<bool>,
    pub last_fetched: Signal<Option<Instant>>,
    pub refetch: Callback<()>,
    pub invalidate: Callback<()>,
}
```

Cache rules:
- **Fresh** (< 30s): Serve from cache, no refetch
- **Stale** (30s - 5min): Serve from cache, refetch in background
- **Expired** (> 5min): Block on refetch, show stale data with indicator
- **Mutations**: Optimistic update → refetch → rollback on error

---

## 5. API Architecture

### 5.1 Code Generation Pipeline

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  TOML Schema │ ──▶ │  Code Gen    │ ──▶ │  Rust Client │ ──▶ │  WASM Binary │
│  (150+ APIs) │     │  (build.rs)  │     │  (src/api/)  │     │  (dist/)     │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
```

### 5.2 Schema Definition

All API paths use the `/api/v1/` prefix for versioning. This is the canonical convention — all documents must reference these paths consistently.

```toml
# api/schema/files.toml
[endpoint.get_files]
method = "GET"
path = "/api/v1/files"
description = "List files in directory"

[endpoint.get_files.query]
path = "string"               # Required
recursive = "bool"            # Optional, default false
page = "u32"                  # Optional, default 1
per_page = "u32"              # Optional, default 50

[endpoint.get_files.response]
status = 200
type = "PaginatedList<FileItem>"

[endpoint.get_files.response.error]
status = 401
type = "UnauthorizedError"
```

### 5.3 Generated Client Interface

```rust
// Generated from TOML schema — DO NOT EDIT
pub struct FileApi {
    client: HttpClient,
}

impl FileApi {
    pub async fn get_files(&self, params: GetFilesParams) -> Result<PaginatedList<FileItem>, ApiError> {
        self.client.get("/api/v1/files")
            .query(&params)
            .send()
            .await
    }

    pub async fn upload_file(&self, path: &str, data: Vec<u8>) -> Result<FileItem, ApiError> {
        self.client.post("/api/v1/files/upload")
            .multipart("file", path, data)
            .send()
            .await
    }
}
```

### 5.4 Request/Response Types

Every endpoint has strongly typed request and response types:

```rust
pub struct GetFilesParams {
    pub path: String,
    pub recursive: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub struct FileItem {
    pub id: FileId,
    pub name: String,
    pub path: String,
    pub mime_type: String,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub checksum: String,
    pub is_dir: bool,
}

pub enum ApiError {
    Unauthorized,
    NotFound { resource: String },
    Validation { field: String, message: String },
    Server(String),
    Network(String),
}
```

### 5.5 Error Handling

```
API Error → ApiError enum → CircuitBreaker check → ErrorBoundary catch → User-friendly toast + retry action
```

| Error Type     | UI Response                                    | Retry? |
|---------------|------------------------------------------------|--------|
| Unauthorized   | Redirect to login                              | No     |
| NotFound       | "Resource not found" toast, navigate home      | No     |
| Validation     | Inline field error                             | No     |
| Server (5xx)   | "Something went wrong" toast with retry button | Yes    |
| Network        | "Connection lost" banner, queue mutation       | Auto   |

### 5.6 Circuit Breaker

Protect against cascading failures when backend is degraded:

```rust
pub struct CircuitBreaker {
    state: Signal<CircuitState>,        // Closed | Open | HalfOpen
    failure_count: Signal<u32>,
    last_failure: Signal<Option<Instant>>,
    success_count: Signal<u32>,
    threshold: u32,                     // 5 failures → open
    timeout: Duration,                  // 30s → half-open
    half_open_max: u32,                 // 3 successes → close
}

pub enum CircuitState {
    Closed,        // Normal operation, count failures
    Open,          // Failing, reject calls immediately
    HalfOpen,      // Testing recovery, allow limited calls
}
```

When circuit is OPEN:
- API calls return `ApiError::CircuitOpen` immediately
- UI shows "Service temporarily unavailable" banner
- After timeout, transition to HALF_OPEN and allow 3 test calls
- If all succeed → CLOSED. If any fail → OPEN again.

### 5.7 Repository Pattern

Data access is abstracted behind repository traits:

```rust
#[async_trait]
pub trait FileRepository {
    async fn list(&self, path: &str, params: ListParams) -> Result<PaginatedList<FileItem>>;
    async fn get(&self, id: &str) -> Result<FileItem>;
    async fn create(&self, params: CreateFileParams) -> Result<FileItem>;
    async fn update(&self, id: &str, params: UpdateFileParams) -> Result<FileItem>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn move_file(&self, id: &str, dest: &str) -> Result<FileItem>;
    async fn copy(&self, id: &str, dest: &str) -> Result<FileItem>;
}

#[async_trait]
pub trait NoteRepository {
    async fn list(&self, params: ListParams) -> Result<PaginatedList<Note>>;
    async fn get(&self, id: &str) -> Result<Note>;
    async fn create(&self, params: CreateNoteParams) -> Result<Note>;
    async fn update(&self, id: &str, params: UpdateNoteParams) -> Result<Note>;
    async fn delete(&self, id: &str) -> Result<()>;
}
```

Implementations:
- `ApiRepository` — calls generated API client
- `CacheRepository` — wraps ApiRepository with IndexedDB cache
- `OfflineRepository` — queue mutations when offline, sync when online

This enables testing with mock repositories and offline mode without changing business logic.

---

## 6. WebSocket Architecture

### 6.1 Connection Manager

```rust
pub struct WebSocketManager {
    url: String,
    status: Signal<ConnectionStatus>,
    message_tx: Callback<WsMessage>,
    message_rx: Signal<Option<WsMessage>>,
    reconnect_delay: Duration,     // Starts at 1s, doubles up to 30s
    max_reconnect_attempts: u32,   // 10 before giving up
}
```

### 6.2 Message Protocol

```json
{
  "type": "file.updated",
  "payload": {
    "file_id": "abc123",
    "changes": ["name", "size"],
    "timestamp": "2026-07-21T10:30:00Z"
  },
  "request_id": "req_xyz"
}
```

### 6.3 Subscription Model

```rust
// Subscribe to specific event types
let file_events = ws.subscribe::<FileEvent>("file.*");
let chat_messages = ws.subscribe::<ChatMessage>("chat.message");
let notifications = ws.subscribe::<Notification>("notification.*");
```

### 6.4 Reconnection Strategy

```
Connected → Disconnected
    │
    ▼
  Wait 1s → Attempt reconnect
    │
    ├─ Success → Connected, replay missed messages
    │
    └─ Failure → Wait 2s → Attempt reconnect
                    │
                    └─ Failure → Wait 4s → ... (exponential backoff, max 30s)
                                    │
                                    └─ After 10 failures → Status: Disconnected, manual retry
```

---

## 7. Security Architecture

### 7.1 Content Security Policy

```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  style-src 'self' 'unsafe-inline';
  img-src 'self' blob: data:;
  font-src 'self';
  connect-src 'self' wss://*.ferro.internal;
  frame-ancestors 'none';
  base-uri 'self';
  form-action 'self';
```

### 7.2 Input Sanitization

| Layer               | Approach                                              |
|---------------------|-------------------------------------------------------|
| Component boundary  | Every `<Input>` sanitizes on `onChange`               |
| API boundary        | Generated client validates against schema before send |
| Render boundary     | Leptos auto-escapes all text interpolation            |
| Rich content        | DOMPurify-equivalent for markdown/HTML preview        |

### 7.3 Authentication Flow

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Login   │ ──▶ │  Cookie  │ ──▶ │  API     │
│  Form    │     │  Set     │     │  Calls   │
└──────────┘     │ HttpOnly │     │ Bearer   │
                 │ SameSite │     │ Token    │
                 └──────────┘     └──────────┘
```

- Tokens stored in httpOnly cookies (not localStorage)
- CSRF: SameSite=Strict + custom `X-CSRF-Token` header
- Session timeout: 30 minutes idle → redirect to login
- No secrets in URL parameters

### 7.4 XSS Prevention

| Vector              | Mitigation                                        |
|---------------------|--------------------------------------------------|
| User input in DOM   | Leptos auto-escaping (no `inner_html` unless marked safe) |
| Markdown rendering  | Parse → sanitize → render pipeline                |
| URL parameters      | Validate against whitelist, encode output         |
| File names          | Sanitize before display, no path traversal        |
| SVG upload          | Strip `<script>` tags, validate SVG structure     |

### 7.5 Rate Limiting

| Scope               | Strategy                                          |
|---------------------|---------------------------------------------------|
| API calls           | Client-side debounce (150ms for search, 500ms for mutations) |
| File uploads        | Max 5 concurrent, queue with progress              |
| WebSocket messages  | Max 100 messages/minute per connection             |
| Retry backoff       | Exponential: 1s → 2s → 4s → ... → 30s max        |

### 7.6 Audit Trail

Every user action that modifies state is logged client-side:

```rust
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,        // FileCreate, FileDelete, ShareCreate, etc.
    pub resource_type: String,      // "file", "share", "note", etc.
    pub resource_id: String,
    pub details: HashMap<String, Value>,
    pub session_id: String,
}

pub enum AuditAction {
    FileCreate, FileDelete, FileMove, FileCopy, FileRename,
    FileUpload, FileDownload, FileShare, FileLock,
    NoteCreate, NoteUpdate, NoteDelete,
    TaskCreate, TaskUpdate, TaskDelete,
    UserLogin, UserLogout, SettingsChange,
    AdminUserCreate, AdminUserDelete, AdminPolicyChange,
}
```

Audit entries are batched (max 50 or 5s interval) and sent to `/api/audit` endpoint.

---

## 8. Performance Architecture

### 8.1 Bundle Optimization

| Strategy                      | Target                | Implementation                    |
|------------------------------|-----------------------|-----------------------------------|
| Route-based code splitting    | < 50KB initial load   | `LazyRoute` with dynamic imports  |
| WASM binary splitting         | < 200KB initial WASM  | Trunk wasm-bindgen code splitting |
| CSS extraction                | Critical CSS inline   | Trunk CSS minification            |
| Static asset caching          | Immutable assets      | Content hash in filenames         |
| Service worker                | Offline static cache  | Precache manifest generation      |

### 8.2 Runtime Performance

| Technique                    | Application                                    |
|-----------------------------|-------------------------------------------------|
| Virtual scrolling           | File lists, contact lists (> 1000 items)        |
| Memoized computations       | File tree traversal, search filtering           |
| Debounced inputs            | Search, settings changes (150ms)                |
| Lazy image loading          | Photo grids, thumbnails (IntersectionObserver)  |
| Request coalescing          | Batch API calls within 50ms window              |
| Optimistic updates          | File operations, task moves, note edits         |

### 8.3 Memory Management

```rust
// Prevent memory leaks in long-running sessions
pub struct MemoryGuard {
    // Clear stale subscriptions after 5 minutes
    subscription_ttl: Duration,
    // Limit cached server responses to 100 entries
    cache_max_size: usize,
    // Garbage collect unused image blobs
    image_cache_max_memory: usize,  // 50MB
}
```

### 8.4 Performance Budgets

| Metric                        | Budget     | Measurement                           |
|------------------------------|------------|---------------------------------------|
| First Contentful Paint       | < 800ms    | Lighthouse WASM metric                |
| Largest Contentful Paint     | < 1.2s     | Lighthouse WASM metric                |
| Time to Interactive          | < 2.0s     | Lighthouse WASM metric                |
| Cumulative Layout Shift      | < 0.1      | Lighthouse WASM metric                |
| Total Blocking Time          | < 200ms    | Lighthouse WASM metric                |
| WASM bundle size             | < 300KB    | gzip compressed                       |
| JS bundle size               | < 50KB     | gzip compressed                       |
| CSS bundle size              | < 30KB     | gzip compressed                       |
| Interaction latency (p99)    | < 100ms    | Custom performance measurement        |
| Memory usage (steady state)  | < 100MB    | Chrome DevTools heap snapshot         |

### 8.5 Per-Component Latency Budgets

| Component | Render Budget | Interaction Budget | Measurement |
|-----------|--------------|-------------------|-------------|
| FileBrowser (1000 items) | < 50ms | < 16ms (60fps) | `performance.now()` in mount |
| CommandPalette | < 30ms | < 16ms | Fuzzy search + render |
| Dialog | < 20ms | < 16ms | Open/close animation |
| Toast | < 10ms | N/A | Show/dismiss |
| PhotoGrid (100 items) | < 100ms | < 16ms | Lazy load + render |
| ChatPanel (100 messages) | < 80ms | < 16ms | Message render |
| NotesEditor | < 50ms | < 16ms | keystroke → preview update |
| WhiteboardCanvas | < 16ms | < 16ms | Drawing tool response |

### 8.6 Memory Allocation Strategy

```rust
// Zero-allocation hot paths
pub struct MemoryStrategy {
    // Pre-allocate file list buffer (1000 items × ~200 bytes = 200KB)
    pub file_list_buffer: Vec<FileItem>,
    
    // Reuse message buffer for WebSocket (max 64KB per message)
    pub message_buffer: Vec<u8>,
    
    // Pre-allocate DOM node pool for virtual scrolling
    pub dom_node_pool: Vec<web_sys::Element>,
    
    // Image blob cache with LRU eviction (50MB max)
    pub image_cache: LruCache<String, Vec<u8>>,
}

// Allocation rules:
// 1. Never allocate in render loops
// 2. Never allocate in event handlers (reuse buffers)
// 3. Pre-allocate all collections at component mount
// 4. Use arena allocation for temporary strings
// 5. Pool DOM nodes for virtual scrolling
```

---

## 9. Offline Architecture

### 9.1 Cache Strategy

```
┌─────────────────────────────────────────────────┐
│                  IndexedDB                       │
├─────────────────────────────────────────────────┤
│  files/         File metadata + content cache    │
│  notes/         Notes content cache              │
│  tasks/         Tasks state cache                │
│  queue/         Pending mutations (offline ops)  │
│  sync/          Sync metadata (last sync time)   │
└─────────────────────────────────────────────────┘
```

### 9.2 Conflict Resolution

| Operation    | Strategy                                          |
|-------------|---------------------------------------------------|
| Create       | Generate local ID, sync when online               |
| Update       | Last-write-wins with version vector               |
| Delete       | Soft delete locally, confirm when online          |
| Move/Rename  | Queue, apply when online, re-fetch if conflict    |
| Upload       | Queue with progress, retry on failure             |

### 9.3 Sync Protocol

```
Online? ──Yes──▶ Fetch changes since last_sync
   │              │
   │              ├─ Apply server changes to local cache
   │              ├─ Apply local queue to server
   │              ├─ Resolve conflicts (last-write-wins)
   │              └─ Update last_sync timestamp
   │
   No ──▶ Queue mutation locally
           │
           └─ Show offline indicator, queue counter
```

---

## 10. Desktop Integration

### 10.1 Tauri v2 Wrapper

The desktop app is a thin Tauri v2 wrapper around the same frontend:

```toml
# tauri.conf.json (simplified)
{
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:8080"
  },
  "app": {
    "windows": [
      {
        "title": "Ferro",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ]
  }
}
```

### 10.2 Platform-Specific Features

| Feature               | Implementation                                    |
|-----------------------|---------------------------------------------------|
| System tray           | Tauri tray plugin, context menu                   |
| Notifications         | Tauri notification plugin, OS-level notifications |
| File associations     | Tauri file association config                     |
| Auto-update           | Tauri updater plugin, GitHub releases             |
| Shell integration     | Custom protocol handler (ferro://)                |
| Mount/sync            | Tauri filesystem plugin, sync directories         |
| OS info               | Tauri os plugin, platform detection               |

### 10.3 Desktop-Specific Workarounds

```
// GDK_BACKEND=x11 workaround — stays in Tauri layer, not frontend
// Applied via environment variable before Tauri window creation
// Frontend code is desktop-agnostic
```

---

## 11. Testing Strategy

### 11.1 Test Pyramid

```
         ╱╲
        ╱  ╲
       ╱ E2E╲          5% — Critical user journeys
      ╱──────╲
     ╱ Integr.╲        25% — Component trees + mock API
    ╱──────────╲
   ╱    Unit    ╲      70% — Individual components + utilities
  ╱──────────────╲
```

### 11.2 Test Types

| Type               | Framework               | Scope                                    | Speed     |
|--------------------|------------------------|------------------------------------------|-----------|
| Unit               | `wasm-bindgen-test`    | Single component, utility function        | < 1s      |
| Integration        | `wasm-bindgen-test`    | Component tree with mock API              | < 5s      |
| E2E                | Playwright              | Full user journeys in real browser        | < 60s     |
| Visual regression  | Playwright screenshots  | Component appearance across themes        | < 30s     |
| Performance        | Lighthouse WASM         | Bundle size, load time, interaction       | < 30s     |
| Security           | OWASP ZAP               | XSS, CSRF, CSP compliance                | < 120s    |
| Accessibility      | axe-core                | WCAG 2.1 AA violations                    | < 30s     |
| Property-based     | `proptest-rs`           | State machine correctness, data invariants | < 10s    |

### 11.3 Test Data Management

```rust
// Shared test fixtures
pub mod fixtures {
    pub fn file_items(count: usize) -> Vec<FileItem> { /* ... */ }
    pub fn notes(count: usize) -> Vec<Note> { /* ... */ }
    pub fn tasks(count: usize) -> Vec<Task> { /* ... */ }
    pub fn user_session() -> AuthState { /* ... */ }
    pub fn offline_queue() -> OfflineQueueState { /* ... */ }
}
```

---

## 12. Internationalization (i18n)

### 12.1 Translation Strategy

| Aspect              | Approach                                           |
|--------------------|----------------------------------------------------|
| Framework          | `rust-i18n` crate or custom Leptos context provider |
| Translation files  | TOML format, one file per language                  |
| Key format         | `feature.component.element` (e.g., `files.button.upload`) |
| Languages          | English (default), Spanish, French, German, Japanese, Chinese |
| RTL support        | CSS logical properties (inline-start/end)           |

### 12.2 Translation Key Structure

```toml
# locales/en.toml
[files]
upload = "Upload"
download = "Download"
delete = "Delete"
rename = "Rename"
move = "Move to..."
copy = "Copy to..."
share = "Share"

[files.dialog.delete]
title = "Delete file?"
message = "Are you sure you want to delete {name}?"
confirm = "Delete"
cancel = "Cancel"

[files.toast.upload_success]
message = "File uploaded successfully"
action = "View"
```

---

## 13. CQRS and Event Sourcing

### 13.1 CQRS for Complex Domains

For domains with complex read/write patterns, separate read and write models:

| Domain | Read Model | Write Model | Sync Strategy |
|--------|-----------|-------------|---------------|
| File Browser | Cached file list (stale-while-revalidate) | Optimistic mutations → queue → server | WebSocket file events invalidate cache |
| Notes | IndexedDB cache + server cache | Command queue → server | Conflict resolution on sync |
| Tasks | Kanban view state (local) | Command queue → server | WebSocket task events |
| Admin | Server state (no local cache) | Direct API calls (admin operations not offline-capable) | N/A |

### 13.2 Atomic Operations

File operations are atomic to prevent partial state:

```rust
pub struct AtomicOperation {
    pub id: Uuid,
    pub operations: Vec<Operation>,
    pub status: Signal<OpStatus>,    // Pending | Applied | RolledBack
    pub created_at: DateTime<Utc>,
}

pub enum OpStatus {
    Pending,      // Queued, not yet sent to server
    Applied,      // Server confirmed all operations
    RolledBack,   // Server rejected, local state restored
    PartialFail,  // Some operations succeeded, needs manual resolution
}

// Example: Move file + update references atomically
let op = AtomicOperation {
    id: Uuid::new_v4(),
    operations: vec![
        Operation::Move { from: "/docs/old.pdf", to: "/archive/old.pdf" },
        Operation::UpdateReference { note_id: "note-123", old_path: "/docs/old.pdf", new_path: "/archive/old.pdf" },
    ],
    ..
};
```

If any operation in the atomic set fails, all are rolled back locally and user is notified.

### 13.3 State Machine Patterns

Domain entities use explicit state machines:

```rust
// File state machine
pub enum FileState {
    Active,         // Normal state
    Locked,         // Locked by user
    Syncing,        // Being synced to/from server
    Offline,        // Cached locally, pending sync
    Deleted,        // Soft-deleted, in trash
    Purged,         // Permanently deleted
}

impl FileState {
    pub fn transition(&self, event: FileEvent) -> Result<Self> {
        match (self, event) {
            (Active, FileEvent::Lock) => Ok(Locked),
            (Active, FileEvent::Delete) => Ok(Deleted),
            (Active, FileEvent::SyncStart) => Ok(Syncing),
            (Locked, FileEvent::Unlock) => Ok(Active),
            (Syncing, FileEvent::SyncComplete) => Ok(Active),
            (Syncing, FileEvent::SyncFail) => Ok(Offline),
            (Offline, FileEvent::SyncStart) => Ok(Syncing),
            (Deleted, FileEvent::Restore) => Ok(Active),
            (Deleted, FileEvent::Purge) => Ok(Purged),
            _ => Err(Error::InvalidTransition),
        }
    }
}
```

State transitions are logged to audit trail. Invalid transitions are rejected with error message.

### 13.4 Strategy Pattern for Algorithms

Algorithms are interchangeable via trait objects:

```rust
// Sort strategy
pub trait SortStrategy {
    fn sort(&self, items: &mut Vec<FileItem>, order: SortOrder);
}

pub struct NameSort;
pub struct DateSort;
pub struct SizeSort;
pub struct TypeSort;

impl SortStrategy for NameSort {
    fn sort(&self, items: &mut Vec<FileItem>, order: SortOrder) {
        items.sort_by(|a, b| match order {
            SortOrder::Asc => a.name.cmp(&b.name),
            SortOrder::Desc => b.name.cmp(&a.name),
        });
    }
}

// Search strategy
pub trait SearchStrategy {
    fn search(&self, query: &str, items: &[FileItem]) -> Vec<FileItem>;
}

pub struct FuzzySearch;
pub struct ExactSearch;
pub struct RegexSearch;

// Conflict resolution strategy
pub trait ConflictResolution {
    fn resolve(&self, local: &Mutation, remote: &Mutation) -> Resolution;
}

pub struct LastWriteWins;
pub struct ManualResolution;
pub struct MergeResolution;
```

Strategies are selected at runtime based on user preferences or system configuration.

### 13.2 Event Sourcing for Collaboration

Whiteboard and chat use event sourcing for conflict-free collaboration:

```rust
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,       // Whiteboard or Chat room ID
    pub sequence_number: u64,     // Monotonic per aggregate
    pub event_type: String,       // "whiteboard.element.added"
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
}

pub trait EventStore {
    fn append(&self, events: &[EventEnvelope]) -> Result<()>;
    fn get_events(&self, aggregate_id: Uuid, after_sequence: u64) -> Result<Vec<EventEnvelope>>;
    fn get_snapshot(&self, aggregate_id: Uuid) -> Result<Option<Snapshot>>;
}
```

Snapshot every 100 events for performance. Rebuild state from events on connection.

**Whiteboard Events:**
- `whiteboard.element.added` — New drawing element
- `whiteboard.element.updated` — Element modified (position, style)
- `whiteboard.element.removed` — Element deleted
- `whiteboard.cursor.moved` — User cursor position update
- `whiteboard.viewport.changed` — User zoom/pan change

**Chat Events:**
- `chat.message.sent` — New message
- `chat.message.edited` — Message edited
- `chat.message.deleted` — Message deleted
- `chat.user.typing` — Typing indicator
- `chat.user.joined` — User joined room
- `chat.user.left` — User left room

Conflict resolution: Last-write-wins for concurrent edits to same element. Operation transforms for chat messages (insert/delete at position).

### 13.3 Plugin Architecture

Frontend supports plugins via WebAssembly modules:

```rust
pub trait FrontendPlugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn render_settings(&self) -> Option<Box<dyn Fn() -> Element>>;
    fn on_file_action(&self, action: FileAction) -> Option<FileAction>;
    fn on_route(&self, route: &str) -> Option<Element>;
}

pub struct PluginManager {
    plugins: Vec<Box<dyn FrontendPlugin>>,
    wasm_cache: HashMap<String, Vec<u8>>,
}
```

Plugins are loaded from `/api/v1/plugins/{id}/wasm` and sandboxed via WASM isolation.

---

## 14. Observability

### 14.1 Structured Logging

```rust
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,           // Debug | Info | Warn | Error
    pub component: String,         // "FileBrowser", "WebSocket", etc.
    pub event: String,             // "file.upload.started"
    pub context: HashMap<String, serde_json::Value>,
    pub duration_ms: Option<u64>,
}
```

### 14.2 Performance Metrics

| Metric                          | Collection Method                              |
|--------------------------------|------------------------------------------------|
| Render time per component      | `performance.now()` wrapper in component mount  |
| API response time              | HTTP client timing middleware                   |
| WebSocket latency              | Ping/pong measurement                          |
| Memory usage                   | `performance.memory` (Chrome) or heap snapshot |
| Bundle load time               | Service worker timing                           |
| Error rate                     | Error boundary catch count                     |

### 14.3 Error Tracking

```
Component Error
    │
    ├─ Log to structured logger (always)
    ├─ Send to error tracking service (if online)
    ├─ Show user-friendly fallback (always)
    └─ Offer retry action (if recoverable)
```

### 14.4 Monitoring and Alerting

| Metric | Threshold | Action |
|--------|-----------|--------|
| Error rate (5min window) | > 5% of requests | Alert, enable verbose logging |
| API response time (p99) | > 500ms | Alert, investigate backend |
| WebSocket disconnect rate | > 10/hour | Alert, check network/backend |
| WASM load time | > 3s | Alert, investigate CDN/bundle |
| Memory usage (steady state) | > 200MB | Alert, investigate memory leak |
| Crash rate | > 0.1% of sessions | Alert, immediate investigation |

Monitoring integrates with backend observability (`crates/observability/`) via structured log shipping.

## 15. Accessibility Specification

### 15.1 WCAG 2.1 AA Success Criteria

| Criterion | Requirement | Implementation |
|-----------|-------------|----------------|
| 1.1.1 Non-text Content | All images have alt text | `alt` attribute on `<img>`, `aria-label` on icons |
| 1.3.1 Info and Relationships | Semantic HTML | Use `<nav>`, `<main>`, `<aside>`, `<table>` correctly |
| 1.4.3 Contrast (Minimum) | 4.5:1 for text, 3:1 for UI | Theme tokens verified against contrast checker |
| 1.4.11 Non-text Contrast | 3:1 for UI components | Focus rings, borders, icons meet ratio |
| 2.1.1 Keyboard | All functionality via keyboard | Tab order, Enter/Space activation, Arrow navigation |
| 2.1.2 No Keyboard Trap | Focus can always escape | FocusTrap component releases on Escape |
| 2.4.1 Bypass Blocks | Skip navigation link | First element in DOM is skip link |
| 2.4.3 Focus Order | Logical tab order | DOM order matches visual order |
| 2.4.7 Focus Visible | Clear focus indicator | 2px solid accent color, 2px offset |
| 3.3.1 Error Identification | Errors described in text | `aria-describedby` links input to error message |
| 4.1.2 Name, Role, Value | ARIA attributes on all interactive elements | `role`, `aria-label`, `aria-expanded`, etc. |

### 15.2 Keyboard Navigation Patterns

| Pattern | Keys | Behavior |
|---------|------|----------|
| List navigation | ↑/↓ | Move focus between items |
| Grid navigation | ↑/↓/←/→ | Move focus in grid |
| Activate | Enter/Space | Toggle selection, open item |
| Multi-select | Ctrl+Click | Add/remove from selection |
| Range select | Shift+Click | Select range |
| Escape | Esc | Close dialog, deselect, cancel |
| Command palette | Ctrl+K | Open/close command palette |
| Search | / | Focus search input |

### 15.3 Screen Reader Support

- All interactive elements have `aria-label` or visible text
- Dynamic content uses `aria-live="polite"` for non-urgent updates
- Dynamic content uses `aria-live="assertive"` for urgent alerts
- Loading states announced via `aria-busy` and `role="status"`
- Modal dialogs use `aria-modal="true"` and trap focus
- File operations announce result: "File uploaded successfully" or "Delete failed"

### 15.4 Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

All animations respect `prefers-reduced-motion`. Users can also toggle animation off in Settings > Appearance.

- [ADR-001: Complete GUI Rewrite](./ADR-001-GUI-REWRITE.md)
- [GUI Rewrite Roadmap](../08_roadmap/GUI_REWRITE_ROADMAP.md)
- [Security Specification](../03_security/SECURITY_SPEC.md)
- [Performance Specification](../04_performance/PERFORMANCE_SPEC.md)
- [Leptos Book](https://book.leptos.dev/)

# Ferro System Requirements — EARS Specification

**Document ID:** FERRO-REQ-001
**Date:** 2026-04-18
**Status:** Draft
**Notation:** EARS (Easy Approach to Requirements Syntax)

---

## EARS Pattern Reference

| Pattern | Syntax | Usage |
|---------|--------|-------|
| Ubiquitous | The system shall [action] | Shall always hold |
| Event-Driven | When [trigger], the system shall [action] | Triggered by event |
| Unwanted Behaviour | If [condition], then the system shall [action] | Error/negative path |
| State-Driven | While [state], the system shall [action] | Holds during state |
| Optional Feature | Where [feature], the system shall [action] | Feature-gated |

---

## Storage (REQ-STOR)

### REQ-STOR-001 — Multi-Backend Storage Abstraction

| Field | Value |
|-------|-------|
| **ID** | REQ-STOR-001 |
| **Title** | Multi-Backend Storage Abstraction |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall provide a unified storage interface that abstracts Local FS, Amazon S3, Google Cloud Storage, and Azure Blob Storage backends behind a single API. |
| **Priority** | Must |
| **Phase** | 1 |
| **Applicable Standards** | ISO 27001 A.5.14, NIST SC-8, OWASP A02 |
| **Verification Method** | Integration test: configure each backend; PUT/GET/DELETE/HEAD operations succeed against all four backends using identical API calls. |

### REQ-STOR-002 — Content-Addressable Storage

| Field | Value |
|-------|-------|
| **ID** | REQ-STOR-002 |
| **Title** | Content-Addressable Storage (SHA-256) |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall index all stored files by their SHA-256 content hash, ensuring that the same content is never stored twice within a single storage backend. |
| **Priority** | Must |
| **Phase** | 1 |
| **Applicable Standards** | NIST SC-13, GDPR Art. 17 |
| **Verification Method** | Integration test: upload identical file from two paths; verify single physical copy exists; verify both paths resolve to same content on GET. |

### REQ-STOR-003 — Cross-Account Deduplication

| Field | Value |
|-------|-------|
| **ID** | REQ-STOR-003 |
| **Title** | Cross-Account Deduplication |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall deduplicate file content across user accounts by sharing a single physical copy per unique SHA-256 hash, while maintaining per-user metadata ownership records. |
| **Priority** | Should |
| **Phase** | 2 |
| **Applicable Standards** | GDPR Art. 17 (erasure requires reference counting), GDPR Art. 5 (data minimization) |
| **Verification Method** | Integration test: upload identical file from two different user accounts; verify single physical copy; delete one user's copy; verify physical copy retained for other user; delete last reference; verify physical copy removed. |

### REQ-STOR-004 — Pre-Signed URL Generation

| Field | Value |
|-------|-------|
| **ID** | REQ-STOR-004 |
| **Title** | Pre-Signed URL Generation for Direct-to-Cloud Uploads |
| **EARS Pattern** | When [trigger], the system shall [action] |
| **Statement** | When a client requests to upload or download a file larger than a configurable size threshold, the system shall generate a pre-signed URL scoped to the specific object, HTTP method, and a short-lived time window (default 60 seconds), allowing the client to transfer data directly to the cloud backend bypassing the Ferro server. |
| **Priority** | Should |
| **Phase** | 1 |
| **Applicable Standards** | ISO 27001 A.5.14, NIST SC-12, OWASP A10 (SSRF prevention) |
| **Verification Method** | Integration test: request pre-signed URL; verify URL is scoped to single object and method; verify URL expires within configured window; verify direct upload via URL succeeds; verify expired URL is rejected. |

### REQ-STOR-005 — Atomic Write Operations

| Field | Value |
|-------|-------|
| **ID** | REQ-STOR-005 |
| **Title** | Atomic Write Operations |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall perform all file write operations atomically such that a failed or partial write does not corrupt existing data, using a write-before-commit pattern with a staging area. |
| **Priority** | Must |
| **Phase** | 1 |
| **Applicable Standards** | NIST SC-13, ISO 27001 A.8.8 |
| **Verification Method** | Property-based test: generate random file writes; inject failures at various stages (network, process kill); verify no partial or corrupt files exist after recovery; verify existing files are unmodified. |

### REQ-STOR-006 — Concurrent Upload Handling

| Field | Value |
|-------|-------|
| **ID** | REQ-STOR-006 |
| **Title** | Concurrent Upload Handling |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall handle concurrent uploads to the same file path by serializing completion and ensuring the final state is determined by the last writer, with no data corruption from interleaved writes. |
| **Priority** | Must |
| **Phase** | 1 |
| **Applicable Standards** | NIST SC-13 |
| **Verification Method** | Concurrency test: 10 clients concurrently upload different content to the same path; verify exactly one file exists after all complete; verify CAS reference count is correct. |

---

## WebDAV (REQ-WEBDAV)

### REQ-WEBDAV-001 — RFC 4918 Class 1 Compliance

| Field | Value |
|-------|-------|
| **ID** | REQ-WEBDAV-001 |
| **Title** | RFC 4918 Class 1 Compliance |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall implement all mandatory WebDAV Class 1 methods (PROPFIND, PROPPATCH, MKCOL, DELETE, PUT, COPY, MOVE) and status codes (207 Multi-Status, 507 Insufficient Storage) in compliance with RFC 4918 sections 8.1–8.8 and 10. |
| **Priority** | Must |
| **Phase** | 1 |
| **Applicable Standards** | RFC 4918 |
| **Verification Method** | Integration test: execute every Class 1 method against the WebDAV endpoint; validate request/response bodies against RFC 4918 XML schemas; verify all required DAV: namespace properties are present. |

### REQ-WEBDAV-002 — RFC 4918 Class 2 Compliance (Locking)

| Field | Value |
|-------|-------|
| **ID** | REQ-WEBDAV-002 |
| **Title** | RFC 4918 Class 2 Compliance (Locking) |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall implement WebDAV Class 2 locking (LOCK, UNLOCK methods) supporting both exclusive and shared locks, with lock token management, timeout handling, and correct 423 (Locked) / 424 (Failed Dependency) status codes per RFC 4918 sections 8.9–8.10 and 13. |
| **Priority** | Must |
| **Phase** | 1 |
| **Applicable Standards** | RFC 4918, RFC 3253 |
| **Verification Method** | Integration test: acquire exclusive lock; verify second writer receives 423; release lock; verify second writer succeeds; verify lock timeout expiry releases lock automatically. |

### REQ-WEBDAV-003 — RFC 4918 Class 3 Compliance (ACL Discovery)

| Field | Value |
|-------|-------|
| **ID** | REQ-WEBDAV-003 |
| **Title** | RFC 4918 Class 3 Compliance (ACL Discovery) |
| **EARS Pattern** | Where [Cedar authorization is configured], the system shall [action] |
| **Statement** | Where Cedar authorization is configured, the system shall expose access control lists via the ACL method and supportedlock property in compliance with RFC 3744, translating Cedar policy decisions into WebDAV ACL responses. |
| **Priority** | Should |
| **Phase** | 2 |
| **Applicable Standards** | RFC 3744, Cedar Specification |
| **Verification Method** | Integration test: configure Cedar policies; call ACL method; verify response reflects policy grants/denials; verify principal resources are correctly mapped from OIDC identities. |

### REQ-WEBDAV-004 — rclone Compatibility

| Field | Value |
|-------|-------|
| **ID** | REQ-WEBDAV-004 |
| **Title** | rclone Compatibility |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall be fully mountable via rclone using the WebDAV backend, supporting directory listing, file upload, file download, rename, delete, and copy operations without errors or data corruption. |
| **Priority** | Must |
| **Phase** | 1 |
| **Applicable Standards** | RFC 4918 |
| **Verification Method** | End-to-end test: mount Ferro via `rclone mount webdav:`; execute full rclone operation suite (ls, copy, move, sync, check); verify no errors in rclone output; verify data integrity via checksum comparison. |

### REQ-WEBDAV-005 — Microsoft Office WebDAV Compatibility

| Field | Value |
|-------|-------|
| **ID** | REQ-WEBDAV-005 |
| **Title** | Microsoft Office WebDAV Compatibility |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall support Microsoft Office WebDAV client requirements including OPTIONS method with correct DAV header, LOCK/UNLOCK for edit session management, If-Match/If-None-Match conditional requests, and correct Content-Type handling for Office file formats. |
| **Priority** | Should |
| **Phase** | 2 |
| **Applicable Standards** | RFC 4918, RFC 3253, WOPI Specification |
| **Verification Method** | Integration test: open a .docx file from Ferro via Microsoft Word; edit and save; verify file is updated correctly; verify lock acquisition and release sequence. |

### REQ-WEBDAV-006 — XML Overhead Minimization

| Field | Value |
|-------|-------|
| **ID** | REQ-WEBDAV-006 |
| **Title** | XML Overhead Minimization |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall use a high-performance XML parser (quick-xml) for WebDAV request/response handling and minimize XML serialization overhead by supporting depth-limited PROPFIND responses and omitting empty properties. |
| **Priority** | Should |
| **Phase** | 1 |
| **Applicable Standards** | RFC 4918 |
| **Verification Method** | Performance test: PROPFIND on a directory with 10,000 items; measure response time and serialized payload size; compare against baseline XML library (serde_xml_rs). |

---

## WOPI (REQ-WOPI)

### REQ-WOPI-001 — WOPI Protocol Implementation

| Field | Value |
|-------|-------|
| **ID** | REQ-WOPI-001 |
| **Title** | WOPI Protocol Implementation |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall implement the WOPI host endpoints (CheckFileInfo, GetFile, PutFile, Lock, Unlock) and the WOPI discovery endpoint in compliance with the WOPI specification, enabling collaborative document editing via WOPI-compatible editors. |
| **Priority** | Should |
| **Phase** | 2 |
| **Applicable Standards** | WOPI Specification |
| **Verification Method** | Integration test: call each WOPI endpoint with valid access token; verify response schemas match WOPI specification; verify WOPI discovery XML is valid and points to correct endpoints. |

### REQ-WOPI-002 — Collabora Online Integration

| Field | Value |
|-------|-------|
| **ID** | REQ-WOPI-002 |
| **Title** | Collabora Online Integration |
| **EARS Pattern** | Where [Collabora Online is configured], the system shall [action] |
| **Statement** | Where Collabora Online is configured as a WOPI client, the system shall integrate with Collabora's discovery and editing endpoints, enabling users to open and collaboratively edit office documents through the Collabora interface. |
| **Priority** | Could |
| **Phase** | 3 |
| **Applicable Standards** | WOPI Specification |
| **Verification Method** | End-to-end test: open a .ods file via Collabora Online through Ferro; edit concurrently from two sessions; verify changes are saved correctly via PutFile. |

### REQ-WOPI-003 — OnlyOffice Integration

| Field | Value |
|-------|-------|
| **ID** | REQ-WOPI-003 |
| **Title** | OnlyOffice Integration |
| **EARS Pattern** | Where [OnlyOffice is configured], the system shall [action] |
| **Statement** | Where OnlyOffice is configured as a WOPI client, the system shall integrate with OnlyOffice's document builder and editing endpoints, enabling users to open and collaboratively edit office documents through the OnlyOffice interface. |
| **Priority** | Could |
| **Phase** | 3 |
| **Applicable Standards** | WOPI Specification |
| **Verification Method** | End-to-end test: open a .xlsx file via OnlyOffice through Ferro; edit and save; verify file content is correctly persisted. |

### REQ-WOPI-004 — Session State Management

| Field | Value |
|-------|-------|
| **ID** | REQ-WOPI-004 |
| **Title** | WOPI Session State Management |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall track active WOPI editing sessions, recording which users are editing which files, session start times, and lock ownership, to enable collaborative awareness and session cleanup on disconnect. |
| **Priority** | Should |
| **Phase** | 2 |
| **Applicable Standards** | WOPI Specification |
| **Verification Method** | Integration test: initiate two concurrent editing sessions on the same file; query session state; verify both sessions are tracked; terminate one session; verify session state is updated. |

### REQ-WOPI-005 — File Locking for Concurrent Editing

| Field | Value |
|-------|-------|
| **ID** | REQ-WOPI-005 |
| **Title** | File Locking for Concurrent Editing |
| **EARS Pattern** | When [a WOPI client requests a lock on a file], the system shall [action] |
| **Statement** | When a WOPI client requests a lock on a file, the system shall grant an exclusive lock and prevent any non-lock-holder from modifying the file until the lock is released or expires, ensuring data integrity during collaborative editing sessions. |
| **Priority** | Must |
| **Phase** | 2 |
| **Applicable Standards** | WOPI Specification, RFC 4918, RFC 3253 |
| **Verification Method** | Integration test: lock a file via WOPI; attempt modification via WebDAV from a different client; verify 423 response; unlock via WOPI; verify modification succeeds. |

---

## Identity & Authorization (REQ-AUTH)

### REQ-AUTH-001 — OIDC Authentication

| Field | Value |
|-------|-------|
| **ID** | REQ-AUTH-001 |
| **Title** | OpenID Connect Authentication |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall authenticate users via OpenID Connect using the Authorization Code flow with PKCE (S256 challenge method), supporting any OIDC-compliant identity provider (Keycloak, Authelia, Okta). |
| **Priority** | Must |
| **Phase** | 2 |
| **Applicable Standards** | OpenID Connect Core 1.0, OWASP A07, NIST IA-2, NIST IA-5 |
| **Verification Method** | Integration test: complete full OIDC flow (authorize → callback → token exchange → UserInfo); verify PKCE S256 challenge is enforced; verify token validation including signature verification and nonce validation; verify RP-Initiated Logout clears session. |

### REQ-AUTH-002 — Cedar Policy Engine Integration

| Field | Value |
|-------|-------|
| **ID** | REQ-AUTH-002 |
| **Title** | Cedar Policy Engine Integration |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall integrate the Cedar policy engine as its sole authorization decision point, evaluating principal, action, and resource tuples against administrator-defined policies with deny-by-default semantics. |
| **Priority** | Must |
| **Phase** | 2 |
| **Applicable Standards** | Cedar Specification, NIST AC-3, ISO 27001 A.9.1 |
| **Verification Method** | Integration test: configure Cedar policies with allow/deny rules; make API requests as various principals; verify all decisions match expected policy outcomes; verify deny-by-default for unconfigured actions. |

### REQ-AUTH-003 — Fine-Grained Authorization Policies

| Field | Value |
|-------|-------|
| **ID** | REQ-AUTH-003 |
| **Title** | Fine-Grained Authorization Policies |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall support attribute-based access control (ABAC) policies that reference resource attributes (tags, owner, path), principal attributes (roles, groups from OIDC claims), and environmental context (time, IP address) in policy conditions. |
| **Priority** | Must |
| **Phase** | 2 |
| **Applicable Standards** | Cedar Specification, NIST AC-4, NIST AC-6 |
| **Verification Method** | Integration test: define policies with attribute conditions (e.g., resource.tag == "public"); verify access is granted/denied based on attribute matching; verify OIDC claim attributes are correctly mapped to Cedar principal attributes. |

### REQ-AUTH-004 — Session Management

| Field | Value |
|-------|-------|
| **ID** | REQ-AUTH-004 |
| **Title** | Session Management |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall manage user sessions with configurable idle timeout and absolute session lifetime, supporting refresh token rotation and immediate session revocation on logout. |
| **Priority** | Must |
| **Phase** | 2 |
| **Applicable Standards** | OpenID Connect Core 1.0, OWASP A07, NIST IA-5 |
| **Verification Method** | Integration test: create session; verify access before timeout; wait for idle timeout; verify session is rejected; verify refresh token rotation produces new tokens and invalidates old ones; verify logout revokes session immediately. |

---

## WASM Plugin System (REQ-WASM)

### REQ-WASM-001 — Wasmtime Runtime Integration

| Field | Value |
|-------|-------|
| **ID** | REQ-WASM-001 |
| **Title** | Wasmtime Runtime Integration |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall embed the Wasmtime runtime to load and execute WebAssembly modules compiled from any WASM-targeting language (Rust, AssemblyScript, Go), providing a stable WASI interface for plugin execution. |
| **Priority** | Should |
| **Phase** | 5 |
| **Applicable Standards** | WASM/WASI Specification |
| **Verification Method** | Integration test: compile a simple WASM plugin in Rust; load and execute it via Wasmtime; verify correct output; verify WASI capability restrictions are enforced. |

### REQ-WASM-002 — Event-Driven Worker Execution

| Field | Value |
|-------|-------|
| **ID** | REQ-WASM-002 |
| **Title** | Event-Driven Worker Execution |
| **EARS Pattern** | When [a file operation matches a configured trigger pattern], the system shall [action] |
| **Statement** | When a file operation (upload, modify, delete) matches a configured trigger pattern (file extension glob or path prefix), the system shall dispatch the file metadata and content to the matching WASM worker for processing. |
| **Priority** | Should |
| **Phase** | 5 |
| **Applicable Standards** | WASM/WASI Specification |
| **Verification Method** | Integration test: configure a trigger for `*.pdf`; upload a PDF file; verify the configured WASM worker is invoked with correct file context; verify trigger patterns that do not match do not invoke the worker. |

### REQ-WASM-003 — Plugin Sandboxing

| Field | Value |
|-------|-------|
| **ID** | REQ-WASM-003 |
| **Title** | Plugin Sandboxing |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall sandbox all WASM plugins using WASI capability restrictions, preventing plugins from accessing the filesystem, network, or environment variables except those explicitly granted, and ensuring that a malicious or buggy plugin cannot crash the Ferro server or access unauthorized data. |
| **Priority** | Must |
| **Phase** | 5 |
| **Applicable Standards** | WASM/WASI Specification, OWASP A04, NIST SC-7 |
| **Verification Method** | Security test: load a WASM plugin that attempts filesystem access, network connections, and environment variable reads without granted capabilities; verify all attempts are denied; verify Ferro server remains responsive; verify plugin cannot read files outside its authorized scope. |

### REQ-WASM-004 — Resource Limits for Plugins

| Field | Value |
|-------|-------|
| **ID** | REQ-WASM-004 |
| **Title** | Resource Limits for Plugins |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall enforce configurable per-plugin resource limits including maximum memory allocation, CPU fuel/execution time, and wall-clock timeout, terminating any plugin that exceeds its limits without affecting other plugins or the server. |
| **Priority** | Must |
| **Phase** | 5 |
| **Applicable Standards** | WASM/WASI Specification, ISO 27001 A.8.8 |
| **Verification Method** | Integration test: load a WASM plugin with an infinite loop; verify it is terminated after the configured CPU fuel limit; load a plugin that allocates large arrays; verify it is terminated at the memory limit; verify the server continues processing other requests during and after plugin termination. |

---

## Search (REQ-SEARCH)

### REQ-SEARCH-001 — Tantivy Full-Text Search

| Field | Value |
|-------|-------|
| **ID** | REQ-SEARCH-001 |
| **Title** | Tantivy Full-Text Search |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall provide full-text search across file contents using the Tantivy search engine, supporting text extraction from common document formats (plain text, PDF, Office documents) and returning ranked results with relevance scoring. |
| **Priority** | Should |
| **Phase** | 5 |
| **Applicable Standards** | N/A |
| **Verification Method** | Integration test: index a corpus of documents with known content; execute search queries; verify results are ranked by relevance; verify recall (all matching documents are found); verify precision (non-matching documents are excluded). |

### REQ-SEARCH-002 — Background Content Indexing

| Field | Value |
|-------|-------|
| **ID** | REQ-SEARCH-002 |
| **Title** | Background Content Indexing |
| **EARS Pattern** | When [a file is uploaded or modified], the system shall [action] |
| **Statement** | When a file is uploaded or modified, the system shall enqueue the file for background content extraction and index update, processing the queue asynchronously without blocking the upload response. |
| **Priority** | Should |
| **Phase** | 5 |
| **Applicable Standards** | N/A |
| **Verification Method** | Integration test: upload a file; verify upload response is returned before indexing completes; verify file becomes searchable within a configurable time window; verify modified files are re-indexed with updated content. |

### REQ-SEARCH-003 — Metadata Search

| Field | Value |
|-------|-------|
| **ID** | REQ-SEARCH-003 |
| **Title** | Metadata Search |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall support search queries against file metadata fields including filename, file extension, size, modification date, owner, tags, and custom attributes defined via the Cedar policy model. |
| **Priority** | Should |
| **Phase** | 5 |
| **Applicable Standards** | N/A |
| **Verification Method** | Integration test: create files with various metadata attributes; execute metadata queries (e.g., size > 1MB, tag:invoice, owner:alice); verify results match query predicates; verify combined metadata + full-text queries work correctly. |

---

## Web Frontend (REQ-WEB)

### REQ-WEB-001 — Leptos SSR + WASM Hydration

| Field | Value |
|-------|-------|
| **ID** | REQ-WEB-001 |
| **Title** | Leptos SSR + WASM Hydration |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall render the web frontend using Leptos with Server-Side Rendering (SSR) for the initial page load and WebAssembly hydration for subsequent client-side interactions, providing instant perceived performance on first load. |
| **Priority** | Should |
| **Phase** | 3 |
| **Applicable Standards** | ISO 27034, OWASP A05 |
| **Verification Method** | End-to-end test: load the web frontend; verify initial HTML contains rendered content (not a blank SPA shell); verify hydration completes without errors; verify client-side navigation works without full page reload. |

### REQ-WEB-002 — Virtualized Scrolling

| Field | Value |
|-------|-------|
| **ID** | REQ-WEB-002 |
| **Title** | Virtualized Scrolling for Large Directory Views |
| **EARS Pattern** | While [displaying a directory with more than 1,000 items], the system shall [action] |
| **Statement** | While displaying a directory with more than 1,000 items, the system shall use virtualized scrolling to render only visible rows, maintaining a consistent frame rate (>= 30 FPS) and initial render time under 2 seconds for directories with up to 10,000 items. |
| **Priority** | Should |
| **Phase** | 3 |
| **Applicable Standards** | N/A |
| **Verification Method** | Performance test: populate a directory with 10,000 files; load the directory view; measure initial render time and scrolling frame rate; verify memory usage remains bounded (no DOM node growth proportional to item count). |

### REQ-WEB-003 — Share-Link Management

| Field | Value |
|-------|-------|
| **ID** | REQ-WEB-003 |
| **Title** | Share-Link Management |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall allow users to create share links for files and directories with configurable expiration, password protection, and download limits, and shall enforce these constraints on every access via share link. |
| **Priority** | Should |
| **Phase** | 3 |
| **Applicable Standards** | ISO 27001 A.8.3, GDPR Art. 32 |
| **Verification Method** | Integration test: create a share link with expiration, password, and download limit; access before and after expiration; access with and without password; verify download counter; verify access is denied after limit is reached. |

### REQ-WEB-004 — Admin Dashboard

| Field | Value |
|-------|-------|
| **ID** | REQ-WEB-004 |
| **Title** | Admin Dashboard |
| **EARS Pattern** | Where [the user has administrative privileges], the system shall [action] |
| **Statement** | Where the user has administrative privileges, the system shall provide a dashboard for managing Cedar authorization policies, monitoring WASM worker status and logs, viewing system health metrics, and managing user accounts. |
| **Priority** | Could |
| **Phase** | 3 |
| **Applicable Standards** | ISO 27001 A.8.2, NIST AC-2 |
| **Verification Method** | End-to-end test: log in as admin; verify all dashboard sections are accessible; create, modify, and delete a Cedar policy; verify policy changes take effect; view WASM worker status. |

### REQ-WEB-005 — Type-Safe API via Shared Crate

| Field | Value |
|-------|-------|
| **ID** | REQ-WEB-005 |
| **Title** | Type-Safe API via Shared Crate |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall define all API request/response types, file structures, and shared constants in a `ferro-common` crate shared between the server and the Leptos frontend, ensuring compile-time type safety and preventing API contract drift between frontend and backend. |
| **Priority** | Must |
| **Phase** | 3 |
| **Applicable Standards** | ISO 27034, OWASP A04 |
| **Verification Method** | Build test: modify a response type in `ferro-common`; verify the server and frontend both fail to compile until both are updated; verify no runtime type mismatches can occur. |

---

## Desktop Client (REQ-DESK)

### REQ-DESK-001 — Tauri Shell Wrapper

| Field | Value |
|-------|-------|
| **ID** | REQ-DESK-001 |
| **Title** | Tauri Shell Wrapper |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall provide a Tauri-based desktop application for Windows, macOS, and Linux that wraps the rclone sidecar and provides a native UI for mount status, settings, and sync feedback. |
| **Priority** | Should |
| **Phase** | 4 |
| **Applicable Standards** | ISO 27001 A.8.1 |
| **Verification Method** | Build test: compile Tauri app for all three target platforms (Windows, macOS, Linux); verify application launches and displays the main window. |

### REQ-DESK-002 — rclone Sidecar Management

| Field | Value |
|-------|-------|
| **ID** | REQ-DESK-002 |
| **Title** | rclone Sidecar Management |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall manage the lifecycle of an embedded rclone binary including startup, shutdown, crash recovery with automatic restart, and health monitoring via stdout/stderr parsing. |
| **Priority** | Should |
| **Phase** | 4 |
| **Applicable Standards** | N/A |
| **Verification Method** | Integration test: launch Tauri app; verify rclone sidecar starts; kill rclone process; verify Tauri detects the failure and restarts rclone; verify mount becomes available again. |

### REQ-DESK-003 — Zero-Config Mount

| Field | Value |
|-------|-------|
| **ID** | REQ-DESK-003 |
| **Title** | Zero-Config Mount |
| **EARS Pattern** | When [a user authenticates via the desktop client], the system shall [action] |
| **Statement** | When a user authenticates via the desktop client, the system shall automatically configure rclone with the server URL and OIDC credentials, mount the Ferro storage as a local filesystem drive without requiring any manual configuration from the user, and verify the mount is accessible via the OS file manager. |
| **Priority** | Must |
| **Phase** | 4 |
| **Applicable Standards** | ISO 27001 A.8.1, NIST IA-2 |
| **Verification Method** | End-to-end test: install desktop client; enter OIDC credentials; verify Ferro drive appears in OS file manager; verify file operations (create, read, update, delete) work through the mounted drive without any manual rclone configuration. |

### REQ-DESK-004 — System Tray Integration

| Field | Value |
|-------|-------|
| **ID** | REQ-DESK-004 |
| **Title** | System Tray Integration |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall provide a system tray icon displaying the current sync status (syncing, up to date, offline, error) with a context menu for quick actions (open drive, pause sync, settings, quit). |
| **Priority** | Could |
| **Phase** | 4 |
| **Applicable Standards** | N/A |
| **Verification Method** | End-to-end test: launch desktop client; verify system tray icon appears; verify status updates reflect sync state; verify context menu actions work correctly. |

### REQ-DESK-005 — Native Notifications

| Field | Value |
|-------|-------|
| **ID** | REQ-DESK-005 |
| **Title** | Native Notifications |
| **EARS Pattern** | When [a file is shared with the user or a sync error occurs], the system shall [action] |
| **Statement** | When a file is shared with the user or a sync error occurs, the system shall display a native OS notification with actionable content (e.g., "Open file" button for shared files, "Retry" for sync errors). |
| **Priority** | Could |
| **Phase** | 4 |
| **Applicable Standards** | N/A |
| **Verification Method** | Integration test: trigger a file share event directed at the authenticated user; verify native notification appears with correct content and action button; trigger a sync error; verify error notification appears. |

---

## Enterprise Governance (REQ-ENT)

### REQ-ENT-001 — Immutable Audit Log

| Field | Value |
|-------|-------|
| **ID** | REQ-ENT-001 |
| **Title** | Immutable Audit Log |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall record every authenticated request (WebDAV, WOPI, API) in a structured, append-only audit log containing timestamp, principal, action, resource, result, and client IP, with tamper-evident hash chaining to detect log modification. |
| **Priority** | Must |
| **Phase** | 2 |
| **Applicable Standards** | ISO 27001 A.12.4, NIST AU-2, AU-3, AU-4, AU-9, GDPR Art. 30 |
| **Verification Method** | Integration test: perform a series of operations; query the audit log; verify all operations are recorded with correct fields; attempt to modify a log entry; verify hash chain detects the tampering; verify entries cannot be deleted. |

### REQ-ENT-002 — gRPC Audit Log Streaming

| Field | Value |
|-------|-------|
| **ID** | REQ-ENT-002 |
| **Title** | gRPC Audit Log Streaming |
| **EARS Pattern** | Where [a gRPC log consumer is configured], the system shall [action] |
| **Statement** | Where a gRPC log consumer is configured, the system shall stream audit log entries in real-time to the consumer endpoint, supporting at-least-once delivery semantics with automatic reconnection and backpressure handling. |
| **Priority** | Should |
| **Phase** | 2 |
| **Applicable Standards** | NIST AU-6, SI-4 |
| **Verification Method** | Integration test: configure a gRPC consumer; perform operations; verify log entries are received in real-time; disconnect the consumer; verify entries are buffered; reconnect; verify buffered entries are delivered. |

### REQ-ENT-003 — Ransomware Protection via Instant Snapshots

| Field | Value |
|-------|-------|
| **ID** | REQ-ENT-003 |
| **Title** | Ransomware Protection via Instant Snapshots |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall support instant metadata snapshots that record the complete file tree state (path → CAS hash mapping) at a point in time, enabling administrators to revert the entire file tree to any previous snapshot without moving or duplicating physical data. |
| **Priority** | Should |
| **Phase** | 2 |
| **Applicable Standards** | ISO 27001 A.12.4, NIST AU-9 |
| **Verification Method** | Integration test: create a snapshot; perform bulk file modifications (simulating ransomware); revert to snapshot; verify file tree state matches the snapshot exactly; verify no physical data was copied during snapshot or revert. |

### REQ-ENT-004 — Global Deduplication

| Field | Value |
|-------|-------|
| **ID** | REQ-ENT-004 |
| **Title** | Global Deduplication |
| **EARS Pattern** | Ubiquitous |
| **Statement** | The system shall deduplicate file content globally across all storage backends and user accounts, maintaining a single physical copy per unique SHA-256 hash and tracking per-user/per-path ownership via the metadata engine. |
| **Priority** | Should |
| **Phase** | 5 |
| **Applicable Standards** | GDPR Art. 17, GDPR Art. 5 |
| **Verification Method** | Integration test: upload identical file to two different backends; verify single physical copy in the originating backend; upload from two different users; verify reference counting works correctly; verify deletion of all references removes physical copy. |

### REQ-ENT-005 — Sovereign Proxying

| Field | Value |
|-------|-------|
| **ID** | REQ-ENT-005 |
| **Title** | Sovereign Proxying |
| **EARS Pattern** | Where [sovereign proxying is configured], the system shall [action] |
| **Statement** | Where sovereign proxying is configured, the system shall act as a unified gateway for multiple external storage sources (S3 buckets, local NAS, remote SFTP servers), presenting them as a single namespace to users while proxying all operations to the appropriate backend. |
| **Priority** | Could |
| **Phase** | 5 |
| **Applicable Standards** | ISO 27001 A.5.14, NIST AC-17 |
| **Verification Method** | Integration test: configure two external storage backends (S3 + SFTP); list files; verify unified namespace shows files from both backends; upload a file specifying target backend; verify file is stored on the correct backend. |

---

## Summary Statistics

| Subsystem | Must | Should | Could | Won't | Total |
|-----------|------|--------|-------|-------|-------|
| Storage (REQ-STOR) | 4 | 2 | 0 | 0 | 6 |
| WebDAV (REQ-WEBDAV) | 3 | 3 | 0 | 0 | 6 |
| WOPI (REQ-WOPI) | 1 | 3 | 2 | 0 | 6 |
| Identity (REQ-AUTH) | 4 | 0 | 0 | 0 | 4 |
| WASM (REQ-WASM) | 2 | 2 | 0 | 0 | 4 |
| Search (REQ-SEARCH) | 0 | 3 | 0 | 0 | 3 |
| Web Frontend (REQ-WEB) | 1 | 3 | 1 | 0 | 5 |
| Desktop (REQ-DESK) | 1 | 2 | 2 | 0 | 5 |
| Enterprise (REQ-ENT) | 1 | 3 | 1 | 0 | 5 |
| **Total** | **18** | **21** | **6** | **0** | **44** |

### Requirements by Phase

| Phase | Requirements |
|-------|-------------|
| Phase 1 (The Core) | REQ-STOR-001, REQ-STOR-002, REQ-STOR-004, REQ-STOR-005, REQ-STOR-006, REQ-WEBDAV-001, REQ-WEBDAV-002, REQ-WEBDAV-004, REQ-WEBDAV-006 |
| Phase 2 (The Metadata) | REQ-STOR-003, REQ-WEBDAV-003, REQ-WEBDAV-005, REQ-WOPI-001, REQ-WOPI-004, REQ-WOPI-005, REQ-AUTH-001, REQ-AUTH-002, REQ-AUTH-003, REQ-AUTH-004, REQ-ENT-001, REQ-ENT-002, REQ-ENT-003 |
| Phase 3 (The Frontend) | REQ-WOPI-002, REQ-WOPI-003, REQ-WEB-001, REQ-WEB-002, REQ-WEB-003, REQ-WEB-004, REQ-WEB-005 |
| Phase 4 (The Desktop) | REQ-DESK-001, REQ-DESK-002, REQ-DESK-003, REQ-DESK-004, REQ-DESK-005 |
| Phase 5 (The Intelligence) | REQ-WASM-001, REQ-WASM-002, REQ-WASM-003, REQ-WASM-004, REQ-SEARCH-001, REQ-SEARCH-002, REQ-SEARCH-003, REQ-ENT-004, REQ-ENT-005 |

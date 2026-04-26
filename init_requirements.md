### I. The Core Architecture (Ferro-Server)
The server is a stateless, high-concurrency binary designed to run on everything from a Raspberry Pi to a Kubernetes cluster.

*   **Runtime:** `Tokio` for asynchronous I/O and `Axum` for the HTTP stack.
*   **Storage Orchestration (`object_store` Crate):** 
    *   Ferro does not "own" the disk. It abstracts storage.
    *   Supports **Local FS, S3, GCS, and Azure Blob** concurrently.
    *   **Direct-to-Cloud Path:** For large files, Ferro generates S3 Pre-signed URLs, allowing the client (rclone) to upload/download directly to the cloud provider, bypassing Ferro’s CPU/RAM to ensure 10Gbps+ throughput.
*   **Metadata Engine:**
    *   **PostgreSQL (via SQLx):** For enterprise deployments requiring High Availability.
    *   **LibSQL/SQLite:** For edge/local deployments.
    *   **Content-Addressable Storage (CAS):** Files are indexed by their SHA-256 hash. If 1,000 users upload the same company handbook, Ferro stores the bytes once (Deduplication).
*   **Search Engine (Tantivy):** 
    *   An embedded, full-text search engine written in Rust.
    *   Indexes file metadata and content (via background workers) without requiring an external ElasticSearch cluster.

---

### II. The Unified Interface Layer
Ferro focuses on industry-standard protocols to ensure maximum compatibility without proprietary clients.

*   **Native WebDAV (Class 1, 2, 3):** 
    *   Specifically tuned for **rclone** and **Microsoft Office** (WebDAV locking).
    *   High-performance implementation to minimize the "XML overhead" of WebDAV.
*   **WOPI (Web Application Open Platform Interface):**
    *   Native implementation of the WOPI protocol to allow **Collabora Online**, **OnlyOffice**, or **Microsoft Office Online** to open and edit files directly from Ferro.
    *   State management for collaborative sessions (Who is currently editing?).
*   **Identity (OIDC & Cedar):**
    *   **SSO:** Native OpenID Connect support (Keycloak, Authelia, Okta).
    *   **Authorization (Cedar Policy Language):** Instead of simple "Read/Write" flags, Ferro uses AWS’s **Cedar** engine. Admins write policies like:
        `allow(principal, action == "view", resource) when { resource.tag == "public" };`
    *   This allows for ultra-granular, verifiable enterprise permissions.

---

### III. The "Active FS" & Extensibility (WASM)
Ferro isn't just a place to store files; it's a place to process them.

*   **Wasmtime Integration:** Ferro embeds a WebAssembly runtime.
*   **File Workers:** Developers can write plugins in any language (Rust, AssemblyScript, Go) that compile to WASM. 
*   **Event-Driven Execution:** 
    *   When a file matches a certain pattern (e.g., `*.pdf` or `path/to/invoices/*`), Ferro triggers a WASM worker.
    *   **Use Cases:** Auto-OCR, image resizing, virus scanning, or sending a notification to a webhook.
*   **Sandboxing:** WASM ensures these workers cannot crash the main Ferro binary or access files they aren't authorized to see.

---

### IV. The Web Frontend (Leptos)
A zero-JavaScript (TS-free) web experience.

*   **Technology:** `Leptos` (Full-stack Rust web framework).
*   **Rendering:** Server-Side Rendering (SSR) for the initial file tree (instant perceived performance) with WASM hydration for a smooth, "App-like" feel.
*   **Shared Logic:** All file structures and API responses are defined in a `ferro-common` crate, shared between the server and the frontend. **Compile-time guarantees** that the frontend and backend never drift.
*   **Functionality:**
    *   High-speed file navigation (handling 10,000+ files in a single view via virtualized scrolling).
    *   Share-link management (password protection, expiration, download limits).
    *   Admin dashboard for Cedar policy management and WASM worker monitoring.

---

### V. The Desktop Ecosystem (Tauri + rclone)
The bridge between the remote cloud and the local OS.

*   **The Shell (Tauri):** A lightweight Rust-based desktop wrapper for Windows, macOS, and Linux.
*   **The VFS Engine (rclone Sidecar):**
    *   Tauri manages the lifecycle of an embedded `rclone` binary.
    *   **Zero-Config Mount:** Upon login, Tauri injects credentials into rclone and mounts the Ferro drive (e.g., `Z:` on Windows).
    *   **Status Monitoring:** Tauri monitors rclone's stdout/stderr to provide real-time feedback (Upload progress, sync errors, "Offline Mode" status).
*   **Native Integration:**
    *   System tray icons for sync status.
    *   Native OS notifications when a file is shared with the user.
    *   "Open in Browser" context menu shortcuts.

---

### VI. Enterprise Governance & Security
The features that make Ferro a "boring, safe choice" for IT departments.

*   **Immutable Audit Log:** Every request (WebDAV, API, or WOPI) is logged in a structured, append-only format. Logs can be streamed via gRPC to a centralized SOC.
*   **Ransomware Protection (Snapshotting):** 
    *   Because of Content-Addressable Storage, Ferro can provide "Instant Snapshots." 
    *   If a user's drive is encrypted by ransomware, the admin can revert the entire metadata state to "5 minutes ago" without moving any physical data.
*   **Global Deduplication:** Reduces storage TCO by up to 40% in large organizations by never storing the same file twice across different user accounts.
*   **Sovereign Proxying:** Ferro can act as a single gateway for multiple S3 buckets, local NAS, and remote SFTP servers, giving the user one unified "Company Drive."

---

### VII. Implementation Roadmap

1.  **Phase 1 (The Core):** Axum Server + `object_store` + basic WebDAV. Ability to mount Ferro via a standard rclone CLI.
2.  **Phase 2 (The Metadata):** Postgres/SQLx integration + Cedar Policy engine for permissions.
3.  **Phase 3 (The Frontend):** Leptos Web UI for file browsing and OIDC login.
4.  **Phase 4 (The Desktop):** Tauri wrapper for rclone to provide the "one-click" mount experience.
5.  **Phase 5 (The Intelligence):** Tantivy search integration and WASM worker runtime.

**Why Ferro wins:** It is a 100% Rust, type-safe, memory-safe, and high-performance alternative to the bloated incumbents. It leverages the best-in-class VFS (rclone) rather than reinventing it, and it offers the enterprise "governance" they actually need.
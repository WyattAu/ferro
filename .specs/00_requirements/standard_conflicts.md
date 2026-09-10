# Ferro Standard Conflicts Analysis

**Document ID:** FERRO-SC-001
**Date:** 2026-04-18
**Status:** Draft

---

## Conflict 1: GDPR Data Minimization vs. Audit Logging Requirements

**ADR Reference:** FERRO-ADR-001

### Position A — GDPR Data Minimization (GDPR Art. 5)

GDPR Article 5(1)(c) requires that personal data be "adequate, relevant and limited to what is necessary in relation to the purposes for which they are processed." The audit log records personal data (user IDs, IP addresses, accessed file paths, timestamps) for every request. Storing detailed audit trails may exceed what is strictly necessary, especially for routine read operations.

### Position B — Audit Logging (ISO 27001 A.12.4, NIST AU-2/AU-3/AU-4)

ISO 27001 A.12.4 and NIST AU-2/3/4 mandate comprehensive logging of access events for security monitoring, incident investigation, and accountability. The immutable audit log (REQ-ENT-001) records all authenticated requests including principal, action, resource, result, and client IP. Reducing log granularity would undermine the ability to detect unauthorized access, investigate breaches (GDPR Art. 33), and demonstrate compliance.

### Impact Analysis

| Dimension | Impact |
|-----------|--------|
| **Security** | Reduced logging creates blind spots for intrusion detection and breach investigation |
| **Compliance** | Both GDPR and ISO 27001/NIST are mandatory; failing either is unacceptable |
| **Storage** | Full audit logging at scale (millions of requests/day) requires significant storage |
| **Privacy** | Storing IP addresses and accessed file paths constitutes processing of personal data |

### Proposed Resolution

**Tiered logging with retention policies:**

1. **Full audit trail** (REQ-ENT-001): Log all authenticated requests with full detail. This satisfies ISO 27001 and NIST requirements and supports GDPR Art. 33 breach notification.
2. **Retention policy**: Implement configurable retention periods (default 90 days for routine logs, 1 year for security-relevant events). This satisfies GDPR data minimization over time.
3. **Access logging scope**: Log principal ID (not display name), action, resource path, result, and client IP. Do not log request/response bodies unless explicitly configured for debug mode.
4. **Data subject access**: Provide a data export endpoint (GDPR Art. 15) that allows users to retrieve their own audit trail entries.
5. **Legal basis**: Document the legitimate interest basis for audit logging under GDPR Art. 6(1)(f) — security monitoring and breach detection constitute legitimate interests that override the data subject's privacy rights to a proportionate degree.

**Decision:** Full audit logging with tiered retention and data subject access. The security and accountability benefits of comprehensive logging outweigh the marginal privacy impact when mitigated by retention limits.

---

## Conflict 2: WebDAV Locking vs. WOPI Collaborative Editing

**ADR Reference:** FERRO-ADR-002

### Position A — WebDAV Locking (RFC 4918 Class 2, RFC 3253)

WebDAV locking (REQ-WEBDAV-002) provides exclusive and shared locks that prevent concurrent modifications. RFC 4918 specifies that a lock grants the holder exclusive write access, and RFC 3253 extends this with versioning semantics. The lock model is file-centric: one lock per resource, lock holder gets exclusive write access.

### Position B — WOPI Collaborative Editing (WOPI Specification)

WOPI (REQ-WOPI-004, REQ-WOPI-005) enables real-time collaborative editing where multiple users edit the same document simultaneously. Collabora Online and OnlyOffice use operational transformation (OT) or CRDT-based conflict resolution to merge concurrent edits. The WOPI lock model is session-centric: a lock is acquired when editing starts and released when editing ends, but the lock purpose is to prevent non-WOPI clients from overwriting the file, not to prevent multiple WOPI clients from editing.

### Impact Analysis

| Dimension | Impact |
|-----------|--------|
| **Interoperability** | A WebDAV client (rclone) that acquires an exclusive lock blocks WOPI collaborative editing, and vice versa |
| **Data integrity** | If both systems independently manage locks on the same resource, lock conflicts can cause data loss or denial of service |
| **User experience** | A user editing via WOPI would be blocked by a WebDAV lock from rclone; a user syncing via rclone would be blocked by a WOPI lock |
| **Protocol compliance** | Both protocols require correct lock semantics; implementing both naively creates a deadlock-prone system |

### Proposed Resolution

**Unified lock manager with lock type precedence:**

1. **Single lock manager**: Implement one lock service that both WebDAV and WOPI use, preventing lock state divergence.
2. **Lock type hierarchy**: WOPI locks take precedence over WebDAV locks for the same resource. When a WOPI session starts, it acquires a "WOPI editing" lock that is visible to WebDAV clients as a 423 Locked response.
3. **WebDAV compatibility**: WebDAV exclusive locks prevent WOPI editing from starting (WOPI CheckFileInfo returns `SupportsCoauth=false` when a WebDAV lock exists).
4. **WOPI lock semantics**: WOPI locks are non-exclusive among WOPI clients from the same WOPI server (multiple users can co-edit), but exclusive against external WebDAV clients.
5. **Lock timeout**: Both lock types share the same timeout mechanism. WOPI locks have longer timeouts (configurable, default 30 minutes) with heartbeat refresh, while WebDAV locks use shorter timeouts (default 60 seconds for rclone compatibility).

**Decision:** Unified lock manager with WOPI precedence. WebDAV and WOPI share a single lock namespace with type-based precedence rules.

---

## Conflict 3: WASM Sandboxing vs. Plugin Performance Needs

**ADR Reference:** FERRO-ADR-003

### Position A — WASM Sandboxing (WASM/WASI Specification, OWASP A04, NIST SC-7)

REQ-WASM-003 requires strict WASI capability restrictions preventing plugins from accessing unauthorized filesystem paths, network endpoints, or environment variables. REQ-WASM-004 requires enforcing memory, CPU fuel, and wall-clock time limits. These constraints are security-critical: a malicious or buggy plugin must not be able to crash the server, exfiltrate data, or exhaust resources (OWASP A04, NIST SC-7).

### Position B — Plugin Performance Needs

WASM plugins for file processing (OCR, virus scanning, image resizing, content extraction for search indexing) may need to:
- Read the file being processed (requires filesystem access)
- Write extracted metadata or converted files (requires filesystem write access)
- Make outbound HTTP calls (e.g., webhook notifications, external API calls)
- Allocate significant memory (e.g., image decoding buffers)
- Execute for extended periods (e.g., OCR on large documents)

Overly restrictive sandboxing makes plugins impractical for their intended use cases, while overly permissive sandboxing negates the security benefits.

### Impact Analysis

| Dimension | Impact |
|-----------|--------|
| **Security** | Insufficient sandboxing allows data exfiltration, resource exhaustion (DoS), or server crashes |
| **Functionality** | Over-restrictive sandboxing prevents plugins from performing their intended tasks |
| **Performance** | WASI capability checks add per-call overhead; fuel metering adds CPU overhead |
| **Developer experience** | Complex capability configuration increases plugin development friction |

### Proposed Resolution

**Capability-based authorization with per-plugin profiles:**

1. **Explicit capability grants**: Each plugin declares required capabilities in a manifest (TOML file alongside the WASM binary). The administrator approves capabilities at plugin install time. No capabilities are granted by default.
2. **Predefined capability profiles**:
   - `file-processor`: Read-only access to the file being processed; no network; limited memory (256MB); 60s timeout
   - `converter`: Read access to input file; write access to a designated output path; no network; 512MB memory; 120s timeout
   - `webhook-notifier`: No filesystem access; outbound HTTP to a single configured URL; 32MB memory; 10s timeout
   - `virus-scanner`: Read-only access to the file being processed; no network; 1GB memory; 300s timeout
   - `custom`: Administrator specifies exact WASI capabilities
3. **Scoped filesystem access**: Filesystem capabilities are scoped to specific paths (e.g., `/tmp/ferro-plugin-{id}/` for output). Plugins cannot read arbitrary server files.
4. **Fuel metering is optional but on by default**: Administrators can disable fuel metering for trusted plugins that need maximum CPU performance, at the cost of DoS vulnerability.
5. **Plugin isolation**: Each plugin instance runs in a separate Wasmtime `Store`, preventing interference between concurrent plugin executions.

**Decision:** Capability-based authorization with predefined profiles. Plugins must declare capabilities; administrators approve at install time. Sensible defaults balance security and usability.

---

## Conflict 4: Pre-Signed URLs vs. Access Control Enforcement

**ADR Reference:** FERRO-ADR-004

### Position A — Pre-Signed URL Direct Access (REQ-STOR-004)

Pre-signed URLs (REQ-STOR-004) allow clients to upload/download directly to the cloud backend (S3/GCS/Azure), bypassing the Ferro server entirely. This provides:
- 10Gbps+ throughput (limited by client-to-cloud bandwidth, not server bandwidth)
- Reduced server CPU and memory load
- Better user experience for large file transfers

### Position B — Access Control Enforcement (REQ-AUTH-002, NIST AC-3, ISO 27001 A.8.3)

Cedar policy engine (REQ-AUTH-002) must enforce authorization on every operation. NIST AC-3 and ISO 27001 A.8.3 require that access enforcement cannot be bypassed. When a client uses a pre-signed URL to access a file directly, the Ferro server is not in the request path and cannot:
- Verify the client's current authorization status
- Audit the access (REQ-ENT-001)
- Enforce time-based or conditional policies
- Revoke access immediately (revocation requires waiting for URL expiration)

### Impact Analysis

| Dimension | Impact |
|-----------|--------|
| **Security** | Pre-signed URLs create a window where access cannot be revoked or audited in real-time |
| **Compliance** | NIST AC-3 requires continuous access enforcement; pre-signed URLs provide point-in-time enforcement |
| **Performance** | Without pre-signed URLs, all large file transfers go through Ferro, creating a bandwidth bottleneck |
| **Audit completeness** | Direct-to-cloud transfers bypass the audit log (the cloud provider logs access, but not in Ferro's structured format) |

### Proposed Resolution

**Pre-signed URLs with short TTL and asynchronous audit:**

1. **Short-lived tokens**: Pre-signed URLs have a configurable but short time-to-live (default 60 seconds, maximum 300 seconds). This minimizes the window where access cannot be revoked.
2. **Scope restriction**: Pre-signed URLs are scoped to a single object and single HTTP method (PUT or GET). They cannot be used to list buckets or access other objects.
3. **Authorization at generation time**: Cedar policy is evaluated when the pre-signed URL is generated. If the user is not authorized, the URL is not generated. This provides point-in-time enforcement.
4. **IP binding where supported**: For S3 and GCS, include the client's IP address in the pre-signed URL constraints. Azure Blob does not support IP binding natively; document this limitation.
5. **Asynchronous audit**: The cloud backend's access logs (S3 CloudTrail, GCS Audit Logs, Azure Storage Analytics) are the source of truth for pre-signed URL access. Ferro can optionally ingest these logs for unified auditing, but this is a Phase 5+ feature.
6. **Policy-based opt-in**: Pre-signed URLs are only enabled when explicitly configured by the administrator (REQ-STOR-004 is "Should" priority). Deployments with strict compliance requirements can disable pre-signed URLs entirely, forcing all traffic through Ferro.
7. **Revocation mechanism**: For emergency revocation, Ferro can invalidate the underlying cloud credentials or rotate the signing key, which invalidates all outstanding pre-signed URLs.

**Decision:** Short-lived, scoped pre-signed URLs with Cedar evaluation at generation time. Asynchronous audit via cloud provider logs. Optional feature (disabled by default for compliance-sensitive deployments).

---

## Conflict Summary

| Conflict | ADR | Severity | Resolution Approach | Default Behavior |
|----------|-----|----------|-------------------|-----------------|
| GDPR vs. Audit Logging | FERRO-ADR-001 | Medium | Tiered logging with retention policies | Full logging with 90-day retention |
| WebDAV Locking vs. WOPI Editing | FERRO-ADR-002 | High | Unified lock manager with type precedence | WOPI locks take precedence |
| WASM Sandboxing vs. Plugin Performance | FERRO-ADR-003 | High | Capability profiles with admin approval | Predefined profiles, no capabilities by default |
| Pre-Signed URLs vs. Access Control | FERRO-ADR-004 | High | Short TTL, scoped, Cedar at generation, opt-in | Disabled by default |

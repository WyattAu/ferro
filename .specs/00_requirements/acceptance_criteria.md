# Ferro Acceptance Criteria — Must-Priority Requirements

**Document ID:** FERRO-AC-001
**Date:** 2026-04-18
**Status:** Draft

---

## REQ-STOR-001 — Multi-Backend Storage Abstraction

### Given/When/Then

```gherkin
Scenario: Store and retrieve a file via Local FS backend
  Given the system is configured with a Local FS storage backend
  When a client PUTs a 10MB file to "/docs/report.pdf"
  Then the system responds with HTTP 201 Created
  And the client can GET "/docs/report.pdf" and receive identical content
  And the client receives a valid ETag header

Scenario: Store and retrieve a file via S3 backend
  Given the system is configured with an Amazon S3 storage backend
  When a client PUTs a 10MB file to "/docs/report.pdf"
  Then the system responds with HTTP 201 Created
  And the client can GET "/docs/report.pdf" and receive identical content

Scenario: Store and retrieve a file via GCS backend
  Given the system is configured with a Google Cloud Storage backend
  When a client PUTs a 10MB file to "/docs/report.pdf"
  Then the system responds with HTTP 201 Created
  And the client can GET "/docs/report.pdf" and receive identical content

Scenario: Store and retrieve a file via Azure Blob backend
  Given the system is configured with an Azure Blob Storage backend
  When a client PUTs a 10MB file to "/docs/report.pdf"
  Then the system responds with HTTP 201 Created
  And the client can GET "/docs/report.pdf" and receive identical content

Scenario: Backend unavailability does not affect other backends
  Given the system is configured with Local FS and S3 backends
  And the S3 backend is unreachable
  When a client PUTs a file to the Local FS backend
  Then the operation succeeds
  And when a client PUTs a file to the S3 backend
  Then the system responds with HTTP 503 Service Unavailable
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| PUT latency (1MB file, Local FS) | < 50ms (p95) |
| PUT latency (1MB file, S3) | < 500ms (p95) |
| GET latency (1MB file, any backend) | < 500ms (p95) |
| Backend failover time | < 5s detection |
| Data integrity | SHA-256 checksum match on every GET |

### Test Category

- Integration tests per backend (unit for backend configuration parsing)

---

## REQ-STOR-002 — Content-Addressable Storage (SHA-256)

### Given/When/Then

```gherkin
Scenario: Identical content is stored once
  Given the system is configured with a storage backend
  When a client uploads file A with content "hello world" to "/dir/file-a.txt"
  And a client uploads file B with content "hello world" to "/dir/file-b.txt"
  Then the physical storage contains exactly one copy of the content
  And GET "/dir/file-a.txt" returns "hello world"
  And GET "/dir/file-b.txt" returns "hello world"
  And both responses return the same ETag

Scenario: Different content produces different physical copies
  Given the system has stored a file with content "hello world"
  When a client uploads a file with content "goodbye world" to "/dir/file-c.txt"
  Then the physical storage contains two distinct copies
  And the ETags differ

Scenario: Partial overwrite preserves unmodified data
  Given a file "/dir/large.bin" exists with 100MB of content
  When a client uploads identical content to "/dir/large-copy.bin"
  Then no additional physical storage is consumed
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Dedup detection time (post-upload) | < 100ms for files up to 1GB |
| SHA-256 computation throughput | >= 500 MB/s |
| Storage savings (identical uploads) | >= 99% (only metadata overhead) |
| CAS lookup correctness | 100% (zero false positives/negatives) |

### Test Category

- Integration tests, property-based tests (Arbitrary byte content, verify single-copy invariant)

---

## REQ-STOR-005 — Atomic Write Operations

### Given/When/Then

```gherkin
Scenario: Failed upload does not corrupt existing file
  Given a file "/docs/important.pdf" exists with known content
  When a client begins uploading new content to "/docs/important.pdf"
  And the connection is terminated mid-upload
  Then a subsequent GET "/docs/important.pdf" returns the original content
  And no partial file exists in the staging area after cleanup

Scenario: Concurrent upload race results in one winner
  Given a file "/docs/shared.txt" exists
  When Client A uploads content "version-A" to "/docs/shared.txt"
  And Client B uploads content "version-B" to "/docs/shared.txt" concurrently
  Then GET "/docs/shared.txt" returns either "version-A" or "version-B" (not interleaved)
  And the file has a valid CAS entry
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Staging cleanup after failure | < 30s |
| Atomicity guarantee | 100% (zero partial writes in 10,000 failure injection iterations) |
| Performance overhead (atomic vs non-atomic) | < 10% latency increase |

### Test Category

- Property-based tests with chaos injection (network failure, process kill)

---

## REQ-STOR-006 — Concurrent Upload Handling

### Given/When/Then

```gherkin
Scenario: Concurrent uploads to same path resolve cleanly
  Given no file exists at "/docs/concurrent.txt"
  When 10 clients concurrently upload different content to "/docs/concurrent.txt"
  Then exactly one file exists at "/docs/concurrent.txt"
  And the CAS reference count matches the number of logical references

Scenario: Concurrent uploads to different paths all succeed
  When 100 clients concurrently upload unique files to unique paths
  Then all 100 files are retrievable
  And all CAS entries are correct
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Correctness under concurrency | 100% (zero data corruption in 1,000 concurrent upload iterations) |
| Throughput degradation with 100 concurrent uploads | < 20% vs single upload |

### Test Category

- Concurrency tests (tokio::spawn multi-client simulation)

---

## REQ-WEBDAV-001 — RFC 4918 Class 1 Compliance

### Given/When/Then

```gherkin
Scenario: Directory listing via PROPFIND
  Given a directory "/files/" containing 3 files and 2 subdirectories
  When a client sends PROPFIND with Depth: 1 to "/files/"
  Then the system responds with HTTP 207 Multi-Status
  And the response contains exactly 6 resources ("/files/" + 3 files + 2 subdirs)
  And each resource includes DAV:getcontentlength, DAV:getlastmodified, DAV:getetag

Scenario: Create directory via MKCOL
  When a client sends MKCOL to "/files/new-folder/"
  Then the system responds with HTTP 201 Created
  And PROPFIND on "/files/" includes "new-folder" in the listing

Scenario: Delete resource via DELETE
  Given a file "/files/temp.txt" exists
  When a client sends DELETE to "/files/temp.txt"
  Then the system responds with HTTP 204 No Content
  And GET "/files/temp.txt" returns HTTP 404 Not Found

Scenario: Copy resource via COPY
  Given a file "/files/original.txt" exists
  When a client sends COPY with Destination header "/files/copy.txt"
  Then the system responds with HTTP 201 Created
  And GET "/files/copy.txt" returns identical content to "/files/original.txt"

Scenario: Move resource via MOVE
  Given a file "/files/source.txt" exists
  When a client sends MOVE with Destination header "/files/dest.txt"
  Then the system responds with HTTP 201 Created
  And GET "/files/source.txt" returns HTTP 404
  And GET "/files/dest.txt" returns the original content

Scenario: Insufficient storage returns 507
  Given the storage backend has 0 bytes available
  When a client sends PUT to "/files/large.bin"
  Then the system responds with HTTP 507 Insufficient Storage
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| PROPFIND response time (1,000 items) | < 200ms (p95) |
| PROPFIND response time (10,000 items) | < 1s (p95) |
| XML response validity | 100% pass rate against RFC 4918 schema |
| All Class 1 methods | Pass rclone WebDAV test suite |

### Test Category

- Integration tests (method-by-method), end-to-end (rclone mount verification)

---

## REQ-WEBDAV-002 — RFC 4918 Class 2 Compliance (Locking)

### Given/When/Then

```gherkin
Scenario: Exclusive lock prevents modification
  Given a file "/docs/locked.txt" exists
  When Client A sends LOCK with Scope: exclusive to "/docs/locked.txt"
  Then the system responds with HTTP 200 OK with a valid Lock-Token
  And when Client B sends PUT to "/docs/locked.txt" without the lock token
  Then the system responds with HTTP 423 Locked

Scenario: Lock holder can modify
  Given Client A holds an exclusive lock on "/docs/locked.txt"
  When Client A sends PUT with the correct If header containing the lock token
  Then the system responds with HTTP 204 No Content
  And the file content is updated

Scenario: Lock timeout auto-release
  Given Client A locks "/docs/locked.txt" with a 10-second timeout
  When 11 seconds have elapsed
  And Client B sends PUT to "/docs/locked.txt"
  Then the system responds with HTTP 204 No Content (lock has expired)

Scenario: Shared lock allows multiple readers
  When Client A and Client B both acquire shared locks on "/docs/shared.txt"
  Then both clients can read the file
  And neither can modify it without upgrading to exclusive

Scenario: UNLOCK releases the lock
  Given Client A holds a lock on "/docs/locked.txt"
  When Client A sends UNLOCK with the correct Lock-Token
  Then the system responds with HTTP 204 No Content
  And Client B can now modify the file
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| LOCK response time | < 50ms (p95) |
| Lock timeout accuracy | +/- 1s of configured value |
| Lock contention resolution | < 100ms under 10 concurrent lock attempts |
| Stale lock cleanup | All expired locks cleaned within 60s |

### Test Category

- Integration tests, concurrency tests (multi-client locking scenarios)

---

## REQ-WEBDAV-004 — rclone Compatibility

### Given/When/Then

```gherkin
Scenario: Mount Ferro via rclone and perform operations
  Given the Ferro server is running with WebDAV enabled
  When rclone is configured with Ferro as a WebDAV remote
  And the user mounts the remote via "rclone mount ferro: /mnt/ferro"
  Then the mount point is accessible via the OS filesystem
  And "ls /mnt/ferro" lists the root directory contents
  And "cp localfile.txt /mnt/ferro/" uploads the file
  And "cat /mnt/ferro/localfile.txt" returns the file content
  And "rclone sync /local/dir/ ferro:/backup/" completes without errors
  And "rclone check /local/dir/ ferro:/backup/" reports all files match
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| rclone ls performance (1,000 files) | < 5s total |
| rclone copy performance (100MB) | >= 50 MB/s (limited by backend, not Ferro) |
| Zero data corruption | 100% checksum match across 10,000 file operations |
| rclone error rate | 0% (no rclone errors in standard operation suite) |

### Test Category

- End-to-end tests (rclone CLI integration)

---

## REQ-WOPI-005 — File Locking for Concurrent Editing

### Given/When/Then

```gherkin
Scenario: WOPI lock blocks WebDAV modification
  Given a file "/docs/collaborative.docx" exists
  When a WOPI client acquires a lock via the WOPI Lock endpoint
  And a WebDAV client sends PUT to "/docs/collaborative.docx" without the lock
  Then the system responds with HTTP 423 Locked

Scenario: WOPI lock holder can save via PutFile
  Given a WOPI client holds a lock on "/docs/collaborative.docx"
  When the WOPI client calls PutFile with the current lock token
  Then the file content is updated
  And the lock is maintained (not released on PutFile)

Scenario: WOPI lock expires and becomes available
  Given a WOPI client holds a lock with a 30-minute timeout
  When 31 minutes have elapsed without a lock refresh
  Then the lock is released
  And other clients can acquire the lock
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| WOPI Lock response time | < 50ms (p95) |
| Lock acquisition under contention | < 500ms (p95) |
| Zero data loss under concurrent editing | 100% (last-write-wins within lock window) |

### Test Category

- Integration tests (WOPI + WebDAV concurrent access)

---

## REQ-AUTH-001 — OpenID Connect Authentication

### Given/When/Then

```gherkin
Scenario: Complete OIDC authentication flow
  Given the system is configured with a valid OIDC provider (Keycloak)
  When a user navigates to the Ferro web interface
  Then the user is redirected to the OIDC provider's login page
  And after successful authentication, the user is redirected back with an authorization code
  And the system exchanges the code for tokens using PKCE S256
  And the user is authenticated and can access protected resources

Scenario: Invalid token is rejected
  Given a user has an expired access token
  When the user makes an API request with the expired token
  Then the system responds with HTTP 401 Unauthorized

Scenario: RP-Initiated Logout clears session
  Given a user is authenticated
  When the user clicks "Logout"
  Then the system clears the local session
  And redirects to the OIDC provider's end_session_endpoint
  And subsequent API requests with the old token are rejected

Scenario: Nonce validation prevents replay attacks
  Given the system generates a nonce for the authentication request
  When the OIDC callback includes a valid nonce
  Then the authentication succeeds
  And when a replayed callback with the same nonce is sent
  Then the system rejects it
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| OIDC flow completion time | < 2s (provider-dependent, Ferro overhead < 100ms) |
| Token validation latency | < 5ms (p95, with cached JWKS) |
| PKCE enforcement | 100% (all auth requests include S256 code_challenge) |
| Session revocation propagation | < 1s |

### Test Category

- Integration tests (against mock OIDC provider), end-to-end (against Keycloak)

---

## REQ-AUTH-002 — Cedar Policy Engine Integration

### Given/When/Then

```gherkin
Scenario: Deny-by-default for unconfigured actions
  Given no Cedar policies are configured
  When a user makes any API request
  Then the system responds with HTTP 403 Forbidden

Scenario: Policy allows specific action
  Given a Cedar policy: "permit(principal == User::\"alice\", action == Action::\"view\", resource);"
  When user "alice" sends a GET request to view a resource
  Then the system responds with HTTP 200 OK

Scenario: Policy denies specific action
  Given a Cedar policy: "permit(principal == User::\"alice\", action == Action::\"view\", resource);"
  When user "alice" sends a PUT request to modify a resource
  Then the system responds with HTTP 403 Forbidden

Scenario: Policy with conditions
  Given a Cedar policy: "permit(principal, action == Action::\"view\", resource) when { resource.owner == principal };"
  When user "alice" views a resource owned by "alice"
  Then the system responds with HTTP 200 OK
  And when user "bob" views the same resource
  Then the system responds with HTTP 403 Forbidden
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Policy evaluation latency | < 1ms (p95, single policy check) |
| Policy evaluation latency (100 policies) | < 10ms (p95) |
| Deny-by-default enforcement | 100% (no action succeeds without explicit permit) |
| Policy hot-reload time | < 100ms |

### Test Category

- Integration tests, property-based tests (random policy + request combinations)

---

## REQ-AUTH-003 — Fine-Grained Authorization Policies

### Given/When/Then

```gherkin
Scenario: Attribute-based access from OIDC claims
  Given user "alice" has OIDC claim "department" = "engineering"
  And a Cedar policy: "permit(principal, action, resource) when { principal.department == \"engineering\" };"
  When user "alice" accesses any resource
  Then access is granted

Scenario: Role-based access from OIDC groups
  Given user "alice" has OIDC group "admin"
  And a Cedar policy: "permit(principal, action == Action::\"admin\", resource) when { principal.groups.includes(\"admin\") };"
  When user "alice" performs an admin action
  Then the system responds with HTTP 200 OK

Scenario: Resource attribute conditions
  Given a resource with tag "confidential"
  And a Cedar policy: "permit(principal, action == Action::\"view\", resource) when { resource.tag != \"confidential\" };"
  When a user without "confidential" clearance views the resource
  Then the system responds with HTTP 403 Forbidden
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| OIDC claim → Cedar attribute mapping latency | < 1ms (per-request, cached after first) |
| Attribute resolution correctness | 100% (all configured claims mapped correctly) |
| Policy complexity support | At least 50 concurrent policies without performance degradation |

### Test Category

- Integration tests (OIDC claim mapping), property-based tests

---

## REQ-AUTH-004 — Session Management

### Given/When/Then

```gherkin
Scenario: Session expires after idle timeout
  Given the system is configured with an idle timeout of 15 minutes
  And user "alice" is authenticated
  When 16 minutes pass without any API requests from "alice"
  Then the session is expired
  And the next API request from "alice" receives HTTP 401 Unauthorized

Scenario: Refresh token rotation
  Given user "alice" has a valid refresh token
  When "alice" uses the refresh token to obtain new access tokens
  Then the system issues new access and refresh tokens
  And the old refresh token is invalidated
  And subsequent use of the old refresh token is rejected

Scenario: Absolute session lifetime
  Given the system is configured with a maximum session lifetime of 24 hours
  When 25 hours have elapsed since session creation (regardless of activity)
  Then the session is expired
  And all tokens are invalidated
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Session validation overhead | < 1ms per request |
| Idle timeout accuracy | +/- 30s of configured value |
| Refresh token rotation | 100% (old tokens invalidated on rotation) |
| Concurrent session support | >= 10,000 active sessions |

### Test Category

- Integration tests, time-sensitive tests (tokio time manipulation)

---

## REQ-WASM-003 — Plugin Sandboxing

### Given/When/Then

```gherkin
Scenario: Plugin cannot access unauthorized filesystem
  Given a WASM plugin is loaded with no filesystem capabilities
  When the plugin attempts to read "/etc/passwd"
  Then the WASM runtime denies the access
  And the plugin receives a WASI error
  And the Ferro server remains responsive

Scenario: Plugin cannot make network connections
  Given a WASM plugin is loaded with no HTTP capabilities
  When the plugin attempts to open a network connection
  Then the WASM runtime denies the access
  And no network traffic is emitted

Scenario: Malicious plugin cannot crash server
  Given a WASM plugin that contains a stack overflow
  When the plugin is executed
  Then the plugin is terminated with an error
  And the Ferro server continues processing other requests normally

Scenario: Plugin with explicit filesystem access is scoped
  Given a WASM plugin is loaded with filesystem access scoped to "/sandbox/"
  When the plugin reads "/sandbox/allowed.txt"
  Then the read succeeds
  And when the plugin reads "/etc/passwd"
  Then the WASM runtime denies the access
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Sandbox escape detection | 100% (no unauthorized access in penetration test) |
| Server stability under malicious plugin | 100% uptime (server never crashes) |
| Capability enforcement latency | < 1ms overhead per WASI call |

### Test Category

- Security tests (adversarial WASM plugins), integration tests

---

## REQ-WASM-004 — Resource Limits for Plugins

### Given/When/Then

```gherkin
Scenario: Plugin with infinite loop is terminated
  Given a WASM plugin containing an infinite loop
  And the system is configured with a CPU fuel limit of 1,000,000 instructions
  When the plugin is executed
  Then the plugin is terminated after the fuel limit is reached
  And an error is logged
  And the Ferro server continues operating normally

Scenario: Plugin exceeding memory limit is terminated
  Given a WASM plugin that allocates memory in a loop
  And the system is configured with a 64MB memory limit per plugin
  When the plugin attempts to allocate beyond 64MB
  Then the plugin is terminated
  And no other plugins are affected

Scenario: Plugin exceeding wall-clock timeout is terminated
  Given a WASM plugin that sleeps indefinitely
  And the system is configured with a 30-second wall-clock timeout
  When 30 seconds elapse
  Then the plugin is terminated
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Fuel limit enforcement accuracy | Within 1% of configured value |
| Memory limit enforcement | Exact (no over-allocation) |
| Wall-clock timeout accuracy | +/- 1s of configured value |
| Impact on other plugins | Zero (termination of one plugin does not affect others) |
| Server stability | 100% uptime during plugin termination |

### Test Category

- Integration tests (resource limit boundary conditions)

---

## REQ-WEB-005 — Type-Safe API via Shared Crate

### Given/When/Then

```gherkin
Scenario: Shared types prevent API contract drift
  Given the ferro-common crate defines a FileMetadata struct with fields: name, size, modified, etag
  When the server returns a FileMetadata response
  Then the Leptos frontend can deserialize it without type conversion
  And modifying FileMetadata in ferro-common causes both server and frontend compilation to fail until both are updated

Scenario: API response types match server implementation
  Given the ferro-common crate defines all API endpoint request/response types
  When a new endpoint is added to the server
  And the corresponding type is not added to ferro-common
  Then the server fails to compile
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Type safety guarantee | 100% compile-time (zero runtime type mismatches possible) |
| Build coupling | Both server and frontend must be rebuilt when shared types change |
| API documentation accuracy | Types serve as single source of truth for API contracts |

### Test Category

- Build tests (compile-time verification)

---

## REQ-DESK-003 — Zero-Config Mount

### Given/When/Then

```gherkin
Scenario: User authenticates and drive mounts automatically
  Given the Tauri desktop client is installed
  And the user has valid OIDC credentials
  When the user launches the desktop client and authenticates
  Then the Ferro drive is mounted as a local filesystem (e.g., "Z:" on Windows, "/Volumes/Ferro" on macOS)
  Without any manual rclone configuration
  And the drive is visible in the OS file manager

Scenario: File operations through mounted drive
  Given the Ferro drive is mounted
  When the user creates a file via the OS file manager in the mounted drive
  Then the file appears in the Ferro WebDAV listing
  And when the user deletes a file via the mounted drive
  Then the file is removed from Ferro

Scenario: Mount survives application restart
  Given the Ferro drive is mounted
  When the user closes and reopens the desktop client
  Then the Ferro drive is automatically remounted
  Without re-authentication (within session lifetime)
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Time from authentication to mounted drive | < 10s |
| File operations through mount | Zero data corruption (checksum verified) |
| Mount survival across restart | 100% (automatic re-mount on app restart) |
| Zero manual configuration steps | 0 (user only provides OIDC credentials) |

### Test Category

- End-to-end tests (OS-level file operations on mounted drive)

---

## REQ-ENT-001 — Immutable Audit Log

### Given/When/Then

```gherkin
Scenario: All authenticated operations are logged
  Given a user is authenticated
  When the user performs a GET, PUT, DELETE, PROPFIND, MKCOL, LOCK, and UNLOCK operation
  Then each operation produces an audit log entry containing: timestamp, principal_id, action, resource_path, result (success/failure), client_ip

Scenario: Audit log is append-only
  Given audit log entries exist
  When an attempt is made to modify or delete a log entry
  Then the operation is rejected
  And the tamper-evident hash chain detects the modification attempt

Scenario: Hash chain integrity verification
  Given 1,000 audit log entries
  When the hash chain is verified from the first entry to the last
  Then all hashes are valid and contiguous
  And modifying any single entry causes the chain verification to fail

Scenario: Log query by principal
  Given multiple users have performed operations
  When the audit log is queried for a specific principal
  Then only entries for that principal are returned
  And the results are ordered by timestamp
```

### Measurable Success Criteria

| Metric | Target |
|--------|--------|
| Audit log write latency overhead | < 5ms per request |
| Log query performance (1M entries by principal) | < 500ms |
| Hash chain verification (1M entries) | < 2s |
| Tamper detection | 100% (any modification detected) |
| Log write failure handling | Fail-closed (reject operation if log write fails) |

### Test Category

- Integration tests, property-based tests (hash chain integrity under concurrent writes)

---
id: YP-WEBDAV-PROTOCOL-001
version: 1.0.0
status: draft
domain: webdav
title: "WebDAV Protocol Yellow Paper"
date: 2026-04-18
author: DeepThought (Research Agent)
applicable_standards:
  - RFC 4918
  - RFC 3253
  - RFC 3744
  - RFC 2616 (HTTP/1.1)
  - RFC 3986 (URI Generic Syntax)
  - RFC 2119 (Key Words)
  - RFC 5689 (BIND)
---

# YP-WEBDAV-PROTOCOL-001: WebDAV Protocol Specification

## YP-1: Document Header

| Field | Value |
|---|---|
| Document ID | YP-WEBDAV-PROTOCOL-001 |
| Version | 1.0.0 |
| Status | Draft |
| Domain | WebDAV Server Implementation |
| Primary RFCs | RFC 4918, RFC 3253, RFC 3744 |
| Target Platform | Rust (axum/hyper), high-performance async I/O |
| Compatibility Targets | rclone client, Microsoft Office suite |

## YP-2: Executive Summary

### Problem Statement

WebDAV (RFC 4918) is an HTTP extension for distributed authoring and versioning. Despite being a standards-track protocol since 1999, it presents significant implementation challenges:

1. **XML complexity**: All request/response bodies use XML with namespace-qualified elements in the `DAV:` namespace. Edge cases in XML processing (empty elements, mixed content, entity handling) are poorly documented and inconsistently implemented across servers.
2. **Multi-status responses**: The 207 Multi-Status response aggregates per-resource status codes, requiring careful error propagation semantics.
3. **Locking edge cases**: Lock-null resources, depth-infinity locks, lock token management, and timeout handling introduce stateful complexity not present in stateless HTTP.
4. **Client compatibility**: Real-world clients (rclone, Microsoft Office, macOS Finder, Windows Explorer) implement varying subsets and make undocumented assumptions.

### Scope

- **Class 1 compliance**: PROPFIND, PROPPATCH, MKCOL, COPY, MOVE (mandatory methods per RFC 4918 Section 18.1)
- **Class 2 compliance**: LOCK, UNLOCK with exclusive write locks (mandatory for locking, Section 18.2)
- **Class 3 compliance**: Access control via ACL method (RFC 3744, Section 18.3)
- **Versioning awareness**: VERSION-CONTROL, REPORT, CHECKOUT, CHECKIN (RFC 3253 — server-side pass-through)
- **rclone compatibility**: PROPFIND with Depth: 1 (default), vendor-specific quirks (Nextcloud chunking, SharePoint zero-depth fallback, ownCloud checksums)
- **Microsoft Office compatibility**: Exclusive locking, If header evaluation, OPTIONS discovery, lock refresh

### Out of Scope

- CalDAV (RFC 4791) — calendar access
- CardDAV (RFC 6352) — contact management
- WebDAV SEARCH (draft-reschke-webdav-search) — structured queries
- WebDAV redirect reference resources (RFC 4437)

## YP-3: Nomenclature and Notation

### 3.1 Core Terms

| Term | Definition | Source |
|---|---|---|
| **Resource** | Any addressable entity in the HTTP URL namespace; identified by a URI | RFC 4918 §3 |
| **Collection** | A resource that acts as a container, mapping path segments to child resources | RFC 4918 §5 |
| **Internal Member** | A child resource directly referenced by a path segment mapping in a collection | RFC 4918 §3 |
| **Member URL** | URL of any descendant (direct or recursive) of a collection | RFC 4918 §3 |
| **Property** | A name/value pair describing resource state; value is always a well-formed XML fragment | RFC 4918 §4 |
| **Live Property** | Property with semantics enforced by the server (e.g., `DAV:getcontentlength`) | RFC 4918 §4.1 |
| **Dead Property** | Property stored verbatim by the server; client is responsible for consistency | RFC 4918 §4.1 |
| **Lock Token** | A URI (typically `opaquelocktoken:` scheme) identifying a specific lock instance | RFC 4918 §6.5 |
| **State Token** | A URI representing a state of a resource; lock tokens are the only defined state tokens | RFC 4918 §3 |
| **Principal** | A distinct human or computational actor; represented as an HTTP resource | RFC 3744 §1.1 |
| **Privilege** | Controls access to a particular set of HTTP operations on a resource | RFC 3744 §1.1 |
| **ACE** | Access Control Entry; grants or denies privileges for a principal | RFC 3744 §1.1 |
| **ACL** | Access Control List; ordered list of ACEs defining access to a resource | RFC 3744 §1.1 |

### 3.2 HTTP Headers

| Header | Purpose | Methods |
|---|---|---|
| `Depth` | Controls recursion depth for PROPFIND, COPY, MOVE, LOCK, DELETE | PROPFIND, COPY, MOVE, LOCK, DELETE |
| `Destination` | Target URI for COPY/MOVE operations | COPY, MOVE |
| `Overwrite` | Controls whether COPY/MOVE may overwrite existing destination (`T`/`F`) | COPY, MOVE |
| `If` | Precondition evaluation using state tokens and ETags | Any method |
| `Lock-Token` | Submit lock token for UNLOCK | UNLOCK |
| `Timeout` | Requested lock timeout (e.g., `Second-60`, `Infinite`) | LOCK |
| `DAV` | Server capability advertisement (comma-separated compliance classes) | OPTIONS response |

### 3.3 Depth Header Values

| Value | Semantics |
|---|---|
| `0` | Apply only to the request-URL resource |
| `1` | Apply to the request-URL resource and its immediate children |
| `infinity` | Apply recursively to all descendants |

### 3.4 Status Codes

| Code | Name | Usage |
|---|---|---|
| 207 | Multi-Status | PROPFIND, PROPPATCH, COPY, MOVE with multiple resource results |
| 422 | Unprocessable Entity | Request body XML is well-formed but semantically invalid |
| 423 | Locked | Resource is locked and request lacks valid lock token |
| 424 | Failed Dependency | Action failed due to failure of a preceding action |
| 507 | Insufficient Storage | Server cannot store the representation (quota exceeded) |

### 3.5 XML Element Notation

Elements are referenced with `DAV:` prefix notation (e.g., `DAV:propfind`, `DAV:propstat`). The XML namespace URI for the DAV: prefix is `DAV:` (literal).

## YP-4: Theoretical Foundation

### 4.1 PROPFIND Method

#### 4.1.1 Formal Specification

```
PROPFIND(Request-URL) -> 207 Multi-Status
  Input:
    - Request-URL: URI of target resource
    - Depth: "0" | "1" | "infinity" (default: "infinity" per RFC, but clients commonly send "1")
    - Body: propfind XML element containing one of:
      (a) <prop> — named property request
      (b) <allprop> — all properties (MAY include <include> for extra properties)
      (c) <propname> — property names only (no values)
  Output:
    - Status: 207 Multi-Status
    - Body: multistatus element containing N response elements (one per resource)
    - Each response: href + propstat (prop + status + responsedescription?)
  Errors:
    - 207 with internal 404: property not found
    - 422: malformed request body
    - 403: insufficient privileges (RFC 3744)
    - 404: resource does not exist
```

#### 4.1.2 XML Request Schema

```xml
<!-- Named properties request -->
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:displayname/>
    <D:getlastmodified/>
    <D:getcontentlength/>
    <D:resourcetype/>
    <D:getetag/>
  </D:prop>
</D:propfind>

<!-- All properties request -->
<D:propfind xmlns:D="DAV:">
  <D:allprop/>
</D:propfind>

<!-- Property names request -->
<D:propfind xmlns:D="DAV:">
  <D:propname/>
</D:propfind>
```

#### 4.1.3 XML Response Schema

```xml
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/collection/resource.txt</D:href>
    <D:propstat>
      <D:prop>
        <D:displayname>resource.txt</D:displayname>
        <D:getlastmodified>Wed, 01 Jan 2026 00:00:00 GMT</D:getlastmodified>
        <D:getcontentlength>1024</D:getcontentlength>
        <D:resourcetype/>
        <D:getetag>"abc123"</D:getetag>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>
```

#### 4.1.4 Complexity Analysis

| Depth | Resources | Time | Space |
|---|---|---|---|
| 0 | 1 | O(P) | O(P) |
| 1 | 1 + N (N = immediate children) | O(N*P) | O(N*P) |
| infinity | total descendants D | O(D*P) | O(D*P) streaming |

Where P = number of requested properties per resource.

**Critical constraint**: Depth:infinity MUST use streaming XML generation. For 10K items with 5 properties each, DOM-based generation would require ~50MB of in-memory XML. Streaming reduces this to O(1) per-response-element memory.

### 4.2 PROPPATCH Method

#### 4.2.1 Formal Specification

```
PROPPATCH(Request-URL) -> 207 Multi-Status
  Input:
    - Request-URL: URI of target resource
    - Depth: "0" (implicit; PROPPATCH only applies to the Request-URL)
    - Body: propertyupdate element containing:
      - <set>/<prop> — properties to set/replace
      - <remove>/<prop> — properties to remove
  Output:
    - Status: 207 Multi-Status
    - Body: multistatus with propstat elements showing per-property success/failure
  Errors:
    - 409 Conflict: missing intermediate collection
    - 423 Locked: resource locked without correct token
    - 424 Failed Dependency: one property operation failed, dependent ones not attempted
```

#### 4.2.2 XML Request Schema

```xml
<D:propertyupdate xmlns:D="DAV:">
  <D:set>
    <D:prop>
      <D:displayname>New Name</D:displayname>
    </D:prop>
  </D:set>
  <D:remove>
    <D:prop>
      <D:custom-prop xmlns:ex="http://example.com/ns"/>
    </D:prop>
  </D:remove>
</D:propertyupdate>
```

#### 4.2.3 Error Semantics

Per RFC 4918 §9.2, the server MUST process `set` instructions before `remove` instructions, and within each group, process in document order. If a `set` fails, subsequent instructions MAY still be attempted. A `remove` of a non-existent property is NOT an error (MUST return 200 OK).

### 4.3 MKCOL Method

#### 4.3.1 Formal Specification

```
MKCOL(Request-URL) -> 201 Created | 405 Method Not Allowed | 409 Conflict | 415 | 507
  Input:
    - Request-URL: URI of collection to create
    - Body: MUST be empty (or server MUST reject with 415)
  Output:
    - 201 Created: collection created successfully
    - 405: resource already exists and is NOT a collection
    - 409: parent collection does not exist
    - 415: request body non-empty and unsupported
    - 507: insufficient storage
  Note: If collection already exists, return 405 (not 301 redirect)
```

### 4.4 COPY Method

#### 4.4.1 Formal Specification

```
COPY(Request-URL, Destination, Overwrite, Depth) -> 201 | 204 | 207
  Input:
    - Request-URL: source URI
    - Destination: target URI (header)
    - Overwrite: "T" (default) | "F"
    - Depth: "0" | "infinity" (default for non-collections: 0; for collections: infinity)
  Output:
    - 201 Created: destination did not exist (new resource created)
    - 204 No Content: destination existed and was overwritten
    - 207 Multi-Status: error occurred during recursive copy
    - 412 Precondition Failed: Overwrite:F but destination exists
    - 423 Locked: source or destination locked
    - 507: insufficient storage during copy
```

#### 4.4.2 Property Copy Semantics

Per RFC 4918 §9.8.2, the COPY method copies:
- All dead properties (verbatim)
- All live properties (recalculated by server for destination)
- NOT the DAV:lockdiscovery property (locks do not transfer)

#### 4.4.3 RFC 3253 Clarification on Overwrite:T

When Overwrite:T and a destination resource exists with the same resourcetype as the corresponding source, the destination MUST be **updated** (not deleted and recreated). This preserves version history. If resourcetypes differ, the destination MUST be deleted first.

### 4.5 MOVE Method

#### 4.5.1 Formal Specification

```
MOVE(Request-URL, Destination, Overwrite) -> 201 | 204 | 207
  Input:
    - Request-URL: source URI
    - Destination: target URI (header)
    - Overwrite: "T" (default) | "F"
  Output:
    - 201 Created: destination did not exist
    - 204 No Content: destination existed and was overwritten
    - 207 Multi-Status: error during move
    - 412: Overwrite:F but destination exists
    - 423: source or destination locked
    - 424: failed dependency (e.g., can't delete source after copy)
```

**Key difference from COPY**: MOVE is atomic — if the DELETE of the source fails after a successful COPY, the server MUST undo the COPY (return 424). Depth header is not used for MOVE (always depth:infinity implicitly).

### 4.6 LOCK Method

#### 4.6.1 Formal Specification

```
LOCK(Request-URL, Depth, Timeout, Body) -> 200 OK | 201 Created
  Input (creation):
    - Request-URL: URI of resource to lock (or unmapped URL for lock-null resource)
    - Depth: "0" | "infinity"
    - Timeout: "Second-N" | "Infinite"
    - Body: lockinfo element with:
      - locktype: "write"
      - lockscope: "exclusive" | "shared"
      - owner: XML fragment identifying the lock owner
  Input (refresh):
    - Request-URL: URI of locked resource
    - Lock-Token header: token of lock to refresh
    - Timeout: new requested timeout
    - Body: MUST be empty
  Output:
    - 200 OK: lock refreshed (existing resource)
    - 201 Created: lock created (possibly lock-null resource)
    - Body: prop element containing lockdiscovery with activelock
  Errors:
    - 412: If header condition failed
    - 423: resource already locked by another principal (conflict)
    - 424: failed dependency
```

#### 4.6.2 Lock Compatibility Table

| Existing \ Requested | Exclusive | Shared |
|---|---|---|
| **None** | OK | OK |
| **Exclusive** | CONFLICT | CONFLICT |
| **Shared** | CONFLICT | OK |

#### 4.6.3 Lock-Null Resources

A lock-null resource is created when LOCK is applied to an unmapped URL. It behaves as a regular locked resource but is invisible to PROPFIND (except to the lock owner). If the lock is removed (via UNLOCK or timeout) before the resource is created (via PUT), the lock-null resource is deleted.

**Critical for Office compatibility**: Microsoft Office relies on lock-null resource behavior when opening documents for editing. It LOCKs a URL, then PUTs content to it.

#### 4.6.4 Lock Token Format

Per RFC 4918 Appendix C, lock tokens use the `opaquelocktoken` URI scheme:

```
opaquelocktoken:<hex-encoded-UUID>
```

Example: `opaquelocktoken:e71d4fae-5dec-22d6-fea5-00a0c91e6be4`

### 4.7 UNLOCK Method

#### 4.7.1 Formal Specification

```
UNLOCK(Request-URL, Lock-Token) -> 204 No Content | 400 | 403 | 409
  Input:
    - Request-URL: URI of locked resource
    - Lock-Token: header containing the lock token URI in angle brackets
  Output:
    - 204 No Content: lock successfully removed
    - 400: Lock-Token header missing or malformed
    - 403: principal does not have permission to remove lock (RFC 3744 DAV:unlock privilege)
    - 409: resource exists but lock does not exist on it
```

### 4.8 OPTIONS Method

#### 4.8.1 WebDAV-Specific Response

```
OPTIONS(Request-URL) -> 200 OK
  Response Headers:
    - DAV: 1, 2, 3               (compliance classes)
    - DAV: <extension-token>     (e.g., "version-control", "access-control")
    - Allow: GET, HEAD, PUT, DELETE, PROPFIND, PROPPATCH, MKCOL, COPY, MOVE, LOCK, UNLOCK, OPTIONS
```

**Office compatibility**: Microsoft Office sends OPTIONS before attempting LOCK to verify the server supports WebDAV. The `DAV` header MUST include `1,2` for Class 1 and Class 2 compliance. The `Allow` header MUST list all supported methods.

## YP-5: Algorithm Specification

### ALG-PROPFIND-001: Recursive Property Enumeration

#### Problem
Enumerate properties for all resources under a collection with given depth, producing a streaming XML multistatus response.

#### Pseudocode

```
function PROPFIND(request_url, depth, prop_request):
    responses = async_stream()
    
    resource = resolve(request_url)
    if resource == null:
        return Error(404)
    
    await responses.emit(enumerate_properties(resource, prop_request))
    
    if depth == "infinity" or depth == "1":
        if resource.is_collection:
            children = await list_children(resource)
            for child in children:
                await responses.emit(enumerate_properties(child, prop_request))
                if depth == "infinity" and child.is_collection:
                    await responses.emit_all(
                        recurse_properties(child, prop_request, depth_limit=100)
                    )
    
    return MultiStatusResponse(207, stream=responses)

function enumerate_properties(resource, prop_request):
    href = canonical_url(resource)
    prop_results = []
    
    match prop_request:
        case Prop(names):
            for name in names:
                value = resource.get_property(name)
                if value != null:
                    prop_results.append(PropStat(name, value, 200))
                else:
                    prop_results.append(PropStat(name, null, 404))
        
        case AllProp(include?):
            for (name, value) in resource.all_properties():
                prop_results.append(PropStat(name, value, 200))
            if include:
                for name in include:
                    value = resource.get_property(name)
                    prop_results.append(PropStat(name, value, value ? 200 : 404))
        
        case PropName:
            for name in resource.all_property_names():
                prop_results.append(PropStat(name, null, 200))
    
    return Response(href, prop_results)

function recurse_properties(collection, prop_request, depth_limit, current_depth=1):
    if current_depth >= depth_limit:
        return empty_stream()  // prevent infinite recursion on circular symlinks
    
    children = await list_children(collection)
    for child in children:
        yield enumerate_properties(child, prop_request)
        if child.is_collection:
            yield_all recurse_properties(
                child, prop_request, depth_limit, current_depth + 1
            )
```

#### Complexity

- **Time**: O(D * P) where D = total descendants, P = properties per resource
- **Space**: O(1) per response element (streaming); O(max(P)) for property resolution
- **I/O**: O(D) filesystem stat calls for live properties

#### Correctness Argument

1. **Completeness**: The algorithm visits every descendant reachable via the collection's path segment mappings. By RFC 4918 §5.2, a collection's members are exactly its internal members plus the recursive members of those internal members that are themselves collections.
2. **Depth semantics**: For Depth:1, only immediate children are enumerated (single level of recursion). For Depth:infinity, recursion continues until no more collection members are found or depth_limit is reached.
3. **Property correctness**: Live properties are computed at query time from filesystem metadata; dead properties are retrieved from the property store. The 404 status for missing properties correctly indicates the property does not exist on that resource.

### ALG-LOCK-001: Exclusive/Shared Lock Management

#### Problem
Manage lock acquisition, refresh, and release with exclusive/shared semantics, timeout tracking, and lock-null resource lifecycle.

#### Data Structures

```
struct LockEntry {
    token: URI,                    // opaquelocktoken:<uuid>
    root: URI,                     // locked resource URL
    depth: Depth,                  // "0" or "infinity"
    scope: LockScope,              // "exclusive" | "shared"
    type: LockType,                // "write"
    owner: XMLFragment,            // owner element from lockinfo
    timeout: Instant,              // absolute expiry time
    created: Instant,              // lock creation time
    is_lock_null: bool,            // true if resource was unmapped at lock time
}

struct LockTable {
    locks: Map<URI, Vec<LockEntry>>,  // resource URL -> active locks
    token_index: Map<URI, LockEntry>, // token -> lock entry (fast lookup)
}
```

#### Pseudocode

```
function ACQUIRE_LOCK(request_url, lockinfo, timeout, principal):
    resource = resolve(request_url)
    is_lock_null = (resource == null)
    
    if not is_lock_null:
        check_permission(principal, "write", resource)
    
    existing_locks = lock_table.get(request_url) || []
    
    // Check lock compatibility
    for existing in existing_locks:
        if not expired(existing):
            if lockinfo.scope == "exclusive" or existing.scope == "exclusive":
                if existing.owner != principal:
                    return Error(423, "Locked")
    
    // Check depth:infinity compatibility
    if lockinfo.depth == "infinity" and resource.is_collection:
        for descendant in all_descendants(resource):
            desc_locks = lock_table.get(descendant.url) || []
            for existing in desc_locks:
                if not expired(existing) and existing.owner != principal:
                    return Error(423, "Locked")
    
    token = generate_opaquelocktoken()
    entry = LockEntry {
        token, root: request_url, depth: lockinfo.depth,
        scope: lockinfo.scope, type: "write",
        owner: lockinfo.owner, timeout: now() + timeout,
        created: now(), is_lock_null
    }
    
    lock_table.insert(request_url, entry)
    
    if is_lock_null:
        create_lock_null_resource(request_url, entry)
    
    status = is_lock_null ? 201 : 200
    return LockResponse(status, entry)

function REFRESH_LOCK(request_url, token, timeout, principal):
    entry = lock_table.token_index.get(token)
    if entry == null:
        return Error(409, "Lock does not exist")
    if entry.owner != principal:
        return Error(403, "Not lock owner")
    if expired(entry):
        lock_table.remove(entry)
        return Error(409, "Lock has expired")
    
    entry.timeout = now() + timeout
    return LockResponse(200, entry)

function RELEASE_LOCK(request_url, token, principal):
    entry = lock_table.token_index.get(token)
    if entry == null:
        return Error(409, "Lock does not exist")
    
    // RFC 3744: lock owner can always remove; others need DAV:unlock privilege
    if entry.owner != principal:
        if not has_privilege(principal, "unlock", request_url):
            return Error(403)
    
    lock_table.remove(entry)
    
    if entry.is_lock_null:
        delete_lock_null_resource(request_url)
    
    return Response(204)

function CHECK_LOCK(resource_url, method, principal):
    // Check if any active lock blocks the requested operation
    for lock in lock_table.get(resource_url) || []:
        if expired(lock): continue
        if lock.owner == principal: continue  // owner always passes
        
        // Write operations blocked by any lock; read operations pass
        if method in {"PUT", "DELETE", "PROPPATCH", "MKCOL"}:
            return Error(423, "Locked")
    
    // Check parent locks (depth:infinity)
    parent = parent_collection(resource_url)
    while parent != null:
        for lock in lock_table.get(parent.url) || []:
            if expired(lock) or lock.owner == principal: continue
            if lock.depth == "infinity":
                if method in {"PUT", "DELETE", "PROPPATCH"}:
                    return Error(423, "Locked")
        parent = parent_collection(parent.url)
    
    return OK
```

#### Complexity

- **Acquire**: O(L + D) where L = existing locks on resource, D = descendants for depth:infinity
- **Refresh**: O(1) via token index
- **Release**: O(1) via token index
- **Check**: O(L_p) where L_p = locks on parent chain (bounded by URL depth, typically < 20)

#### Correctness Argument

1. **Mutual exclusion**: The compatibility table (§4.6.2) ensures exclusive locks never coexist on the same resource, and shared locks never coexist with exclusive locks.
2. **Depth inheritance**: A depth:infinity lock on a collection blocks operations on all descendants. The CHECK_LOCK function traverses the parent chain to verify this.
3. **Timeout correctness**: Expired locks are silently removed during check/acquire/refresh, ensuring stale locks don't permanently block access.
4. **Lock-null lifecycle**: Lock-null resources are created on LOCK of unmapped URLs and deleted on UNLOCK/timeout if never materialized via PUT.

### ALG-COPY-001: Recursive Copy with Overwrite Semantics

#### Pseudocode

```
function COPY(source_url, dest_url, overwrite, depth):
    source = resolve(source_url)
    if source == null: return Error(404)
    dest = resolve(dest_url)
    
    if dest != null and overwrite == "F":
        return Error(412, "Precondition Failed")
    
    if source.is_collection:
        if depth == "0":
            // Copy collection properties only, not members
            return copy_collection_shallow(source, dest_url, overwrite)
        else:
            return copy_collection_deep(source, dest_url, overwrite)
    else:
        return copy_resource(source, dest_url, overwrite)

function copy_resource(source, dest_url, overwrite):
    dest = resolve(dest_url)
    
    if dest != null and dest.is_collection:
        return Error(422, "Cannot overwrite collection with non-collection")
    
    // Per RFC 3253 §1.7: update if same resourcetype, delete+create if different
    if dest != null and dest.resourcetype == source.resourcetype:
        dest.content = await source.read_content()
        dest.dead_properties = deep_copy(source.dead_properties)
        return Response(204)
    elif dest != null:
        await delete(dest_url, depth="infinity")
    
    new_resource = create(dest_url)
    new_resource.content = await source.read_content()
    new_resource.dead_properties = deep_copy(source.dead_properties)
    return Response(201)

function copy_collection_deep(source, dest_url, overwrite):
    dest = resolve(dest_url)
    
    if dest != null and not dest.is_collection:
        if overwrite == "T":
            await delete(dest_url, depth="infinity")
        else:
            return Error(412)
    
    if dest == null:
        dest = create_collection(dest_url)
        status = 201
    else:
        status = 204
    
    // Copy collection dead properties
    dest.dead_properties = deep_copy(source.dead_properties)
    
    errors = []
    for child in await list_children(source):
        child_dest = dest_url + "/" + basename(child.url)
        try:
            if child.is_collection:
                copy_collection_deep(child, child_dest, "T")
            else:
                copy_resource(child, child_dest, "T")
        catch e:
            errors.append(Response(child.url, Error(e)))
    
    if not errors.empty:
        return MultiStatusResponse(207, errors)
    return Response(status)
```

#### Complexity

- **Non-collection copy**: O(S) where S = size of resource content
- **Collection copy (shallow)**: O(P) where P = number of properties
- **Collection copy (deep)**: O(N*S_avg + N*P) where N = total descendants, S_avg = average content size

## YP-6: Test Vector Specification

Test vectors are defined in `test_vectors/test_vectors_webdav.toml`. The following categories are covered:

### 6.1 Nominal Cases

| ID | Method | Description |
|---|---|---|
| TV-PROP-001 | PROPFIND | Empty collection, Depth:0 |
| TV-PROP-002 | PROPFIND | Collection with 3 children, Depth:1 |
| TV-PROP-003 | PROPFIND | Nested collection, Depth:infinity |
| TV-PROP-004 | PROPPATCH | Set and remove properties |
| TV-MKCOL-001 | MKCOL | Create new collection |
| TV-COPY-001 | COPY | Copy file to new location |
| TV-COPY-002 | COPY | Copy collection with overwrite |
| TV-MOVE-001 | MOVE | Move file between collections |
| TV-LOCK-001 | LOCK/UNLOCK | Full lock lifecycle |
| TV-LOCK-002 | LOCK | Lock refresh |

### 6.2 Boundary Cases

| ID | Method | Description |
|---|---|---|
| TV-PROP-005 | PROPFIND | Propname on resource with no custom properties |
| TV-COPY-003 | COPY | Copy to existing destination, Overwrite:F |
| TV-MOVE-002 | MOVE | Move to self (should fail) |
| TV-LOCK-003 | LOCK | Lock with Timeout:Infinite |
| TV-LOCK-004 | LOCK | Lock-null resource lifecycle |

### 6.3 Adversarial Cases

| ID | Method | Description |
|---|---|---|
| TV-LOCK-005 | LOCK | Concurrent exclusive lock conflict |
| TV-LOCK-006 | LOCK | Shared locks from different principals |
| TV-PROP-006 | PROPFIND | Depth:infinity with circular symlinks |
| TV-PROP-007 | PROPFIND | Malformed XML request body |
| TV-COPY-004 | COPY | Copy collection to nested descendant (should fail) |

### 6.4 rclone-Specific Cases

| ID | Method | Description |
|---|---|---|
| TV-RCLONE-001 | PROPFIND | Depth:1 with ownCloud checksum extension |
| TV-RCLONE-002 | PROPFIND | Depth:0 retry on SharePoint (404 → Depth:0) |
| TV-RCLONE-003 | PROPFIND | Microsoft IsCollection extension parsing |
| TV-RCLONE-004 | PUT | Chunked upload via Nextcloud TUS protocol |

## YP-7: Domain Constraints

### 7.1 Timing Constraints

| Constraint | Value | Rationale |
|---|---|---|
| PROPFIND response (10K items) | < 50ms p99 | rclone sync performance; streaming XML |
| PROPFIND response (100K items) | < 500ms p99 | Large enterprise collections |
| LOCK acquisition | < 10ms p99 | Office interactivity requirement |
| MKCOL | < 20ms p99 | Directory creation during sync |
| COPY (file, < 100MB) | < 2s p99 | Office save-as behavior |

### 7.2 XML Processing Constraints

| Constraint | Value | Rationale |
|---|---|---|
| Parser type | Streaming (SAX/pull) | Memory safety for large collections |
| Max response body | Unlimited (streaming) | Must not buffer entire multistatus |
| Max request body | 1 MiB | Prevent XML bomb DoS (RFC 4918 §20.6) |
| Entity expansion | Disabled | Billion Laughs attack prevention |
| Property count per resource | No hard limit | Live properties + dead properties |

### 7.3 Locking Constraints

| Constraint | Value | Rationale |
|---|---|---|
| Default lock timeout | 60 seconds | Office compatibility (Office uses 60-120s) |
| Maximum lock timeout | 3600 seconds (1 hour) | Prevent indefinite resource starvation |
| Minimum lock timeout | 5 seconds | Prevent hammering lock refresh |
| Max concurrent locks per principal | 1000 | Resource starvation prevention |
| Lock-null resource timeout | Same as lock timeout | Tied to lock lifecycle |
| Lock token format | `opaquelocktoken:` UUID v4 | RFC 4918 Appendix C |

### 7.4 Namespace Constraints

| Constraint | Value | Rationale |
|---|---|---|
| Max path depth | 100 levels | Prevent stack overflow in recursive operations |
| Max path length | 4096 bytes | OS filesystem limits |
| Collection URL | MUST end with `/` | RFC 4918 §5.2 convention |
| Case sensitivity | Configurable | Windows (case-insensitive) vs Unix (case-sensitive) |

### 7.5 Compatibility Constraints

| Constraint | Value | Rationale |
|---|---|---|
| DAV header in OPTIONS | `1, 2` | Office requires Class 2 (locking) |
| Allow header | Full method list | Client capability discovery |
| Content-Type for XML | `text/xml; charset="utf-8"` | rclone and Office expect this |
| PROPFIND empty body | Default to `allprop` | RFC 4918 §9.1; rclone sends empty body |
| Collection trailing slash | MUST redirect or auto-append | Office opens collections without trailing `/` |
| Status line format | `HTTP/1.1 NNN Description` | rclone regex parses this format |

### 7.6 Security Constraints

| Constraint | Value | Rationale |
|---|---|---|
| XML entity expansion | Disabled (RFC 4918 §20.6) | Billion Laughs / XML bomb prevention |
| Path traversal | Reject `..` segments | Prevent access outside root |
| Lock token entropy | 128 bits (UUID v4) | Prevent token guessing |
| Rate limiting | Configurable per-IP | DoS prevention (RFC 4918 §20.2) |

## YP-8: Bibliography

### Normative References

| Ref | Citation |
|---|---|
| [RFC 4918] | Dusseault, L., Ed., "HTTP Extensions for Web Distributed Authoring and Versioning (WebDAV)", RFC 4918, June 2007. https://www.rfc-editor.org/rfc/rfc4918 |
| [RFC 3253] | Clemm, G., Amsden, J., Ellison, T., Kaler, C., Whitehead, J., "Versioning Extensions to WebDAV", RFC 3253, March 2002. https://www.rfc-editor.org/rfc/rfc3253 |
| [RFC 3744] | Clemm, G., Reschke, J., Sedlar, E., Whitehead, J., "WebDAV Access Control Protocol", RFC 3744, May 2004. https://www.rfc-editor.org/rfc/rfc3744 |
| [RFC 2616] | Fielding, R., et al., "Hypertext Transfer Protocol -- HTTP/1.1", RFC 2616, June 1999. https://www.rfc-editor.org/rfc/rfc2616 |
| [RFC 3986] | Berners-Lee, T., et al., "Uniform Resource Identifier (URI): Generic Syntax", RFC 3986, January 2005. https://www.rfc-editor.org/rfc/rfc3986 |
| [RFC 2119] | Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997. https://www.rfc-editor.org/rfc/rfc2119 |
| [RFC 5689] | Reschke, J., "Binding Extensions to Web Distributed Authoring and Versioning (WebDAV)", RFC 5689, September 2009. https://www.rfc-editor.org/rfc/rfc5689 |

### Informative References

| Ref | Citation |
|---|---|
| [RFC 2518] | Goland, Y., et al., "HTTP Extensions for Distributed Authoring -- WEBDAV", RFC 2518, February 1999. (Obsoleted by RFC 4918) |
| [RFC 2291] | Kaler, C., "Requirements for a Distributed Authoring and Versioning Protocol for the World Wide Web", RFC 2291, February 1998. |
| [Dusseault2009] | Dusseault, L., "WebDAV Server Implementation Guide", in *Web Distributed Authoring and Versioning (WebDAV)*, Morgan Kaufmann, 2009. |
| [MS-WEBDAV] | Microsoft, "[MS-WEBDAV]: Web Distributed Authoring and Versioning (WebDAV) Protocol Extensions", Microsoft Docs. |
| [rclone] | rclone project, "WebDAV backend implementation", GitHub, https://github.com/rclone/rclone/tree/master/backend/webdav |
| [warp] | tokio-rs, "warp WebDAV filters", GitHub, https://github.com/seanmonstar/warp |

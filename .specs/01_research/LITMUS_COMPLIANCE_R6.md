# Litmus WebDAV compliance: R6 baseline and results (2026-10-26)

First external compliance validation of Ferro's WebDAV implementation
(litmus 0.13, built from source at `/tmp/opencode/litmus/inst/bin/litmus`;
server: debug build, `FERRO_STORAGE=local:`, Basic auth + Cedar policy file,
per-user root resolution active).

## Scores

| Suite | Baseline | After R6 fixes | Delta |
|---|---|---|---|
| basic (PUT/GET/MKCOL/conditionals) | 13/16 (81.2%) | **16/16 (100%)** | +3 |
| copymove | 7/13 (53.8%) | **12/13 (92.3%)** | +5 |
| props | 8/14 (57.1%; 16 SKIPPED) | **25/30 (83.3%)** | +17 net |
| locks | 25/34 (73.5%) | **32/34 (94.1%)** | +7 |
| http | 4/4 (100%) | 4/4 (100%) | 0 |
| **Total** | **57/81 (70.4%)** | **72/81 (88.9%)** | **+15** |

## Fixed this round

1. **RFC 4918 §10.6 `Overwrite: F`**; COPY/MOVE onto an existing destination
   with `Overwrite: F` now 412s (was silently overwriting).
2. **§9.3.1 MKCOL strictness**; request body → 415; missing intermediate
   collection → 409 (user-root ancestors exempt).
3. **§10.4 If-header enforcement on PUT**; conditions evaluate against
   server-validated lock tokens (`LockManager::valid_submitted_tokens`) and
   the current ETag; unsatisfied → 412 (bogus tokens no longer satisfy their
   own condition).
4. **§6.1 lock scope conflicts**; an exclusive LOCK now conflicts with ANY
   existing lock (exclusive-over-shared used to be granted).
5. **§7.3 unmapped LOCK**; returns 201 (creates the empty resource).
6. **Destination isolation (security-relevant)**; COPY/MOVE Destination
   headers are now resolved through the per-user root mapping like request
   paths; previously a Destination header bypassed `/users/<sub>/` isolation
   entirely.
7. **§9.8.3 depth-infinity collection COPY**; implemented recursively
   (storage's flat copy cannot copy directories; litmus copymove 502s).
8. **Collection DELETE robustness**; local-backend directory delete is now
   subtree-safe (list() walks object metadata and misses COPY-created
   directories; the old `remove_dir` poisoned every later suite via cleanup
   failures).
9. **§9.1.2 PROPFIND body validation**; malformed XML (unclosed elements)
   and undeclared namespace prefixes now 400 instead of 207 (+2 props).
10. **§6.4 lock-token uniqueness (SECURITY)**; `LockToken::new()` returned
   the nil UUID for EVERY lock; any client holding any lock token could
   satisfy any other lock's token check. Now UUID v4 per lock.
11. **§10.4 conditional PUT on lock+etag**; the trait-object dispatch of
   `valid_submitted_tokens` hit the trait's empty default, so every token
   condition evaluated false (412). Overridden in
   `impl LockManagerTrait for LockManager`; cond_put now 204.
12. **ETag quote normalization**; stored etags embed their own quotes
   (`""abc""`); If-header etag comparison strips all quote layers.
13. **§9.8.4/§9.9.4 collection-overwrite COPY/MOVE**; `Overwrite: T` onto
   an existing collection now replaces the destination subtree
   (recursive delete, then copy) instead of 502 (+2 copymove).

## Reproduction

```
/tmp/opencode/run-litmus.sh   # wipes, starts server, seeds /litmus/, runs litmus
# litmus expects the BASE collection (/litmus/) to already exist
```

## R7 addendum: dead-property storage IMPLEMENTED

`handlers/deadprops.rs`: JSON sidecar per resource (`<path>.davprops`, hidden
from listings). PROPPATCH persists Set/Remove for any non-DAV: property and
returns proper 207 propstats; DAV:-namespace props are live-protected (403).
PROPFIND now honors `<D:prop>` requests; dead props from the sidecar,
getetag from metadata, missing → 404 propstat. Lifted litmus's 16 props
SKIPPINGs (suite widened 14→30). PROPPATCH on locked resources honors
submitted If-header tokens.

## Remaining gaps (R7 candidates, ordered by client impact)

1. **PROPPATCH → 403** (4 props/locks tests): dead-property storage is not
   implemented; PROPPATCH should return a 207 multistatus with per-property
   outcomes (or store dead props) instead of denying. (RESOLVED above.)
2. **PROPFIND body validation** (2 tests): malformed XML / invalid namespace
   declarations must 400, not 207.
4. **COPY of a locked source by the lock owner** (1 test): source-lock check
   must honor the submitted token exactly like destination locks.
5. **Complex conditional PUT with valid lock+etag** (1 test): the If-header
   evaluation must accept a list combining a validated token and current ETag
   (evaluation semantics; 412 fires where 204 is expected).

Note: litmus's debug-build server round-trips run 2-5 s each (3-8 min/suite);
a release build should cut that ~10×.


## 2026-10-30: namespace handling consolidated

ALSO: PROPFIND/PROPPATCH response hrefs leaked the internal storage layout
(`/users/<sub>/...`) instead of echoing the request URI; a compliance bug
(neon matches response hrefs against the request path, which is why propget
reported "No value given") AND an information-disclosure issue. Fixed by
rebasing response hrefs onto the request path in both handlers (the
user-root prefix is stripped); internal paths never leave the server.

Clark-notation keys (default-ns included), element-scoped xmlns scope,
NsScope generated prefixes for responses. props 25/30; remaining 5 are
state-dependent propget sequences inside litmus's own test flow (responses
verified valid via expat); needs a request/replay trace to close.

## 2026-10-30 round 2: live props + href fixes, wire-verified server

- PROPFIND requested-props responses now serve the live DAV: set (resourcetype
  with nested collection, getcontentlength/type, getlastmodified, creationdate)
  alongside dead props + getetag; `propfind_d0` passes.
- Response hrefs rebased to the request URI (internal /users/<sub>/ no longer
  leaked; dead-prop lookups stay on resolved paths; trailing slash preserved).
- Wire-verified: the propget response for litmus's 10-prop sequence contains
  all 10 values in a 200 propstat with correct namespace declarations (captured
  via response logging; expat-parses clean).
- Remaining 4 props failures show a STALE-VALUE pattern (removeset: got x,
  expected y after set-x/remove/set-y); suspect a read-cache or concurrent
  load/save race in the sidecar path; needs a PROPPATCH-body capture (add the
  PROPPATCH-DBG probe back) plus sidecar dumps between requests.
- Final: props 26/30 (86.7%), overall 90/97 (92.8%).

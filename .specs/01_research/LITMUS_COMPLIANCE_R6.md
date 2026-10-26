# Litmus WebDAV compliance — R6 baseline and results (2026-10-26)

First external compliance validation of Ferro's WebDAV implementation
(litmus 0.13, built from source at `/tmp/opencode/litmus/inst/bin/litmus`;
server: debug build, `FERRO_STORAGE=local:`, Basic auth + Cedar policy file,
per-user root resolution active).

## Scores

| Suite | Baseline | After R6 fixes | Delta |
|---|---|---|---|
| basic (PUT/GET/MKCOL/conditionals) | 13/16 (81.2%) | **16/16 (100%)** | +3 |
| copymove | 7/13 (53.8%) | **10/13 (76.9%)** | +3 |
| props | 8/14 (57.1%) | 8/14 (57.1%) | 0 |
| locks | 25/34 (73.5%) | **29/34 (85.3%)** | +4 |
| http | 4/4 (100%) | 4/4 (100%) | 0 |
| **Total** | **57/81 (70.4%)** | **67/81 (82.7%)** | **+10** |

## Fixed this round

1. **RFC 4918 §10.6 `Overwrite: F`** — COPY/MOVE onto an existing destination
   with `Overwrite: F` now 412s (was silently overwriting).
2. **§9.3.1 MKCOL strictness** — request body → 415; missing intermediate
   collection → 409 (user-root ancestors exempt).
3. **§10.4 If-header enforcement on PUT** — conditions evaluate against
   server-validated lock tokens (`LockManager::valid_submitted_tokens`) and
   the current ETag; unsatisfied → 412 (bogus tokens no longer satisfy their
   own condition).
4. **§6.1 lock scope conflicts** — an exclusive LOCK now conflicts with ANY
   existing lock (exclusive-over-shared used to be granted).
5. **§7.3 unmapped LOCK** — returns 201 (creates the empty resource).
6. **Destination isolation (security-relevant)** — COPY/MOVE Destination
   headers are now resolved through the per-user root mapping like request
   paths; previously a Destination header bypassed `/users/<sub>/` isolation
   entirely.
7. **§9.8.3 depth-infinity collection COPY** — implemented recursively
   (storage's flat copy cannot copy directories; litmus copymove 502s).
8. **Collection DELETE robustness** — local-backend directory delete is now
   subtree-safe (list() walks object metadata and misses COPY-created
   directories; the old `remove_dir` poisoned every later suite via cleanup
   failures).

## Reproduction

```
/tmp/opencode/run-litmus.sh   # wipes, starts server, seeds /litmus/, runs litmus
# litmus expects the BASE collection (/litmus/) to already exist
```

## Remaining gaps (R7 candidates, ordered by client impact)

1. **PROPPATCH → 403** (4 props/locks tests): dead-property storage is not
   implemented; PROPPATCH should return a 207 multistatus with per-property
   outcomes (or store dead props) instead of denying.
2. **PROPFIND body validation** (2 tests): malformed XML / invalid namespace
   declarations must 400, not 207.
3. **Collection overwrite COPY/MOVE** (2 tests): `Overwrite: T` onto an
   existing collection must replace it (currently 502 — needs recursive
   delete-then-copy).
4. **COPY of a locked source by the lock owner** (1 test): source-lock check
   must honor the submitted token exactly like destination locks.
5. **Complex conditional PUT with valid lock+etag** (1 test): the If-header
   evaluation must accept a list combining a validated token and current ETag
   (evaluation semantics — 412 fires where 204 is expected).

Note: litmus's debug-build server round-trips run 2-5 s each (3-8 min/suite);
a release build should cut that ~10×.

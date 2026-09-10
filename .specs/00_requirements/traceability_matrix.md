# Ferro Traceability Matrix

**Document ID:** FERRO-TM-001
**Date:** 2026-04-18
**Status:** Draft

---

## 1. Requirements → Subsystem → Implementation Crate → Phase

| Requirement ID | Subsystem | Implementation Crate | Phase | Priority |
|---------------|-----------|---------------------|-------|----------|
| REQ-STOR-001 | Storage | `ferro-storage` | 1 | Must |
| REQ-STOR-002 | Storage | `ferro-storage` | 1 | Must |
| REQ-STOR-003 | Storage | `ferro-storage` + `ferro-meta` | 2 | Should |
| REQ-STOR-004 | Storage | `ferro-storage` | 1 | Should |
| REQ-STOR-005 | Storage | `ferro-storage` | 1 | Must |
| REQ-STOR-006 | Storage | `ferro-storage` | 1 | Must |
| REQ-WEBDAV-001 | Interface | `ferro-webdav` | 1 | Must |
| REQ-WEBDAV-002 | Interface | `ferro-webdav` | 1 | Must |
| REQ-WEBDAV-003 | Interface | `ferro-webdav` + `ferro-auth` | 2 | Should |
| REQ-WEBDAV-004 | Interface | `ferro-webdav` | 1 | Must |
| REQ-WEBDAV-005 | Interface | `ferro-webdav` + `ferro-wopi` | 2 | Should |
| REQ-WEBDAV-006 | Interface | `ferro-webdav` | 1 | Should |
| REQ-WOPI-001 | Interface | `ferro-wopi` | 2 | Should |
| REQ-WOPI-002 | Interface | `ferro-wopi` | 3 | Could |
| REQ-WOPI-003 | Interface | `ferro-wopi` | 3 | Could |
| REQ-WOPI-004 | Interface | `ferro-wopi` | 2 | Should |
| REQ-WOPI-005 | Interface | `ferro-wopi` + `ferro-webdav` | 2 | Must |
| REQ-AUTH-001 | Identity | `ferro-auth` | 2 | Must |
| REQ-AUTH-002 | Identity | `ferro-auth` | 2 | Must |
| REQ-AUTH-003 | Identity | `ferro-auth` | 2 | Must |
| REQ-AUTH-004 | Identity | `ferro-auth` | 2 | Must |
| REQ-WASM-001 | WASM | `ferro-wasm` | 5 | Should |
| REQ-WASM-002 | WASM | `ferro-wasm` | 5 | Should |
| REQ-WASM-003 | WASM | `ferro-wasm` | 5 | Must |
| REQ-WASM-004 | WASM | `ferro-wasm` | 5 | Must |
| REQ-SEARCH-001 | Search | `ferro-search` | 5 | Should |
| REQ-SEARCH-002 | Search | `ferro-search` | 5 | Should |
| REQ-SEARCH-003 | Search | `ferro-search` | 5 | Should |
| REQ-WEB-001 | Frontend | `ferro-web` | 3 | Should |
| REQ-WEB-002 | Frontend | `ferro-web` | 3 | Should |
| REQ-WEB-003 | Frontend | `ferro-web` | 3 | Should |
| REQ-WEB-004 | Frontend | `ferro-web` | 3 | Could |
| REQ-WEB-005 | Frontend | `ferro-common` + `ferro-web` | 3 | Must |
| REQ-DESK-001 | Desktop | `ferro-desktop` | 4 | Should |
| REQ-DESK-002 | Desktop | `ferro-desktop` | 4 | Should |
| REQ-DESK-003 | Desktop | `ferro-desktop` | 4 | Must |
| REQ-DESK-004 | Desktop | `ferro-desktop` | 4 | Could |
| REQ-DESK-005 | Desktop | `ferro-desktop` | 4 | Could |
| REQ-ENT-001 | Enterprise | `ferro-audit` | 2 | Must |
| REQ-ENT-002 | Enterprise | `ferro-audit` | 2 | Should |
| REQ-ENT-003 | Enterprise | `ferro-meta` | 2 | Should |
| REQ-ENT-004 | Enterprise | `ferro-storage` + `ferro-meta` | 5 | Should |
| REQ-ENT-005 | Enterprise | `ferro-storage` | 5 | Could |

---

## 2. Requirements by Implementation Crate

| Crate | Requirements | Count |
|-------|-------------|-------|
| `ferro-storage` | REQ-STOR-001, 002, 003, 004, 005, 006, REQ-ENT-004, 005 | 8 |
| `ferro-webdav` | REQ-WEBDAV-001, 002, 003, 004, 005, 006, REQ-WOPI-005 | 7 |
| `ferro-wopi` | REQ-WOPI-001, 002, 003, 004, 005 | 5 |
| `ferro-auth` | REQ-AUTH-001, 002, 003, 004, REQ-WEBDAV-003 | 5 |
| `ferro-wasm` | REQ-WASM-001, 002, 003, 004 | 4 |
| `ferro-search` | REQ-SEARCH-001, 002, 003 | 3 |
| `ferro-meta` | REQ-STOR-003, REQ-ENT-003, 004 | 3 |
| `ferro-web` | REQ-WEB-001, 002, 003, 004, 005 | 5 |
| `ferro-common` | REQ-WEB-005 | 1 |
| `ferro-desktop` | REQ-DESK-001, 002, 003, 004, 005 | 5 |
| `ferro-audit` | REQ-ENT-001, 002 | 2 |

---

## 3. Standards → Requirements Mapping

| Standard | Applicable Requirements |
|----------|------------------------|
| **RFC 4918 (WebDAV)** | REQ-WEBDAV-001, 002, 003, 004, 005, 006, REQ-WOPI-005 |
| **RFC 3253 (WebDAV Versioning/Locking)** | REQ-WEBDAV-002, 005, REQ-WOPI-005 |
| **RFC 3744 (WebDAV ACL)** | REQ-WEBDAV-003, REQ-AUTH-003 |
| **WOPI Specification** | REQ-WOPI-001, 002, 003, 004, 005, REQ-WEBDAV-005 |
| **OpenID Connect Core 1.0** | REQ-AUTH-001, 004 |
| **Cedar Specification** | REQ-AUTH-002, 003, REQ-WEBDAV-003 |
| **WASM/WASI Specification** | REQ-WASM-001, 002, 003, 004 |
| **ISO 27001** | REQ-STOR-001, 004, REQ-AUTH-002, 003, REQ-WEB-001, 004, 005, REQ-DESK-001, 003, REQ-ENT-001, 003, 005 |
| **ISO 27034** | REQ-WEB-001, 005 |
| **NIST SP 800-53** | REQ-STOR-001, 002, 005, 006, REQ-AUTH-001, 002, 003, 004, REQ-WASM-003, REQ-ENT-001, 002, REQ-DESK-003 |
| **GDPR / CCPA** | REQ-STOR-002, 003, REQ-WEB-003, REQ-ENT-001, REQ-ENT-004 |
| **OWASP Top 10** | REQ-STOR-001, 004, REQ-AUTH-001, REQ-WASM-003, REQ-WEB-001, 005 |

---

## 4. Phase → Requirements Summary

| Phase | Must | Should | Could | Total |
|-------|------|--------|-------|-------|
| **Phase 1 (The Core)** | 7 | 2 | 0 | 9 |
| **Phase 2 (The Metadata)** | 6 | 7 | 0 | 13 |
| **Phase 3 (The Frontend)** | 1 | 3 | 2 | 6 |
| **Phase 4 (The Desktop)** | 1 | 2 | 2 | 5 |
| **Phase 5 (The Intelligence)** | 2 | 5 | 1 | 8 |
| **Cross-Phase** | 1 | 2 | 1 | 3 |
| **Total** | **18** | **21** | **6** | **44** |

---

## 5. Risk → Requirements Coverage

| Risk ID | Risk Description | Mitigating Requirements |
|---------|-----------------|------------------------|
| CPR-1.1 | WebDAV XML parsing overhead | REQ-WEBDAV-001, REQ-WEBDAV-006 |
| CPR-1.2 | CAS dedup atomicity with object_store | REQ-STOR-002, REQ-STOR-005 |
| CPR-1.3 | rclone WebDAV locking compatibility | REQ-WEBDAV-002, REQ-WEBDAV-004 |
| CPR-1.4 | Pre-signed URL security | REQ-STOR-004 |
| CPR-2.1 | Cedar policy evaluation latency | REQ-AUTH-002, REQ-AUTH-003 |
| CPR-2.2 | Dual-backend metadata schema drift | REQ-AUTH-001 (shared schema) |
| CPR-2.3 | Cedar ↔ OIDC claim mapping | REQ-AUTH-001, REQ-AUTH-003 |
| CPR-3.1 | Leptos SSR hydration mismatches | REQ-WEB-001, REQ-WEB-005 |
| CPR-3.2 | Virtualized scrolling at 10K+ files | REQ-WEB-002 |
| CPR-4.1 | rclone sidecar lifecycle | REQ-DESK-002, REQ-DESK-003 |
| CPR-4.2 | Cross-platform credential injection | REQ-DESK-003 |
| CPR-4.3 | Tauri WebView compatibility | REQ-DESK-001 |
| CPR-5.1 | Wasmtime sandbox escape | REQ-WASM-003, REQ-WASM-004 |
| CPR-5.2 | Tantivy index corruption | REQ-SEARCH-001, REQ-SEARCH-002 |
| CPR-5.3 | WASM plugin ABI stability | REQ-WASM-001 |

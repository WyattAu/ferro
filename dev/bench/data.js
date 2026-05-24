window.BENCHMARK_DATA = {
  "lastUpdate": 1779661002424,
  "repoUrl": "https://github.com/WyattAu/ferro",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "distinct": true,
          "id": "ab35e2ca4d33a4658a1589554c191bc01db328a9",
          "message": "fix: eliminate bcrypt panic in hash_password, correct doc inaccuracies\n\n- Change hash_password() to return Result<String, UserError> instead of\n  panicking on bcrypt failure (the only remaining production expect)\n- Update create_admin() to return Option<User> with graceful error handling\n- Fix all 6 callers (server API, user_api, main, simple_auth tests)\n- Fix rate limiter terminology: sliding window -> token-bucket across\n  introduction.md and security.md (matches actual implementation)\n- Fix ROADMAP.md crate count: 21 -> 20 (matches Cargo.toml)\n- Fix ROADMAP.md unwrap claims: update TD-001 to reflect actual state\n- Fix deployment.md: remove duplicate quick-start section\n- Fix deployment.md: health check returns JSON, not plain text\n- Fix owasp-checklist.md: remove stale v1.1 references\n- Update VERSION.md test count: 813 -> 814\n\nAll 814 tests pass, 0 clippy warnings, fmt clean, cargo-deny clean.",
          "timestamp": "2026-05-20T02:15:35+01:00",
          "tree_id": "f5ca11d373ec97e94a9de55fdda80783e0190ab2",
          "url": "https://github.com/WyattAu/ferro/commit/ab35e2ca4d33a4658a1589554c191bc01db328a9"
        },
        "date": 1779240205213,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 254156612,
            "range": "± 1283766",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 254116419,
            "range": "± 1034320",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 22964,
            "range": "± 2614",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23017,
            "range": "± 1159",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8652,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5314,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1172,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 880,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2629,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1504,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8591,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 79084,
            "range": "± 1573",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 105,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 21233,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 964,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 88,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 84,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 148,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 156,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 159,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 703,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 800,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "distinct": true,
          "id": "5149f88496f1a28a55ca0e9cf9e1be6ff656496b",
          "message": "feat: implement Phase 1-2 roadmap items (production hardening, observability)\n\nPhase 1 - Production Hardening:\n- AU-005: Warn when external_url uses HTTP in non-localhost config\n- AU-006: Remove plaintext password from startup warning log\n- AU-011: Add schema_version field to config file format\n- AX-007: Add XML body size limits (10 MiB) in all parsers\n- TD-010: Pin third-party image tags in all docker-compose files\n  (grafana:11.5.2, loki:3.3.2, prometheus:v3.1.0,\n   victoriametrics:v1.108.0, victorialogs:v0.8.0, vmagent:v1.108.0)\n\nPhase 2 - Reliability and Observability:\n- AV-006: Implement panic handler middleware (logs 500 context)\n- AV-007: Graceful degradation on search runtime errors\n- AV-011: Dynamic WASM worker count in prometheus metrics\n- AV-014: Reduce benchmark regression threshold 150% -> 120%\n- AX-003: Add base-uri and form-action to CSP header\n\nPhase 1+ - Backup:\n- AU-004: Add SQLite VACUUM INTO to admin backup API\n\n814 tests passing, 0 clippy warnings",
          "timestamp": "2026-05-20T15:56:34+01:00",
          "tree_id": "130e1946c82a0451ab9bf0628805577d7c68c282",
          "url": "https://github.com/WyattAu/ferro/commit/5149f88496f1a28a55ca0e9cf9e1be6ff656496b"
        },
        "date": 1779289491610,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300769978,
            "range": "± 1354405",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300768196,
            "range": "± 913691",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24346,
            "range": "± 5746",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 24023,
            "range": "± 5441",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8495,
            "range": "± 153",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5210,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1277,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 938,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2655,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1384,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8155,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74886,
            "range": "± 400",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 92,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19248,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 888,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 73,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 150,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 176,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 180,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 912,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 755,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "distinct": true,
          "id": "2b63673f57c56f8dd07db5200407df00415f237e",
          "message": "feat: implement Phase 2-4 roadmap items (slow query, share brute-force)\n\nPhase 2 - Reliability:\n- AV-008: Add SQLite slow query logging (>100ms threshold) via rusqlite\n  profile callback with 'trace' feature\n\nPhase 4 - Security:\n- AX-008: Share link brute-force protection for password-protected shares\n  - 10 max failed attempts per token\n  - 5-minute lockout after exceeding limit\n  - Tracks failures in DashMap with automatic expiry\n  - Returns 429 with remaining time on lockout\n\n814 tests passing, 0 clippy warnings",
          "timestamp": "2026-05-20T16:12:06+01:00",
          "tree_id": "598ffd098c980e6211ce0b8a5d2615a379ea88a3",
          "url": "https://github.com/WyattAu/ferro/commit/2b63673f57c56f8dd07db5200407df00415f237e"
        },
        "date": 1779290444531,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300708153,
            "range": "± 534499",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300736590,
            "range": "± 1264081",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24749,
            "range": "± 4983",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 24165,
            "range": "± 690",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8769,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5229,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1204,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 971,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2693,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1384,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8159,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74872,
            "range": "± 2080",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 92,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19403,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 874,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 151,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 176,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 179,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 889,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 755,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "id": "ad7a213eb4cad8a3fbb2e53f34c93d5898eef44b",
          "message": "chore(deps): bump utoipa-swagger-ui from 8.1.0 to 9.0.2",
          "timestamp": "2026-05-20T15:07:49Z",
          "url": "https://github.com/WyattAu/ferro/pull/18/commits/ad7a213eb4cad8a3fbb2e53f34c93d5898eef44b"
        },
        "date": 1779290653035,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267090176,
            "range": "± 724460",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 266994949,
            "range": "± 134099",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 27732,
            "range": "± 2056",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 27685,
            "range": "± 3033",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8631,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5268,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1224,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 932,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2767,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1323,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7387,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66747,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 86,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19374,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 886,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 77,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 67,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 147,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 180,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 189,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 868,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 824,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "id": "735c7fd6de828e54e0edc961ad26a51db03071a2",
          "message": "chore(deps): bump pdf from 0.9.1 to 0.10.0",
          "timestamp": "2026-05-20T15:07:49Z",
          "url": "https://github.com/WyattAu/ferro/pull/20/commits/735c7fd6de828e54e0edc961ad26a51db03071a2"
        },
        "date": 1779290664069,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300781501,
            "range": "± 1513771",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300744477,
            "range": "± 974550",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24128,
            "range": "± 857",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23796,
            "range": "± 849",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8756,
            "range": "± 157",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5233,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1248,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 956,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2674,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1388,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8119,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74795,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 96,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 18940,
            "range": "± 759",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 894,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 158,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 176,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 179,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 921,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 760,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "id": "d15eed0695243a214fc435c10a66b227381692b3",
          "message": "chore(deps): bump pdf from 0.9.1 to 0.10.0",
          "timestamp": "2026-05-20T17:55:59Z",
          "url": "https://github.com/WyattAu/ferro/pull/20/commits/d15eed0695243a214fc435c10a66b227381692b3"
        },
        "date": 1779302549841,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 254088545,
            "range": "± 444146",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 254135865,
            "range": "± 136107",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 21802,
            "range": "± 2020",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 21663,
            "range": "± 1993",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8919,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5224,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1112,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 860,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2584,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1496,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8553,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 78563,
            "range": "± 292",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 103,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 22327,
            "range": "± 807",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 941,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 80,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 142,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 151,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 721,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 796,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "distinct": true,
          "id": "ec2bb0158af2a7787d37a87d50af742aa91b286f",
          "message": "fix: restore proptest dev-dependency for property-based tests\n\nProptest was accidentally removed during a git checkout revert.\nRequired for property_tests.rs compilation in CI.",
          "timestamp": "2026-05-21T02:42:14+01:00",
          "tree_id": "8a573552ba1b437eb3a3b7b0f179fdb3321870c6",
          "url": "https://github.com/WyattAu/ferro/commit/ec2bb0158af2a7787d37a87d50af742aa91b286f"
        },
        "date": 1779328149952,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267547326,
            "range": "± 375295",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267499569,
            "range": "± 277441",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 28849,
            "range": "± 2427",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 28435,
            "range": "± 2251",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8726,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5083,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1298,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 1006,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2791,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1335,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7324,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66511,
            "range": "± 110",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19228,
            "range": "± 243",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 945,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 73,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 72,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 146,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 171,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 876,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 805,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "distinct": true,
          "id": "d274895119b1842e046ac823a5227bd1dced6011",
          "message": "feat(security): secret redaction in logs and atomic file writes\n\n- Add custom Debug impls for ServerConfig, FileConfigValues, FileConfig\n  that redact admin_password, wopi_token_secret, federation_secret,\n  ldap_bind_password, and metadata_db credentials\n- Add redact_url_credentials() helper for sanitizing PostgreSQL/Redis URLs\n  in log output (postgres://user:***REDACTED***@host)\n- Fix 3 log lines in main.rs that leaked DB/Redis connection URLs\n- Add ferro_core::fs_util::atomic_write() using temp-file-then-rename\n  pattern to prevent partial file corruption on crash\n- Convert 7 bare fs::write sites to atomic writes: backup.rs (2),\n  trash.rs (1), thumbnails.rs (1), wasm_upload.rs (1),\n  server-versioning (2)\n- Add 11 new tests (6 redaction + 5 atomic write)",
          "timestamp": "2026-05-24T08:59:55+01:00",
          "tree_id": "53e705aa55243736d1cc2410e5be4a0e09339d30",
          "url": "https://github.com/WyattAu/ferro/commit/d274895119b1842e046ac823a5227bd1dced6011"
        },
        "date": 1779610363224,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267154699,
            "range": "± 718056",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267090740,
            "range": "± 128694",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 28289,
            "range": "± 2039",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 27687,
            "range": "± 2484",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9392,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5121,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1172,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 906,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2770,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1324,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7359,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66606,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 88,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 18811,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 874,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 67,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 146,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 179,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 187,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 876,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 829,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "distinct": true,
          "id": "52e68516911d9e5dc29ae1302141fd1517c6fc7f",
          "message": "feat(auth,metrics): OIDC refresh, LDAP group mapping, Prometheus fix\n\n- Add OIDC token refresh: POST /api/auth/refresh endpoint accepts\n  refresh_token, exchanges it for new access_token via provider\n  token_endpoint. Returns new refresh_token if provider rotates.\n- Add OidcValidator::refresh_access_token() method using grant_type=\n  refresh_token against discovered token_endpoint\n- Add LDAP group-to-role mapping: new fields on LdapConfig\n  (group_search_base, group_filter, group_role_map) enable querying\n  user groups and mapping to Admin/User/ReadOnly roles\n- Fix Prometheus histogram _sum: was hardcoded to 0, now tracks\n  cumulative request duration in milliseconds via AtomicU64\n- Add config file schema_version validation: rejects unsupported\n  versions at startup with clear error message\n- Export auth_refresh_token route on /api/auth/refresh",
          "timestamp": "2026-05-24T15:55:53+01:00",
          "tree_id": "88c7039065b7115d063a8225d014b0cdcee2f8f4",
          "url": "https://github.com/WyattAu/ferro/commit/52e68516911d9e5dc29ae1302141fd1517c6fc7f"
        },
        "date": 1779635265645,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 254182644,
            "range": "± 962867",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 254050296,
            "range": "± 106396",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 23009,
            "range": "± 2143",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 22366,
            "range": "± 1937",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8801,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5293,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1108,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 862,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2589,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1494,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8525,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 78421,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 108,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 21232,
            "range": "± 309",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 966,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 79,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 142,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 153,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 161,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 721,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 776,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "distinct": true,
          "id": "160bdf7ac690b6f41397355d6d2855f4587231fc",
          "message": "feat(audit): chain hash verification endpoint and security model audit\n\nAdd SqlitePersistence::verify_audit_chain() to recompute and validate\nSHA-256 chain hashes across all audit log entries. Expose via\nGET /api/admin/audit-chain for tamper detection.\n\nSecurity audit confirms CSRF protection unnecessary: Ferro uses\nheader-based auth (Basic+Bearer) with no cookies. Session token\nrotation similarly not applicable.\n\n3 new tests: valid chain, tamper detection, legacy NULL skip.\nROADMAP updated: 847 tests, 5 more items marked DONE/N/A.",
          "timestamp": "2026-05-24T17:55:01+01:00",
          "tree_id": "1e1d531a47391bfe162c6ee663cbc21b45718eaa",
          "url": "https://github.com/WyattAu/ferro/commit/160bdf7ac690b6f41397355d6d2855f4587231fc"
        },
        "date": 1779642109267,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267154370,
            "range": "± 1620217",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267063685,
            "range": "± 415973",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 27787,
            "range": "± 2440",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 27744,
            "range": "± 2144",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9136,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5059,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1159,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 903,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2894,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1320,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7319,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66499,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 87,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 18454,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 877,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 149,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 177,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 187,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 856,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 844,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "committer": {
            "email": "wyatt_au@protonmail.com",
            "name": "WyattAu",
            "username": "WyattAu"
          },
          "distinct": true,
          "id": "0b3066c5dd350183dcbdb0937ef11138f298835e",
          "message": "feat(security,metrics): path validation, WASM/cache metrics, Content-Type logging\n\nAdd security::validate_path() to encryption handlers (encrypt_file,\ndecrypt_file) to prevent path traversal via JSON request bodies.\n\nAdd WASM worker metrics to AppState: dispatch count, error count,\nfuel consumed. Update inline trigger and background runner to track\nmetrics. Expose via Prometheus endpoint.\n\nExpose read cache hit/miss/eviction counters in Prometheus output.\n\nLog Content-Type mismatches in WebDAV PUT handler (warn level)\nwithout blocking uploads for compatibility.\n\nROADMAP updated: property tests verified (19 cases), CSP/cookies\ndocumented, 8 more items marked DONE/N/A.",
          "timestamp": "2026-05-24T23:09:48+01:00",
          "tree_id": "75e21e237c18b3d999db4144e61c3eff481f3c89",
          "url": "https://github.com/WyattAu/ferro/commit/0b3066c5dd350183dcbdb0937ef11138f298835e"
        },
        "date": 1779661001965,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300782900,
            "range": "± 1849917",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300748340,
            "range": "± 1240245",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 23948,
            "range": "± 874",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23408,
            "range": "± 2950",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9189,
            "range": "± 136",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5127,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1206,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 938,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2688,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1411,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8176,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74828,
            "range": "± 727",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 94,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 18925,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 909,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 153,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 175,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 178,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 905,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 785,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "ferro_error_not_found",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}
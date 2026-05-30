window.BENCHMARK_DATA = {
  "lastUpdate": 1780175305192,
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
          "id": "862b89af8e0a056067d50f0ba5d7b8db659b88ac",
          "message": "feat(observability,tests): startup probe, XML proptests, SRI assessment\n\nAdd GET /startupz endpoint for Kubernetes-style startup probes.\nReturns 200 after all startup checks (CAS verification, storage\nreachability) pass. Returns 503 during initialization.\n\nAdd 6 XML parsing property tests using proptest: fuzz parse_proppatch\nand LockRequest::parse with random XML-like content to verify no\npanics. Test valid PROPPATCH/LOCK XML parsing. Total: 25 property\ntests (up from 19).\n\nAssess SRI: only external CDN is Google Fonts CSS (dynamic per UA,\nSRI inapplicable). System font fallback covers offline deployments.",
          "timestamp": "2026-05-24T23:37:30+01:00",
          "tree_id": "dc636655dce00a605b6566f674befd6fa8b297a3",
          "url": "https://github.com/WyattAu/ferro/commit/862b89af8e0a056067d50f0ba5d7b8db659b88ac"
        },
        "date": 1779662658148,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300660425,
            "range": "± 742936",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300615455,
            "range": "± 1077605",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 23690,
            "range": "± 879",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23632,
            "range": "± 671",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9122,
            "range": "± 143",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5042,
            "range": "± 99",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1184,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 942,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2934,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1407,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8148,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74759,
            "range": "± 466",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 92,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19087,
            "range": "± 230",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 898,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 70,
            "range": "± 3",
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
            "value": 175,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 179,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 936,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 771,
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
          "id": "f75c5c00e55f69f0f7c91829623e66278bfbfddc",
          "message": "feat(tools,migration): storage migration tool, Grafana dashboard\n\nAdd --migrate-from flag to ferro-server for cross-backend data\nmigration. Copies all files from a source storage backend to the\nconfigured destination. Skips existing files. Reports progress\nevery 100 files with final summary (copied/skipped/errors).\n\nExtract build_storage_backend() helper to support creating storage\nbackends independently of AppState (used by migration source).\n\nAdd Grafana dashboard template with 10 panels: request rate,\nduration percentiles, status codes, storage operations, cache\nhit rate, file count, storage used, uptime, WASM workers,\ncache evictions.",
          "timestamp": "2026-05-25T00:41:19+01:00",
          "tree_id": "a9359860d13cc2d30658e20ff4711d73173d865c",
          "url": "https://github.com/WyattAu/ferro/commit/f75c5c00e55f69f0f7c91829623e66278bfbfddc"
        },
        "date": 1779666488576,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300689747,
            "range": "± 779259",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300731423,
            "range": "± 212852",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24293,
            "range": "± 690",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23768,
            "range": "± 703",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9038,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5195,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1185,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 922,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2721,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1400,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8144,
            "range": "± 170",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74851,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 93,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19257,
            "range": "± 304",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 888,
            "range": "± 5",
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
            "range": "± 2",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 899,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 754,
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
          "id": "806c866dc6e2175141e96149f19c731c4fa7190e",
          "message": "Fuzzing harnesses and load testing infrastructure\n\nFuzzing (cargo-fuzz, 4 harnesses):\n- fuzz_proppatch: arbitrary bytes to parse_proppatch, 613K iters/10s, 0 crashes\n- fuzz_lock_request: arbitrary bytes to LockRequest::parse, 663K iters/10s, 0 crashes\n- fuzz_escape_xml: verifies no raw < > \" ' in escaped output, 1.3M iters/10s\n- fuzz_wasm_magic: validates WASM magic byte check correctness\n\nLoad testing (k6, 3 scripts):\n- concurrent-upload.js: ramps to 100 VUs, PUT+GET+DELETE with thresholds\n- large-directory.js: populates N files, benchmarks PROPFIND depth:1 and infinity\n- soak-test.js: 1h continuous random ops (PUT/GET/DELETE/PROPFIND/COPY)\n\nROADMAP: Phases 3.3 and 3.4 complete. Only 1 item remains (external pen test).",
          "timestamp": "2026-05-25T19:29:13+01:00",
          "tree_id": "89b801c095b82081e1241de80fc5d30fcac89b17",
          "url": "https://github.com/WyattAu/ferro/commit/806c866dc6e2175141e96149f19c731c4fa7190e"
        },
        "date": 1779734171816,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267119810,
            "range": "± 686095",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267176269,
            "range": "± 187096",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 31355,
            "range": "± 2223",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 28239,
            "range": "± 1924",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9217,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5157,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1174,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 909,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2766,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1332,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7349,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66410,
            "range": "± 60",
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
            "value": 18809,
            "range": "± 187",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 879,
            "range": "± 24",
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
            "value": 147,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 180,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 188,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 881,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 852,
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
          "id": "fda03e9520baf2609388ff7132ff03bf8547cebc",
          "message": "fix(ci): correct benchmark-action SHA (was invalid digest)",
          "timestamp": "2026-05-26T21:07:24+01:00",
          "tree_id": "ef091cb01cfeba708efd714628544543120522c4",
          "url": "https://github.com/WyattAu/ferro/commit/fda03e9520baf2609388ff7132ff03bf8547cebc"
        },
        "date": 1779826614663,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267151723,
            "range": "± 1349461",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267096350,
            "range": "± 373531",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 29895,
            "range": "± 2087",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 28376,
            "range": "± 2509",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9143,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5097,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1157,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 909,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2911,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1328,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7352,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66486,
            "range": "± 106",
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
            "value": 18925,
            "range": "± 254",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 909,
            "range": "± 9",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 175,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 185,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 865,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 843,
            "range": "± 8",
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
          "id": "271250afabeb76f1adc82c351ef57b2a4cd5fd10",
          "message": "fix(web,docker): IntersectionObserver root=scroll container, fix victoria-logs image\n\nIntersectionObserver now uses the scroll container as root element instead\nof the viewport. This ensures intersection is computed relative to the\nscrollable file list area, not the browser viewport. This fixes the\ninfinite scroll E2E test which was failing in CI headless browsers.\n\nDocker fix:\n- victoriametrics/victorialogs:v0.8.0 never existed on Docker Hub\n- Replaced with victoriametrics/victoria-logs:v1.50.0 (pinned to SHA)",
          "timestamp": "2026-05-27T02:19:23+01:00",
          "tree_id": "eade92e18eee5c0014c2a684cf1d99cdaff5802e",
          "url": "https://github.com/WyattAu/ferro/commit/271250afabeb76f1adc82c351ef57b2a4cd5fd10"
        },
        "date": 1779845177233,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267151394,
            "range": "± 1525409",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267095108,
            "range": "± 136723",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 28824,
            "range": "± 1799",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 28538,
            "range": "± 2410",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9053,
            "range": "± 238",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5080,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1155,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 903,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2773,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1330,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7402,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66732,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 87,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 18646,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 876,
            "range": "± 4",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 178,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 185,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 896,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 826,
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
          "id": "77f306b1522e763158092dea18f56a7ed727a59e",
          "message": "fix(ci,docs): pin actions, fix doc inaccuracies, add pre-commit hook\n\n- docs.yml: add missing toolchain + rust-cache\n- bench.yml: add missing rust-cache\n- release.yml: fix softprops/action-gh-release SHA\n- rest.md: add /api/v1/ deprecation note\n- websocket.md: remove fabricated 1000-connection limit\n- installation.md: fix Rust version 1.92 -> 1.95.0\n- introduction.md: qualify binary size claim\n- configuration.md: add missing CLI flags (maintenance-mode, api-version, cors-origins, migrate-from)\n- .githooks/pre-commit: fast hook (fmt + clippy), full suite in CI",
          "timestamp": "2026-05-27T09:47:33+01:00",
          "tree_id": "d6aa4cd58a75fff66f0e1d58fb27dfd112663fc8",
          "url": "https://github.com/WyattAu/ferro/commit/77f306b1522e763158092dea18f56a7ed727a59e"
        },
        "date": 1779872114510,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300634554,
            "range": "± 911742",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300600418,
            "range": "± 142482",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 23873,
            "range": "± 762",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 24078,
            "range": "± 1058",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9025,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5085,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1187,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 939,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2681,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1416,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8169,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74855,
            "range": "± 96",
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
            "value": 19220,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 916,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 91,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 71,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 154,
            "range": "± 3",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 921,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 760,
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
          "id": "881364e1a8bf285c8a49f18bfddef3e529dfd52a",
          "message": "fix(ci): increase benchmark alert threshold 120% -> 150% (CI runner noise)",
          "timestamp": "2026-05-27T09:59:19+01:00",
          "tree_id": "0a42d2a6b6589b3f343536b96db1d60b4d8586d7",
          "url": "https://github.com/WyattAu/ferro/commit/881364e1a8bf285c8a49f18bfddef3e529dfd52a"
        },
        "date": 1779872782786,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300798156,
            "range": "± 1289555",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300764584,
            "range": "± 360368",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24270,
            "range": "± 599",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 24487,
            "range": "± 941",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9179,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5203,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1182,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 945,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2670,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1398,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8139,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74852,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 92,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 18861,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 925,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 91,
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
            "value": 152,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 175,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 179,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 927,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 787,
            "range": "± 10",
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
          "id": "26c02331321d9a5f3473e469be7f1aa8cf60df84",
          "message": "fix(quality): resolve TD-015 through TD-022 technical debt items\n\nTD-015: Replace swallowed DB errors with tracing::warn/error logging\n- pg_state.rs: share, favorite, preferences operations (5 sites)\n- lib.rs: tags, sync, activity, lock store loading (4 sites)\n- snapshots.rs: persist and restore operations (2 sites)\n\nTD-017: Fix poisoned lock recovery in server-activitypub/store.rs\n- Log mutex poison warning before recovering lock\n- Log DB write failures instead of silently swallowing\n\nTD-018: Add SAFETY doc comments to 15 unsafe blocks\n- client/src/ffi.rs: 13 comments on FFI boundary operations\n- fuse/src/main.rs: 2 comments on libc syscalls\n\nTD-019: Document 70+ undocumented API endpoints\n- New docs/src/api/admin.md (admin stats, backups, users, webhooks)\n- Updated docs/src/api/rest.md (files, trash, tags, auth, sync, locks, etc.)\n- Updated docs/src/SUMMARY.md with admin page\n\nTD-021: Fix benchmark auto-push flakiness (fail-on-alert: false)\nTD-022: Opt into Node.js 24 (FORCE_JAVASCRIPT_ACTIONS_TO_NODE24)",
          "timestamp": "2026-05-27T13:16:07+01:00",
          "tree_id": "7e76135ea91bf84ab5b4e763686aea89c798e6e7",
          "url": "https://github.com/WyattAu/ferro/commit/26c02331321d9a5f3473e469be7f1aa8cf60df84"
        },
        "date": 1779884692632,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 254260184,
            "range": "± 974285",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 254260441,
            "range": "± 670202",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 23617,
            "range": "± 1391",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 22759,
            "range": "± 1303",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8888,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5152,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1106,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 862,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2579,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1516,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8565,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 78514,
            "range": "± 231",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 21293,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 991,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 88,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 80,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 144,
            "range": "± 7",
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
            "value": 159,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 771,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 765,
            "range": "± 11",
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
          "id": "45b500442841adc375d4c9250112a2c579181c40",
          "message": "fix(docs,quality): resolve TD-013/014, update release criteria to 11/11\n\n- TD-013: Replace hardcoded version '2.5.1' with 'x.y.z' in 8 doc files\n  (JSON examples, security docs) to prevent version drift\n- TD-014: Deprecate --cors-origins flag (hidden from --help),\n  add deprecation notice in configuration.md\n- Update Phase 5 release criteria: 11/11 satisfied (soak test passed)\n- Update ROADMAP.md with session 5 soak test results (21,600+ req, 0 failures)\n- Update VERSION.md to reflect 11/11 release criteria met\n- Add load-test-results.json and root package-lock.json to .gitignore",
          "timestamp": "2026-05-29T03:28:01+01:00",
          "tree_id": "f69ddc72c37600f0f0f18ba97c9d7f044e27761e",
          "url": "https://github.com/WyattAu/ferro/commit/45b500442841adc375d4c9250112a2c579181c40"
        },
        "date": 1780022958911,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267537508,
            "range": "± 1349189",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267128396,
            "range": "± 138610",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 28778,
            "range": "± 2467",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 29014,
            "range": "± 2455",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9051,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 4977,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1162,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 897,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2786,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1331,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7327,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66373,
            "range": "± 230",
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
            "value": 18866,
            "range": "± 92",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 877,
            "range": "± 7",
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
            "value": 147,
            "range": "± 0",
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
            "value": 188,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 856,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 847,
            "range": "± 14",
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
          "id": "1803972cd0a96d5ef10ee45fa8889c8854aca9cc",
          "message": "fix(quality): resolve TD-009, replace top 10 high-risk panicking calls\n\n- TD-009: Enable 'vendored' feature on utoipa-swagger-ui for offline builds\n  (eliminates build-time network dependency on swagger-ui zip download)\n- Replace expect() in main.rs TCP listener with proper error propagation\n- Replace expect() in main.rs admin user creation with error logging + exit\n- Replace expect() in WOPI HMAC init with proper error response\n- Replace expect() in webhook HMAC init with error logging + graceful skip\n- Replace expect() in federation/webhook reqwest clients with fallback\n- Replace expect() in signal handlers with error logging + exit\n- Update ROADMAP: TD-009 resolved, unwrap/expect count corrected to ~34\n- All 854 tests pass, 0 clippy warnings",
          "timestamp": "2026-05-29T04:04:48+01:00",
          "tree_id": "9bba546933c735d94b1e7ee021d507a90853b41f",
          "url": "https://github.com/WyattAu/ferro/commit/1803972cd0a96d5ef10ee45fa8889c8854aca9cc"
        },
        "date": 1780024617469,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267166511,
            "range": "± 294917",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267141466,
            "range": "± 447710",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 29839,
            "range": "± 2442",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 28272,
            "range": "± 2115",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9139,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5102,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1180,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 902,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2729,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1347,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7365,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66378,
            "range": "± 277",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 87,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 18664,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 916,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 76,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 68,
            "range": "± 5",
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
            "value": 178,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 187,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 861,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 840,
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
            "name": "Wyatt Au",
            "username": "WyattAu"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f71893a778b2778dab54759b7d569b4a44fc0c5b",
          "message": "Merge pull request #19 from WyattAu/dependabot/cargo/bcrypt-0.19.1\n\nchore(deps): bump bcrypt from 0.17.1 to 0.19.1",
          "timestamp": "2026-05-29T04:33:24+01:00",
          "tree_id": "3b644d2bbd29de52f4f7bc7270778029620d090c",
          "url": "https://github.com/WyattAu/ferro/commit/f71893a778b2778dab54759b7d569b4a44fc0c5b"
        },
        "date": 1780026237131,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300752925,
            "range": "± 1794214",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300747727,
            "range": "± 1138601",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24534,
            "range": "± 948",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 24225,
            "range": "± 771",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9016,
            "range": "± 815",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 4961,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1240,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 974,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2740,
            "range": "± 20",
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
            "value": 8139,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74847,
            "range": "± 506",
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
            "value": 19304,
            "range": "± 202",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 889,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 68,
            "range": "± 1",
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 181,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 920,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 732,
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
          "id": "be14bd69ab9813aac273a926b9453ecd80dbc137",
          "message": "fix(quality): replace remaining high-risk expect() with error handling\n\n- rclone.rs: stderr/stdout pipe access now returns error instead of panic\n- gui.rs: tauri run() now propagates errors via Result return type\n- actor.rs: rcgen KeyPair::generate() now returns Result instead of panic\n- lib.rs: ActivityPub get_actor handles key generation failure gracefully\n- Remaining expect() calls are browser/WASM invariants or compile-time constants\n- All 854 tests pass",
          "timestamp": "2026-05-29T04:54:17+01:00",
          "tree_id": "678f92ce97ede5abe0cc46bfd77dc9ccf504f18f",
          "url": "https://github.com/WyattAu/ferro/commit/be14bd69ab9813aac273a926b9453ecd80dbc137"
        },
        "date": 1780027318830,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267075172,
            "range": "± 1057991",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267045515,
            "range": "± 910598",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 28518,
            "range": "± 2330",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 28539,
            "range": "± 2404",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8983,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5095,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1202,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 968,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2760,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1314,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7311,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66625,
            "range": "± 87",
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
            "value": 18844,
            "range": "± 95",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 914,
            "range": "± 36",
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
            "value": 148,
            "range": "± 0",
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
            "value": 188,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 862,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 835,
            "range": "± 9",
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
          "id": "41b26cf006dd6c7e303f3a1ac51e88dee5ca2372",
          "message": "fix(docker): unify Rust version across build stages\n\nIntroduce ARG RUST_VERSION=1.95 and reference it in both the\nWASM build stage and the server build stage, eliminating drift\nbetween the two toolchain installations.",
          "timestamp": "2026-05-29T21:39:25+01:00",
          "tree_id": "3e9ffee519275ddb8da4b5cb0c698201b0dff2b0",
          "url": "https://github.com/WyattAu/ferro/commit/41b26cf006dd6c7e303f3a1ac51e88dee5ca2372"
        },
        "date": 1780087653920,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 267042668,
            "range": "± 1305653",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 267472202,
            "range": "± 2355585",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 31073,
            "range": "± 3155",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 27050,
            "range": "± 2177",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8766,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5147,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1209,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 922,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2790,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1321,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7327,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66452,
            "range": "± 307",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 88,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 18845,
            "range": "± 287",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 909,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 77,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 72,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 150,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 192,
            "range": "± 7",
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
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 839,
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
          "id": "aff627e17cb66a1830f9ee1beee67f50bb2d9f7a",
          "message": "docs: update VERSION.md and ROADMAP.md for v3.0.0\n\n917 tests (+63 from new features), all TD items resolved,\nPhase 6.3/6.4/6.5 items marked DONE. Web UI lock indicator\npolls /api/locks every 10s.",
          "timestamp": "2026-05-30T01:53:08+01:00",
          "tree_id": "68d650de4e9193c5772b8f4386ac993cb569f341",
          "url": "https://github.com/WyattAu/ferro/commit/aff627e17cb66a1830f9ee1beee67f50bb2d9f7a"
        },
        "date": 1780102839317,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300763175,
            "range": "± 1129742",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300705125,
            "range": "± 123618",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 25335,
            "range": "± 1025",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 25554,
            "range": "± 810",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9121,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5101,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1220,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 931,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2720,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1399,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8109,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74827,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 92,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19015,
            "range": "± 65",
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
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 70,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 152,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 831,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 728,
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
          "id": "7ed0e0f10460a993cf14f485437834d5a803fb00",
          "message": "docs: update VERSION.md and ROADMAP.md for batch 2 features\n\n967 tests, 0 failures. Marked branding, ranged GET, notifications,\nevent triggers, WORM, remote mount, GDPR export/erasure,\ncomments, thumbnail cache as DONE.",
          "timestamp": "2026-05-30T07:01:32+01:00",
          "tree_id": "f3e3c0819267d907ff79c458c4d11a0991143fe9",
          "url": "https://github.com/WyattAu/ferro/commit/7ed0e0f10460a993cf14f485437834d5a803fb00"
        },
        "date": 1780121326617,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 233248655,
            "range": "± 833107",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 233255103,
            "range": "± 1446738",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 19472,
            "range": "± 613",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 19183,
            "range": "± 573",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 7167,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 4181,
            "range": "± 107",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 978,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 750,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2176,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1111,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 6342,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 57901,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 73,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 14740,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 727,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 57,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 53,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 117,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 137,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 140,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 660,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 568,
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
          "id": "c2eb4840c3ff3c4235021f1750dee80100e509dc",
          "message": "fix(auth): deny requests on invalid Cedar EntityUid parse\n\nPreviously, when Cedar failed to parse a principal/action/resource\nEntityUid (e.g. from crafted usernames with special characters),\nthe authorizer fell back to anonymous/unknown defaults. Since\nthe default policy permits everything including anonymous users,\nthis allowed authorization bypass.\n\nAlso fixed simple_auth granting admin access to disabled accounts:\ninactive users with matching admin credentials are now rejected\nwith 401 ACCOUNT_DISABLED.",
          "timestamp": "2026-05-30T11:34:03+01:00",
          "tree_id": "f4d099485bc3359e57880838f4e4b7a74085397d",
          "url": "https://github.com/WyattAu/ferro/commit/c2eb4840c3ff3c4235021f1750dee80100e509dc"
        },
        "date": 1780137791967,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300739472,
            "range": "± 1346052",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300736543,
            "range": "± 346919",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24349,
            "range": "± 791",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23913,
            "range": "± 4214",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9360,
            "range": "± 185",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5236,
            "range": "± 114",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1218,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 952,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2710,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1424,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8188,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74869,
            "range": "± 436",
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
            "value": 19220,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 996,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
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
            "value": 154,
            "range": "± 14",
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
            "value": 179,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 851,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 735,
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
          "id": "5e92db129e77fec535a755db183626748c3311eb",
          "message": "fix(ci): add desktop build job and fix benchmark Node.js version\n\nAdd GTK/webkit desktop build job to checks workflow for CI coverage\nof ferro-desktop crate (TD-006). Fix FORCE_JAVASCRIPT_ACTIONS_TO_NODE24\ntypo to NODE22 matching ubuntu-latest default (TD-022).",
          "timestamp": "2026-05-30T12:14:50+01:00",
          "tree_id": "a8f4bb58fc22354397d72c5e8cefe7e2dee5b1c1",
          "url": "https://github.com/WyattAu/ferro/commit/5e92db129e77fec535a755db183626748c3311eb"
        },
        "date": 1780140596140,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300827524,
            "range": "± 603846",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300817523,
            "range": "± 336011",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24461,
            "range": "± 902",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 24069,
            "range": "± 748",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9114,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5125,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1248,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 932,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2905,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1420,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8171,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74878,
            "range": "± 205",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 93,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19440,
            "range": "± 367",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 934,
            "range": "± 10",
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
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 152,
            "range": "± 4",
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
            "value": 181,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 863,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 754,
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
          "id": "92fd8e02485928aa33e6ab8392f4ebd55733a720",
          "message": "fix(auth): derive Default for E2eeConfig per clippy::derivable_impls",
          "timestamp": "2026-05-30T18:29:11+01:00",
          "tree_id": "f724faa92915df8637de26035fe5498cadd3b4a9",
          "url": "https://github.com/WyattAu/ferro/commit/92fd8e02485928aa33e6ab8392f4ebd55733a720"
        },
        "date": 1780162827845,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300682349,
            "range": "± 1318481",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300657666,
            "range": "± 786186",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 25065,
            "range": "± 1907",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 24430,
            "range": "± 2651",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9110,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5085,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1350,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 944,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2724,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1415,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8198,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74876,
            "range": "± 261",
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
            "value": 20311,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 923,
            "range": "± 9",
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
            "value": 182,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 848,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 746,
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
          "id": "b554273168514bfc55b9495f0ad33827934ce415",
          "message": "docs: update ROADMAP.md with CalDAV multiget, E2EE API, WASM ABI, ClamAV\n\n1043 tests. TD-006 partial DONE, G-11 DONE (skeleton).\nPhase 7.1 stable WASM plugin API marked DONE.",
          "timestamp": "2026-05-30T19:27:06+01:00",
          "tree_id": "2e3bf5e4e8841fbd7355e8c64c3c1b64faed7ed1",
          "url": "https://github.com/WyattAu/ferro/commit/b554273168514bfc55b9495f0ad33827934ce415"
        },
        "date": 1780166110413,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300797628,
            "range": "± 434409",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300782321,
            "range": "± 402220",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24061,
            "range": "± 672",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23931,
            "range": "± 662",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8981,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 4953,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1268,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 953,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2687,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1410,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8180,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74838,
            "range": "± 55",
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
            "value": 19217,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 971,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "exists/miss",
            "value": 68,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 159,
            "range": "± 4",
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
            "value": 181,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 867,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 754,
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
          "id": "9dc130748ae60792f491ec363a6d798a81fc2310",
          "message": "chore: audit cycle 2 — formatting, test count verification, metadata update\n\n- cargo fmt --all: fix indentation in desktop commands/gui, server dav/e2ee/lib\n- VERSION.md: correct test count (967→998), update status timestamp\n- ROADMAP.md: correct stale test counts (1043/1030→998)\n- CHANGELOG.md: add Unreleased section documenting audit findings\n- Verify: 998 tests pass, 0 clippy warnings, mdBook docs build OK\n- Pre-commit hook: fmt + clippy + tests all enforced",
          "timestamp": "2026-05-30T19:40:55+01:00",
          "tree_id": "3ff98dab3ab4eca0cad91fc082831d3158900b3e",
          "url": "https://github.com/WyattAu/ferro/commit/9dc130748ae60792f491ec363a6d798a81fc2310"
        },
        "date": 1780166950785,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300868540,
            "range": "± 399965",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 300811533,
            "range": "± 692853",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24091,
            "range": "± 857",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23520,
            "range": "± 807",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8897,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 4970,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1279,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 967,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2672,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1413,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8196,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74856,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 93,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19098,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 921,
            "range": "± 3",
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
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "head",
            "value": 157,
            "range": "± 1",
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
            "value": 859,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 720,
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
          "id": "f8b8e23ff2a7d0af3127ffcda9b8b123c53a6009",
          "message": "fix(ci): fix cargo-deny audit failure — upgrade lru 0.12->0.16, add AGPL-3.0-or-later license\n\n- Upgrade lru crate: 0.12->0.16 resolves RUSTSEC-2026-0002 soundness advisory\n  (IterMut violates Stacked Borrows rules in lru <0.16.3)\n- deny.toml: add AGPL-3.0-or-later to allowed licenses (web crate uses it)\n- cargo-deny: advisories ok, bans ok, licenses ok, sources ok\n- All 998 tests pass with lru upgrade",
          "timestamp": "2026-05-30T20:05:27+01:00",
          "tree_id": "db1d323566cac0f79d3dddc9ab07ca2666f7b16a",
          "url": "https://github.com/WyattAu/ferro/commit/f8b8e23ff2a7d0af3127ffcda9b8b123c53a6009"
        },
        "date": 1780168413531,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 266987283,
            "range": "± 235463",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 266998589,
            "range": "± 110775",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 28298,
            "range": "± 1978",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 29792,
            "range": "± 2501",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 9089,
            "range": "± 309",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 5041,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1246,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 992,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2786,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1332,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 7356,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 66506,
            "range": "± 79",
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
            "value": 18822,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 890,
            "range": "± 9",
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
            "value": 148,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_simple_path",
            "value": 178,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 222,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 838,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 849,
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
          "id": "87affc49788cdd48349eeab003d68a04561e0a01",
          "message": "feat(server): implement SMTP email and ClamAV daemon scanning\n\n- SMTP: lettre crate with STARTTLS/rustls, AUTH support, HTML+plain\n  multipart, graceful disabled-mode logging\n- ClamAV: clamd INSTREAM protocol via Unix socket, 4KB chunked\n  streaming, timeout enforcement, max file size limit\n- 8 new unit tests (1002 total, 0 failures)",
          "timestamp": "2026-05-30T22:00:11+01:00",
          "tree_id": "85ccd41ab9e43ee85f770e3b54e65066104f9751",
          "url": "https://github.com/WyattAu/ferro/commit/87affc49788cdd48349eeab003d68a04561e0a01"
        },
        "date": 1780175304700,
        "tool": "cargo",
        "benches": [
          {
            "name": "password_hash",
            "value": 300733709,
            "range": "± 576870",
            "unit": "ns/iter"
          },
          {
            "name": "password_verify",
            "value": 301065497,
            "range": "± 141923",
            "unit": "ns/iter"
          },
          {
            "name": "hmac_sha256_sign",
            "value": 24102,
            "range": "± 873",
            "unit": "ns/iter"
          },
          {
            "name": "sha256",
            "value": 23812,
            "range": "± 811",
            "unit": "ns/iter"
          },
          {
            "name": "parse_icalendar_3_components",
            "value": 8914,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "parse_vcard_complex",
            "value": 4916,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "parse_calendar_query",
            "value": 1236,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse_addressbook_query",
            "value": 965,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "build_multistatus_3_responses",
            "value": 2688,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "put/1kb",
            "value": 1413,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "put/10kb",
            "value": 8159,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "put/100kb",
            "value": 74855,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "get_10kb",
            "value": 94,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "list_100_files",
            "value": 19417,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "delete",
            "value": 914,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "exists/hit",
            "value": 74,
            "range": "± 2",
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
            "value": 177,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "normalize_traversal_path",
            "value": 179,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_serialize",
            "value": 829,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "metadata_deserialize",
            "value": 728,
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
      }
    ]
  }
}
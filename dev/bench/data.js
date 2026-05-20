window.BENCHMARK_DATA = {
  "lastUpdate": 1779290665475,
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
      }
    ]
  }
}
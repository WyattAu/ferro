window.BENCHMARK_DATA = {
  "lastUpdate": 1779240206102,
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
      }
    ]
  }
}
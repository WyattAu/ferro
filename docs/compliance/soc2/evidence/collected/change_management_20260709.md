# Change Management Evidence - 2026-07-09

## Git History (Last 30 days)
bc5e18b fix: mobile Android build (NDK env, use-after-free, import fix)
9633b32 fix: add bytes dev-dependency to security-middleware, document extraction decisions
5bdb142 fix: dependency vulnerabilities and mobile OpenSSL cross-compile
ac96468 docs: update ROADMAP to v8.1
a5cfa2e fix: CalDAV/CardDAV robustness hardening
945a0d2 docs: add penetration test scope document
e4d12d9 fix: desktop client compilation (Tauri v2 API changes)
baec75f refactor: extract photos_api to productivity crate, fix WASM frontend errors
9ea957b refactor(server): extract thumbnail_cache, api_error, offline_wiring to existing crates
23d1565 refactor(server): Phase 3 complete — slim lib.rs 2340→227 lines
995df2b refactor(server): extract 12 crates, reduce lib.rs 43%
2790ba2 docs: update ROADMAP v6.0 - Phase 1 COMPLETE
5d7ebbf feat: TrashStore + Phase 1 completion (batch 6)
761a1b9 docs: update ROADMAP v5.6 with Phase 1 batch 5 progress
99f1d37 feat: create 4 more Store structs for remaining db-only handlers (Phase 1 batch 5)
da67f9b feat: create 7 Store structs for db-only handlers (Phase 1 batch 4)
ce248b9 docs: update ROADMAP v5.5 with Phase 1 progress (21 handlers refactored)
d032b96 feat: refactor add_favorite, remove_favorite, startup to use generic traits
bcbb748 feat: refactor stream_video to use HasStorage trait
51efea0 docs: update ROADMAP v5.4 with Phase 1 progress
7c9c06b feat: Phase 1 batch 3 -- add HasQuota, HasStorageHealth traits + refactor get_quota
5fd7e4a feat: Phase 1 batch 2 -- refactor 5 more storage-only handlers
0b047f2 feat: Phase 1 batch -- refactor 10 storage-only handlers to use traits
42d8c34 docs: update ROADMAP with Phase 1 proof of concept progress
a0361b9 feat: Phase 1 proof of concept -- generic handler decomposition
c2c5efb docs: update ROADMAP v5.3 and VERSION with decomposition Phase 0 findings
aceef71 feat: Phase 0 of server crate decomposition -- composite traits
0cddef7 chore: remove dead backup.rs duplicate from server-admin
8003eb5 feat: oCIS OIDC migration support and live instance testing
9ad3463 chore: audit cycle 14 -- dead code removal, pre-commit optimization, docs
ba1ae96 fix: CalDAV/CardDAV depth calculation off-by-one in unified handlers
4c0e499 fix: production hardening and matchit 0.7.3 route compatibility
02bdeff ci: fix docs workflow Node.js version (20 to 22 for Astro 7)
48c67ef docs: update ROADMAP.md with audit cycle 13 findings
47d9eba docs: migrate from mdBook to Astro + Starlight
5d6075b docs: fix MSRV, crate counts, and phantom crate references
51fb11a fix: pre-commit hook shebang (sh to bash)
3325833 fix(landing): accessibility contrast, title formatting, crate count
24a936f ci: pin trivy-action to commit hash in release workflow
bba9e73 fix: resolve clippy warnings across workspace
92f7a12 test: GUI audit passes all viewports (desktop/mobile/tablet)
5193a74 feat: auto-update across all platforms
d569636 fix: remote server deployed, mobile app connects to localhost
e5b33e5 fix(web): reduce mobile toolbar padding, fix WASM type error
7ffb9ee fix(web): add get_server_base() for Android WebView absolute URLs
cd6f147 feat(android): working Ferro Android app with WASM frontend
ef28154 feat: parity closure execution - 29 features built, WASM fixed, Android SDK ready
33d1fd9 docs: Android build guide, dev environment setup scripts
a0ca796 feat(web): dark mode, keyboard shortcuts, search, drag-and-drop polish
9ee2612 feat(web): mail, analytics, admin, settings, navigation sidebar

## Pull Requests
31	chore(deps): bump actions/deploy-pages from 4.0.5 to 5.0.0	dependabot/github_actions/actions/deploy-pages-5.0.0	MERGED	2026-05-27T11:33:18Z
30	chore(deps): bump docker/setup-buildx-action from 3.10.0 to 4.1.0	dependabot/github_actions/docker/setup-buildx-action-4.1.0	MERGED	2026-05-27T11:33:15Z
28	chore(deps): bump actions/upload-artifact from 4.6.0 to 7.0.1	dependabot/github_actions/actions/upload-artifact-7.0.1	MERGED	2026-05-27T11:32:20Z
24	chore(deps): bump tauri from 2.11.1 to 2.11.2	dependabot/cargo/tauri-2.11.2	MERGED	2026-05-17T16:53:51Z
19	chore(deps): bump bcrypt from 0.17.1 to 0.19.1	dependabot/cargo/bcrypt-0.19.1	MERGED	2026-05-10T16:53:55Z
6	build(deps): bump softprops/action-gh-release from 2 to 3	dependabot/github_actions/softprops/action-gh-release-3	MERGED	2026-04-26T02:56:24Z
4	build(deps): bump docker/build-push-action from 5 to 7	dependabot/github_actions/docker/build-push-action-7	MERGED	2026-04-26T02:56:22Z
3	build(deps): bump actions/upload-artifact from 4 to 7	dependabot/github_actions/actions/upload-artifact-7	MERGED	2026-04-26T02:56:21Z
2	chore(deps): bump actions/checkout from 4 to 6	dependabot/github_actions/actions/checkout-6	MERGED	2026-04-26T02:56:19Z
1	build(deps): bump actions/cache from 4 to 5	dependabot/github_actions/actions/cache-5	MERGED	2026-04-26T02:56:17Z

## CI/CD Pipeline
name: Benchmarks

on:
  push:
    branches: [main]
    paths:
      - 'crates/**'
      - '.github/workflows/bench.yml'
  pull_request:
    branches: [main]
    paths:
      - 'crates/**'
      - '.github/workflows/bench.yml'

concurrency:
  group: bench-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: write

jobs:
  benchmark:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    env:
      FORCE_JAVASCRIPT_ACTIONS_TO_NODE22: true
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with:
          ref: ${{ github.event.pull_request.head.sha || github.sha }}
      
      - uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9 # v1
        with:
          toolchain: stable

      - uses: Swatinem/rust-cache@23869a5bd66c73db3c0ac40331f3206eb23791dc # v2.9.1

      - name: Install system deps
        run: sudo apt-get update && sudo apt-get install -y pkg-config libssl-dev

      - name: Run benchmarks
        run: |
          cargo bench -p ferro-benchmarks --locked -- --output-format bencher > benchmark-results.txt 2>&1 || true
          # Ensure file has content for benchmark-action
          if [ ! -s benchmark-results.txt ]; then
            echo "No benchmark output, creating placeholder"
            echo '{"name":"placeholder","unit":"ns","value":0}' > benchmark-results.txt
          fi
      
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@d48d326b4ca9ba73ca0cd0d59f108f9e02a381c7 # v1.20.4
        with:
          tool: 'cargo'
          output-file-path: benchmark-results.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: ${{ github.event_name == 'push' }}
          alert-threshold: '105%'
          comment-on-alert: true
          fail-on-alert: true
          fail-on-error: false
          alert-comment-cc-users: '@WyattAu'
          gh-pages-branch: 'bench-data'
          benchmark-data-dir-path: 'dev/bench'
name: Checks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

concurrency:
  group: checks-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  msrv:
    name: MSRV (1.92)
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
      - uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9 # v1
        with:
          toolchain: "1.92"
      - name: Install system deps
        run: sudo apt-get update && sudo apt-get install -y pkg-config libssl-dev protobuf-compiler
      - uses: Swatinem/rust-cache@23869a5bd66c73db3c0ac40331f3206eb23791dc # v2.9.1
      - name: Fetch dependencies
        run: cargo fetch --locked
      - run: cargo check --all --locked

No CI config found

# ADR-001: Complete GUI Rewrite

| Field        | Value                        |
| ------------ | ---------------------------- |
| **Title**    | Complete GUI Rewrite         |
| **Status**   | Proposed                     |
| **Date**     | 2026-07-21                   |
| **Deciders** | Wyatt (Project Lead)         |

---

## Context

### Current State

The existing Leptos 0.8 frontend consists of **16 pages** and approximately **60 components**. While functional, the codebase has accumulated structural debt that blocks meaningful progress:

| Problem                        | Impact                                           |
| ------------------------------ | ------------------------------------------------ |
| Broken CSS utility system      | Rendering bugs on multiple pages, inconsistent spacing, missing responsive behavior |
| Missing WebSocket integration  | Real-time features (chat, live collaboration, notifications) are non-functional |
| Incomplete feature utilization | ~40 of 150+ backend endpoints are consumed; >70% of server capability is untapped |
| No global state management     | Each page independently manages state — duplicated fetch logic, inconsistent loading/error states |
| No type-safe API contract      | Backend changes break frontend silently; no compile-time guarantee on request/response shapes |
| Incremental build patterns     | Inconsistent component interfaces, ad-hoc event handling, mixed imperative/reactive patterns |
| Desktop workarounds            | Tauri v2 wrapper contains platform-specific hacks that leak into shared component code |

### Backend Exposure

The backend exposes **150+ API endpoints** covering file management, collaboration, media, admin, and enterprise features. The frontend currently consumes ~40 (~27%). The gap represents significant user-facing functionality that will never ship under the current architecture.

### Root Cause Analysis

The current frontend was built incrementally over multiple months. Each page was added with its own conventions, its own CSS approach, and its own state management (or lack thereof). No design system was established upfront. No API client layer was generated — each component fetches data with ad-hoc `fetch` calls and manually parses responses. This created a system where:

1. Fixing one page's CSS breaks another
2. Adding a new page requires copy-pasting boilerplate from an existing page
3. Backend API changes require manual grep-and-fix across the frontend
4. No component can be tested in isolation

---

## Decision

**Rewrite the entire frontend from scratch** with a clean architecture, a proper design system, and type-safe API integration.

### What Changes

| Area                   | Current Approach              | New Approach                                      |
| ---------------------- | ----------------------------- | ------------------------------------------------- |
| **CSS**                | Hand-written, incomplete utilities | CSS custom properties + complete utility system (owned, no framework) |
| **State Management**   | None (per-page ad-hoc)        | Layered signal architecture: Global → Feature → Component |
| **API Layer**          | Manual `fetch` + JSON parse   | Generated type-safe client from OpenAPI/TOML spec |
| **Real-time**          | Not implemented               | WebSocket manager with auto-reconnect, message queue |
| **Component Design**   | Inconsistent interfaces       | Primitive-based design system with strict composition |
| **Offline Support**    | None                          | IndexedDB local cache with conflict resolution      |
| **Testing**            | Minimal                       | Property-based, visual regression, E2E              |
| **Accessibility**      | Ad-hoc                        | WCAG 2.1 AA baseline, keyboard-first navigation     |

### What Stays the Same

- **Rendering Engine**: Leptos 0.8 CSR — still the best Rust WASM framework for this use case
- **Backend**: No backend changes required. All 150+ endpoints remain as-is.
- **Desktop Wrapper**: Tauri v2 stays unchanged. New frontend compiles to the same `dist/` directory.
- **Design Tokens**: Colors, spacing, typography from `dark_mode.rs` are preserved.
- **Test Vectors**: Existing test data and fixtures are reused.

### Technology Stack Decision

| Option                   | Pros                                        | Cons                                            | Verdict   |
| ------------------------ | ------------------------------------------- | ----------------------------------------------- | --------- |
| **Leptos 0.8 (rewrite)** | Keep Rust stack, mature WASM, signals API   | Learning curve already paid, framework quirks   | **Selected** |
| Yew                      | Larger community, mature                    | Different signal model, less ergonomic routing   | Rejected  |
| Dioxus                    | Hot reload, multi-platform                  | Newer, less stable, smaller ecosystem           | Rejected  |
| React/Vue/Svelte         | Massive ecosystem, proven patterns          | Breaks Rust end-to-end requirement, WASM perf   | Rejected  |
| Fix current CSS only     | Smallest change                             | Component structure too coupled to broken patterns | Rejected  |

Leptos 0.8 is selected because:
1. The Rust WASM performance characteristics match the HFT-inspired latency requirements
2. Signals API provides fine-grained reactivity without virtual DOM overhead
3. Staying in Rust eliminates FFI boundaries and serialization overhead
4. The team already has institutional knowledge of Leptos patterns

---

## Alternatives Considered

### Alternative 1: Incremental Refactor

**Description**: Fix the CSS system, add global state, generate API types — all within the existing codebase.

**Rejected because**:
- The component interfaces are fundamentally inconsistent. Fixing CSS requires changing every component's class usage.
- Adding global state requires refactoring every page's data fetching pattern.
- The cost of incremental cleanup exceeds a clean rewrite when accounting for context-switching and regression risk.
- You cannot incrementally move from "no design system" to "design system" without touching every file.

### Alternative 2: React/Vue/Svelte SPA

**Description**: Rewrite in a mainstream JavaScript framework with TypeScript.

**Rejected because**:
- Breaks the Rust end-to-end stack. Serialization boundaries between Rust backend and JS frontend add complexity.
- WASM performance advantage is lost.
- Adds Node.js toolchain dependency (npm, webpack, etc.)
- Introduces two language ecosystems to maintain.

### Alternative 3: Keep Leptos, Fix CSS Only

**Description**: Maintain current component structure but rebuild the CSS utility system.

**Rejected because**:
- The CSS problems are symptoms of deeper architectural issues (no design tokens, no component contracts).
- Without fixing state management and API layer, the rewrite only addresses one-third of the problems.
- The component structure itself is coupled to the broken patterns — you end up rewriting anyway.

---

## Consequences

### Positive

| Consequence                              | Detail                                                        |
| ---------------------------------------- | ------------------------------------------------------------- |
| Clean architecture from day one          | No technical debt to carry forward                            |
| All 150+ endpoints can be utilized       | Type-safe API client makes consuming new endpoints trivial     |
| Consistent user experience               | Design system enforces visual consistency across all pages     |
| Testable components                      | Isolated components with mock API can be unit tested           |
| Real-time features ship                  | WebSocket integration built into the architecture, not bolted on |
| Offline mode becomes possible            | IndexedDB cache layer is part of the state architecture       |
| Desktop app "just works"                 | New frontend compiles to same dist/ — Tauri wrapper unchanged  |

### Negative

| Consequence                              | Mitigation                                                    |
| ---------------------------------------- | ------------------------------------------------------------- |
| 3-4 month development timeline           | Phased rollout (see GUI_REWRITE_ROADMAP.md) — ship value incrementally |
| Existing frontend code is abandoned      | Reuse design tokens, test vectors, API knowledge — no total loss |
| Temporary feature parity gap             | Phase 0-2 cover core features; advanced features follow       |
| Single developer risk                    | Document every architectural decision; maintain comprehensive tests |
| Leptos framework quirks remain           | Abstract framework-specific code behind trait boundaries where possible |

### Risks

| Risk                                     | Likelihood | Impact | Mitigation                              |
| ---------------------------------------- | ---------- | ------ | --------------------------------------- |
| Leptos 0.8 breaking changes during dev   | Low        | Medium | Pin exact version, vendor critical deps |
| Scope creep beyond 24 weeks              | High       | High   | Strict phase gates, MVP-first mindset  |
| API schema changes during frontend dev   | Medium     | Low    | Backend is stable; version API endpoints |
| WASM bundle size grows unmanageably      | Medium     | Medium | Route-based code splitting from Phase 0 |
| Accessibility requirements expand        | Low        | Medium | WCAG 2.1 AA baseline, audit every phase |

---

## Implementation Notes

### Directory Structure (Proposed)

```
ferro-frontend/
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── api/                    # Generated API client
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── types.rs
│   │   └── endpoints/
│   ├── components/
│   │   ├── primitives/         # Button, Input, Select, Dialog, etc.
│   │   ├── layout/             # Shell, Sidebar, Header, ContentArea
│   │   ├── domain/             # FileBrowser, NotesEditor, TaskBoard
│   │   └── infrastructure/     # ErrorBoundary, Suspense, WebSocketProvider
│   ├── state/
│   │   ├── global.rs           # Auth, Theme, WebSocket, Offline queue
│   │   ├── features/           # FileBrowser, Notes, Tasks state
│   │   └── server.rs           # TanStack Query-style cached server state
│   ├── styles/
│   │   ├── tokens.css          # Design tokens as CSS custom properties
│   │   ├── utilities.css       # Complete utility system
│   │   └── components/         # Component-specific styles
│   ├── hooks/                  # Custom Leptos hooks
│   ├── utils/                  # Shared utilities
│   └── routes/                 # Route definitions with LazyRoute
├── tests/
│   ├── unit/
│   ├── integration/
│   ├── e2e/
│   └── visual/
└── Cargo.toml
```

### Migration Strategy

1. **No big-bang switch** — new frontend runs alongside old until feature parity
2. **Backend serves both** — route `/v2/*` to new frontend, `/` to old
3. **Progressive cutover** — once each phase completes, migrate its routes
4. **Rollback capability** — old frontend stays deployable until Phase 7 completion

---

## References

- [GUI Architecture Specification](./GUI_ARCHITECTURE.md)
- [GUI Rewrite Roadmap](../08_roadmap/GUI_REWRITE_ROADMAP.md)
- [Current Backend API Spec](../00_requirements/API_SPECIFICATION.md)
- [Leptos 0.8 Documentation](https://book.leptos.dev/)
- [Tauri v2 Documentation](https://v2.tauri.app/)

# R9 — Fresco spike: Ferro screens on the estate UI engine

Status: CURRENT (research round R9, 2026-11-03/04)
Deliverables: [ferro-fresco-spike](https://github.com/WyattAu/ferro-fresco-spike)
(headless-verifiable screens + native live-API probe) and RESEARCH.md with
the measured 8-gap list.

## Verdict

GO for a ferro-fresco desktop client once fresco lands the reactive layer
(ADR-0001) or with a documented channel-bridge pattern. GO for Android
after the window-kit Android backend. Video playback (no decode story in
fresco/media-kit) is the largest product gap — the WebView shell remains
the video-sharing UI until a `video-kit` exists. Re-evaluate at fresco
0.4 / Phase 4 exit.

## Headline measurements

- fresco 0.3.5 renders the Ferro Connect + Browser screens headlessly;
  golden BMPs + fnv1a hashes are deterministic; the focus-chain
  interaction contract (walk -> activate -> action id) is CI-gateable.
- Native client (reqwest, no browser stack) against production:
  ranged video read `206 video/mp4` in 100 ms through Cloudflare; list
  123 ms; unauthenticated upload correctly 401.

## Gap list (fresco 0.3.5 vs Ferro needs)

1. Video playback — HIGH — needs `video-kit` (ffmpeg-kit on Android,
   decode-to-frames on desktop) or Android platform-player handoff.
2. Async data loading — HIGH — reactive layer is ADR-0001-deferred;
   channel-bridge pattern workable today but not ergonomic.
3. Virtualized file list — MEDIUM — TreeList derives visibility but
   compute walks all rows; server pagination covers 10k-folder cases.
4. Android window backend — HIGH (mobile) — winit android-activity
   unproven in fresco; ADR-0003 plans NDK.
5. Mobile IME — MEDIUM — desktop-proven; Android IME unproven.
6. Thumbnails — LOW — paint-kit image blits + media-kit decode ready.
7. OAuth/OIDC native login — MEDIUM — system-browser handshake helper
   missing (oauth-toolkit covers web-side primitives).
8. fresco API churn — MEDIUM — pre-1.0; spike goldens are the tripwire.

## Positives

- Retained model + focus chain + typed actions: headless CI-able.
- Golden-image rendering deterministic across runs.
- Same-origin proxy pattern (Android AssetServer -> localhost backend)
  composes with adb reverse for dev-loop parity.
- Estate precedent confirmed: kestrel-gui migrates Slint -> fresco; ferro
  is the named next consumer (fresco ROADMAP section 2).

## Decision

Track as: ferro adopts fresco at Phase 4 exit (GPU parity + golden
gallery) or when the video-kit gap closes — whichever first. The spike
repo is the standing tripwire: `cargo run` re-renders goldens; any fresco
API drift shows up as a compile/golden diff.

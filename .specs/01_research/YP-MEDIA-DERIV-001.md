# YP-MEDIA-DERIV-001: Media Derivatives, Thumbnails & On-Demand Streaming

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-MEDIA-DERIV-001                         |
| Domain       | Media Pipeline / Streaming                 |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/server-productivity/src/photos.rs`, `crates/server-versioning/src/lib.rs` |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| Extension-based type detection; `taken_at: None` always (EXIF struct exists, never populated); no orientation handling | `photos.rs:79-112` |
| **No thumbnails, no transcoding anywhere** — video served whole (progressive download; 4 GB home video = 4 GB egress per view) | repo-wide |
| Albums are flat `albums.json` — last-writer-wins corruption under concurrent writes | `photos.rs:62-64` |
| Every list is an O(all-files) scan: `list_all("/", 10000)` per request | `photos.rs:131-164` |
| Versioning already stores `content_hash` — exploitable for derivative keying | `server-versioning/src/lib.rs:55-76` |

---

## YP-2: Executive Summary

### Problem Statement

`photos.rs` has the metadata of a photos product and none of the pipeline. Serving video whole breaks seeking and mobile delivery; photo listing is a full-corpus scan; albums corrupt under concurrency. Transcode CPU is scarce on the shared NAS — the design must be **derivative-minimal and content-addressed**.

### Decisions

1. **Ingest sidecar metadata** (new `media` module, not inside `photos.rs`): EXIF via `kamadak-exif` once per object at ingest → populate `taken_at`, orientation, GPS; store in DB keyed by content hash; **never mutate originals** (CAS invariant).
2. **Thumbnails precomputed on ingest** (queued, not on request): presets `t160.webp`, `t320.webp`, `t1024.webp` (+ JPEG fallback), always EXIF-orientation-normalized, always GPS-stripped. Key: `deriv/{H(c ‖ preset)}` inside the existing CAS. 50 users × ~20k photos ≈ 150k derivative objects ≈ few GB — trivial for TrueNAS.
3. **Video: on-demand transcode with permanent cache** (library is long-tail; precompute burns CPU on unwatched files): first play ⇒ fMP4/CMAF + HLS VOD playlist, 3 rungs {1080p@~3 Mb/s AV1, 720p@1.2 Mb/s, 480p@600 kb/s} + AAC audio, 4 s segments. Cache key `H(content_hash, preset, encoder_ver)`; on hit serve from CAS; eviction reuses `retention.rs` semantics (`min_free_bytes` exists for exactly this). Encoder: shell out to `ffmpeg` + `libsvtav1` (`-crf 32 -preset 8 -g 96`) — do not link a Rust encoder. Plus a cheap `moov`-faststart remux cache (no re-encode) for short-clip seekability.
4. **Albums into SQLite** (flat JSON breaks under concurrent writes); **listing via index table** `photos(content_hash, path, taken_at, width, height, ...)` maintained by the ingest job — date-range queries become indexed range scans.
5. **WebRTC stays for live/screen-share only**; recorded video rides HLS (WebRTC adds signaling/TURN complexity with zero VOD benefit; LL-HLS unnecessary at 50 users).

### Guarantees

- Derivative function deterministic ⇒ cache identity sound; hits byte-identical to recomputation.
- Originals bit-identical before/after any derivative job (write-set disjointness).
- First-play latency bounded by one encode; steady-state egress ≤ min(original, requested segments).
- Derivatives leak no geolocation (GPS-stripped by construction).

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| Derivative | Secondary media object computed from a CAS original (thumbnail, transcode rung). |
| Preset | Named (resolution, codec, bitrate) tuple in the encode ladder. |
| CMAF | Common Media Application Format (ISO/IEC 23000-19): segmented fMP4. |
| Per-title ladder | Bitrate/quality ladder chosen per content on the rate-distortion convex hull (Netflix method). |
| VMAF | Perceptual video quality metric used to pick the ladder knee (~93–95). |
| Storyboard / sprite | Tiled keyframe thumbnails for scrub preview. |
| faststart | MP4 remux placing `moov` before `mdat` for progressive playback. |

---

## YP-4: Theoretical Foundation

### THM-DERIV-DETERMINISM

For content hash `c` and preset `p` with pinned encoder `(E, v)`, the derivative function `D: (c, p) → bytes` is deterministic; hence `key = H(c ‖ p ‖ v)` is sound and cache hits are exact — serving cached `D(c, p)` is byte-identical to recomputation. Encoder upgrades ⇒ new namespace; **no invalidation logic needed**.

### THM-ORIGINAL-IMMUTABILITY

The media pipeline's write set is disjoint from the CAS object set of originals; therefore `∀ D: CAS(c) = CAS₀(c)`. Enforced by construction: jobs read via content-address and write to the `deriv/` namespace only.

### THM-LADDER-PARETO

The per-content ladder chosen on the sampled convex hull (per-title method, Aaron et al. 2015) weakly Pareto-dominates any fixed ladder: `∀` rung `q` of the fixed ladder, `∃ q′`: `bitrate(q′) ≤ bitrate(q) ∧ VMAF(q′) ≥ VMAF(q)`.

### THM-CACHE-COMPLETENESS

With on-demand transcoding + persistent keyed cache, first-play latency ≤ `T_enc(dur, preset)` once per `(file, preset, encoder version)` and ≤ `T_read` for all subsequent plays; steady-state egress per view ≤ min(original size, Σ segments requested).

### THM-THUMB-PRIVACY

Derivatives generated by the recommended pipeline contain no GPS EXIF and no original timestamps ⇒ derivative objects leak no geolocation even if exposed without the metadata API (webp/jpeg encoders are steganography-free for practical purposes).

---

## YP-5: Algorithm Specification

### ALG-INGEST-001: Sidecar Metadata Extraction

```
on CAS put (image/*): EXIF via kamadak-exif → {taken_at, orientation, gps, camera}
  store row (content_hash, taken_at, width, height, orientation, has_gps)
  strip GPS from derivatives; keep Orientation in metadata (avoid double-rotation)
on CAS put (video/*): probe duration/resolution via ffprobe; register in photos index
```

### ALG-THUMB-001: Thumbnail Presets

```
presets: t160.webp, t320.webp, t1024.webp, fallback .jpg
normalize orientation before encode; key deriv/H(c ‖ preset)
queued at ingest (bounded worker pool, YP-TENANCY-ISOLATION-001 ALG-BATCHPOOL-001)
```

### ALG-VOD-001: On-Demand HLS Transcode

```
first play: ffmpeg -i master -c:v libsvtav1 -crf 32 -preset 8 -g 96 \
                 [rungs 1080p/720p/480p] -seg_duration 4 -f hls playlist.m3u8
HDR masters: tonemap to SDR for AV1 rungs (ffmpeg tonemapx); master untouched
cache: deriv/H(c ‖ preset ‖ encoder_ver); eviction via retention policy (min_free_bytes)
short clips (<60 s): moov-faststart remux cache only (no re-encode)
concurrency: 1–2 transcode jobs, niced; TrueNAS ZFS ARC pressure noted
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_infra.toml` — sections `media_*`.

Critical vectors: EXIF orientation 1–8 round trip (no double rotation); GPS absent in all derivatives (byte-scan); determinism (same input+encoder ⇒ same derivative hash); original-immutability (CAS hash unchanged after thumbnail job); HLS playlist/segment well-formedness; cache hit == recompute (byte equality); albums concurrent-write integrity (SQLite).

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_infra.toml`.

Summary: originals MUST never be mutated (`DC-MEDIA-001`); derivative keys MUST pin encoder version (`DC-MEDIA-002`); derivatives MUST be orientation-normalized and GPS-free (`DC-MEDIA-003`); photo listing MUST be index-backed (`DC-MEDIA-004`); transcode jobs MUST run in the bounded batch pool (`DC-MEDIA-005`).

---

## YP-8: Bibliography

1. IETF. **RFC 8216**: HTTP Live Streaming (Aug 2017, Informational; HLS v7; fMP4/CMAF via EXT-X-MAP). [VERIFIED]
2. **ISO/IEC 23000-19:2024** — CMAF for segmented media (2024 edition current; 2018 and 2020 editions withdrawn). [VERIFIED]
3. **ISO/IEC 14496-12:2026** — ISO Base Media File Format (2022 edition withdrawn 2026-04-22). [VERIFIED]
4. **ISO/IEC 23009-1** — DASH. Standard exists; latest edition believed 2022 (4th) — not fetched. [UNCERTAIN]
5. Aaron, A., Li, Z., Manohara, M., De Cock, J., Ronca, D. "Per-Title Encode Optimization." Netflix Technology Blog, 2015-12-14. Convex-hull/QP-ladder algorithm. **Correction: "Shakiro et al. 2015" does not exist** — the post names Aaron et al. [VERIFIED]
6. Li, Z., Aaron, A., Katsavounidis, I., Moorthy, A., Manohara, M. "Toward a Practical Perceptual Video Quality Metric" (VMAF), Netflix Tech Blog, June 2016. [UNCERTAIN — canonical intro, not re-fetched]
7. libjxl v0.12.0 (Jul 2026, signed release): decode overhaul, progressive lossless, `jxltran`; marked "for evaluation"; multiple CVEs patched. **Verdict: JPEG XL alive but not 1.0 — delivery format NO; archival lossless-JPEG transcode (`cjxl`/`djxl --reconstruct_jpeg`) only.** [VERIFIED]
8. **SVT-AV1**: repo exists; Releases page empty at fetch; 2.x line (2.0.0 Dec 2023 → 2.3.0 ≈ Dec 2024, ffmpeg `libsvtav1`) real but exact latest tag unconfirmed. [UNCERTAIN]
9. Immich docs (Oct 2026): async job queue per asset (thumbnail/metadata/transcode; NVENC/QSV/VAAPI); transcode policies; the closest self-hosted reference architecture. [VERIFIED]
10. W3C Media Source Extensions — client primitive for fMP4 playback; stable, universal. [VERIFIED as technology; edition UNCERTAIN]
11. CIPA DC-008 (EXIF): Orientation tag 0x0112 values 1–8 — the JPEG/HEIC rotation problem. [UNCERTAIN — edition not fetched]
12. ffmpeg tiled-thumbnails (`tile` filter) storyboards — documented filter; used by Jellyfin/Immich. [UNCERTAIN]
13. rav1e / av1an — active Rust AV1 encoder + orchestrator; rav1e slower than SVT at target quality; av1an batch-oriented. [UNCERTAIN — status pages not fetched]
14. Jellyfin / Nextcloud preview-generator engine behavior: on-demand transcode ladders with session cache; `occ preview:generate-all` precompute. [VERIFIED behavior / UNCERTAIN URLs]

---

## YP-9: Revision 1.1 — Hardening Pass (Iteration R2-5, 2026-10-14)

### R2-5.1: Production Reference Verified — imgproxy

**Source:** imgproxy documentation (`docs.imgproxy.net`, fetched 2026-10-14). [VERIFIED]

imgproxy (libvips-based, Evil Martians) validates the YP-5 core decisions independently: on-demand processing with signed URLs (the derivative identity = source + processing options, HMAC-signed), preset registries, **metadata stripping as a first-class OSS feature**, and pre-decode dimension checks against image bombs. Ferro's derivative keys `H(content, preset, encoder_ver)` are the CAS-native version of imgproxy's signed `(source, options)` identity — same shape, content-addressed instead of URL-addressed. **Adopt imgproxy's two additional hardening rules:** check real image dimensions before decode (image-bomb defense), and restrict allowed source/processing combinations server-side.

### R2-5.2: Album race — albums become a view over the sync op log

The `albums.json` write race disappears when album membership is not a file but a **deterministic fold over the server-sequenced op log** (YP-SYNC-CONFLICT-001 ALG-OPLOG-001): `AddToAlbum`/`RemoveFromAlbum` ops fold over the sequenced order, last-op-per-file wins. No shared mutable JSON, no lock, and sync replicas converge by construction.

**New theorems (added to scope; all proved in Lean companion):**

| ID | Statement | Formal status |
|----|-----------|---------------|
| `THM-DERIV-DETERMINISM` (quantified) | With bounded preset/encoder registries, the derivative key is injective in (content, preset, encoder version) — no cross-content derivative collisions | **Proved** — Lean `derivKey_injective` |
| `THM-NAMESPACE-DISJOINT` | Original and derivative namespaces are separated by the derivative multicodec prefix — an original address can never collide with a derivative address | **Proved** — Lean `namespaces_disjoint` |
| `THM-ORIGINAL-IMMUTABILITY` (write-set) | A derivative job structurally returns the original namespace unchanged — it cannot touch originals | **Proved** — Lean `originals_untouched` |
| `THM-THUMB-PRIVACY` (model) | Thumbnail derivatives emit an empty metadata block — no EXIF (GPS/device) can leak through a thumbnail | **Proved** — Lean `thumb_privacy` |
| `THM-ALBUM-DETERMINISM` | Album membership is a pure function of the sequenced op list; replaying any prefix on any replica yields identical membership | **Proved** — Lean `album_deterministic`, `album_prefix_compose` |

### R2-5.3: Lean Proof Companion

`formal/Ferro/Derivatives.lean` (new; imported by `Ferro.lean`). Derivative keys carry a multicodec prefix constant (`DERIV_PREFIX = 2^200`) modeling real multihash code separation; preset/encoder registries bounded to 5 bits (policy). Codec determinism (SVT-AV1/Libvips bit-exactness) remains an external assumption — what is proven is the addressing and namespace layer.

### R2-5.4: Bibliography Additions

15. imgproxy documentation — Evil Martians. On-demand processing, signed URL identity, presets, metadata stripping, image-bomb defenses. `docs.imgproxy.net`. [VERIFIED]

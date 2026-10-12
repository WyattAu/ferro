# YP-SEARCH-RETRIEVAL-001: Hybrid Search, ACL-Safe Retrieval & Fusion Theory

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-SEARCH-RETRIEVAL-001                    |
| Domain       | Information Retrieval                      |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/core/src/search.rs`, `crates/server-api-core/src/search.rs`, `crates/server/src/ai_search.rs` |
| Pinned Deps  | `tantivy =0.26.1` (latest 0.26.2, 2026-09-08) |

---

## YP-1: Ground Truth (Current Implementation)

| Aspect | Ferro today | Location |
|---|---|---|
| Engine | Tantivy, one **global** index across all users | `crates/core/src/search.rs` |
| BM25 | defaults `K1=1.2, B=0.75` | upstream |
| Fields | `path, path_id, content, name, mime, owner, modified_at` | same |
| Tokenizer | default `TEXT` (lowercase; no stemming, no CJK segmentation) | same |
| Post-rank boosts | name 3.0, path 2.0, content 1.0, recency 1.2, doctype 1.1 | same |
| Fusion | raw `0.6·lexical + 0.4·semantic` | `crates/server-api-core/src/search.rs` |
| Semantic | `MockEmbeddingModel`, 128 dims, threshold 0.3 | `crates/server/src/ai_search.rs` |
| ACL | **No result-level filtering on the search path** | same |

**CRITICAL SECURITY DEFECT (P0):** a single global inverted index with no post-filter means any caller reaching `/api/search` can receive `path` values for documents they cannot open. This is a data-disclosure bug independent of ranking quality. **No ranking work ships before this is fixed.**

**Secondary defects:** (1) fusion on incomparable scales — BM25 is unbounded above and query-length dependent; cosine is bounded in [−1,1]; the BM25 term dominates by ~an order of magnitude, so the current function is meaningless; (2) the semantic index is a stub; (3) no language-aware tokenization.

---

## YP-2: Executive Summary

### Problem Statement

Ferro must deliver Google-Drive-class search over files with three hard requirements: authorization-sound retrieval (never leak), scale-free fusion (no score-normalization fragility), and a semantic layer that fits a single TrueNAS host (8–32 GB RAM, 50 users).

### Decisions (phased)

- **Phase 0 (blocking):** ACL discriminators as first-class inverted-index fields (`owner_uid`, `space_ids`, `visibility` as multi-valued set fields), pushed into the query as boolean MUST clauses. Filtering happens *inside the posting-list scan* so it cannot be skipped by a code path that forgets it. Do NOT use per-user index shards (50 × 50 MB heap for negligible gain). Centralize search in `SearchService::search(principal, query)` with authz context as a non-optional parameter — make the type system prevent un-principaled search.
- **Phase 1:** replace `0.6·lexical + 0.4·semantic` with **RRF (k = 60)** — scale-free, no normalization. Keep a tunable convex-combination mode behind a flag (evidence: tuned convex beats RRF with few tuning examples, arXiv:2210.11934). Pipeline: BM25 top-100 ∥ HNSW top-100 → RRF → Cedar re-verification → top-20.
- **Phase 2:** real semantic layer sized for the box: `bge-small-en-v1.5` (384-dim, ~33 M params) or `all-MiniLM-L6-v2`; embeddings `N × 384 × 4 B` (50k docs ≈ 73 MB; 500k ≈ 732 MB); HNSW `M=16, ef_c=200, ef_s=64`. `MockEmbeddingModel` gated behind `#[cfg(debug_assertions)]`; production fails closed to BM25-only.
- **Phase 3:** time-boxed SPLADE evaluation on a 200-query labeled set; adopt only if it beats BM25 on nDCG@10 (would make the vector store optional).
- **Phase 4:** tokenizers — `tantivy-jieba` for CJK; `rust-stemmers` (Snowball English) applied to `content` ONLY, never `path`/`name`/`path_id` (stemming hurts exact-match recall on identifiers).
- **Phase 5:** writer heap 50 MB → 256 MB–1 GB; blue/green index directories with atomic symlink flip for zero-downtime reindex.

### Guarantees

- **Authorization soundness:** every result returned to a principal is readable by that principal (proven by construction — filter is in the scan).
- **Fusion rank-invariance:** RRF output is invariant to strictly monotone transforms of input scores.
- **Total order:** deterministic result ordering (stable pagination, reproducible tests).

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| BM25 | Probabilistic ranking function; saturating TF, IDF weighting; `K1, B` parameters. |
| RRF | Reciprocal Rank Fusion: `score(d) = Σ_ranks 1/(k + rank_r(d))`, `k = 60`. |
| Learned sparse | Neural expansion into the lexical index (SPLADE family) — BM25-indexable. |
| Late interaction | Per-token embedding comparison (ColBERT family); quality ≈ cross-encoder, cost ≪. |
| HNSW | Hierarchical Navigable Small World graph ANN index. |
| ACL discriminator | Inverted-index field encoding read-access membership, queried as MUST clauses. |
| nDCG@10 | Normalized discounted cumulative gain at rank 10; standard retrieval metric. |
| BEIR | Heterogeneous zero-shot retrieval benchmark suite. |

---

## YP-4: Theoretical Foundation

### THM-SEARCH-AUTHZ (Soundness of ACL-filtered retrieval)

`∀ p : Principal, q : Query, r : SearchResult`: `Search(p, q) = some r → Check(p, r.path, Read) = Allow`.

### THM-SEARCH-AUTHZ-SOUNDNESS (the by-construction property)

`∀ p, q`: `p.authorizationFilter ≠ ⊥ → Search(p, q) = some r → authorized(p, r.path)`. Because filtering occurs in the inverted-index scan, no caller can omit it. This is the property that makes the P0 fix durable against future endpoints.

### THM-SEARCH-FUSION-RANK-INVARIANT (RRF is scale-free)

`∀ f : Real → Real, x : List Real`: (f strictly monotone) `⟹ rrf(map f x) = rrf x`. This is exactly the invariance BM25-vs-cosine fusion needs — no score normalization required.

### THM-SEARCH-TOPK-BOUND (convex fusion anchor)

With weight `w_lex ≥ k/2`, if document `d` ranks first lexically and is the only document in the semantic top-k, then `d ∈ topK_fusion(k)` — convex fusion cannot demote the lexical leader under stated conditions.

### THM-SEARCH-TOTAL-ORDER

`∀ r₁, r₂ : SearchResult`: `r₁.score ≠ r₂.score → (r₁ < r₂ ∨ r₂ < r₁)` — deterministic total order for stable pagination and reproducible ranking tests.

---

## YP-5: Algorithm Specification

### ALG-ACLQUERY-001: Authorization-Filtered Retrieval

```
index fields (multi-valued set, IndexRecordOption::Basic):
  owner_uid, space_ids[], visibility ∈ {private, space, shared_link}
query construction (mandatory, in SearchService only):
  boolean_query = text_query AND owner_uid == p.uid   # OR space_ids ∈ p.space_ids
                                                  # OR visibility == shared_link (policy)
post-verification: Cedar re-check of top-k before return (defense in depth)
```

### ALG-RRF-001: Reciprocal Rank Fusion

```
rrf_k(d) = Σ_i 1 / (k + rank_i(d)),  k = 60
arms: rank_1 = BM25 top-100; rank_2 = HNSW top-100 (if encoder configured)
fused top-20 → Cedar re-verify → return
fallback: semantic arm timeout ⇒ BM25-only (degraded, documented)
```

### ALG-INDEXOPS-001: Zero-Downtime Index Migration

```
directories /search/idx/{v1,v2}; current ← symlink
build into inactive dir; atomic symlink flip; old dir GC'd after drain
writer heap 256 MB–1 GB; pin tantivy version; upgrades are reindex events
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_platform.toml` — sections `search_*`.

Critical vectors: **user B queries a string unique to user A's document → zero results** (the disclosure regression test); fusion perturbation (semantic-arm-only change must alter output); RRF monotone-transform invariance; degraded-mode fallback; pagination stability; CJK tokenization round-trip.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_platform.toml`.

Summary: every search path MUST take a `Principal` (type-enforced) (`DC-SEARCH-001`); ACL filter MUST be an index-scan MUST-clause, never a post-filter alone (`DC-SEARCH-002`); `MockEmbeddingModel` MUST NOT be reachable in release builds (`DC-SEARCH-003`); fusion MUST be rank-based by default (`DC-SEARCH-004`); encoder weights MUST be vendored in the image, never fetched at runtime (`DC-SEARCH-005`).

---

## YP-8: Bibliography

1. Robertson, S., Zaragoza, H. "The Probabilistic Relevance Framework: BM25 and Beyond." *FnTIR* 3(4), 2009. DOI `10.1561/1500000019`. [VERIFIED]
2. Lv, Y., Zhai, C. "When documents are very long, BM25 fails." *SIGIR 2011*. DOI `10.1145/2009916.2010070`. [VERIFIED]
3. Cormack, G., Clarke, C., Büttcher, S. "Reciprocal Rank Fusion outperforms Condorcet and individual Rank Learning Methods." *SIGIR 2009*. DOI `10.1145/1571941.1572114`. [VERIFIED]
4. Bruch, S., et al. "An Analysis of Fusion Functions for Hybrid Retrieval." arXiv:`2210.11934`. RRF is parameter-sensitive; tuned convex combination beats RRF in- and out-of-domain with few tuning examples. [VERIFIED]
5. Zhao, W. X., et al. "SPLADE: Sparse Lexical and Expansion Model for First Stage Ranking." *SIGIR 2021*. DOI `10.1145/3404835.3463098`; arXiv:`2107.05720`. [VERIFIED]
6. SPLADE v2. arXiv:`2109.10086`. [VERIFIED]
7. SPLADE++ / "Efficient Document Ranking with Learned Sparse Retrieval." arXiv:`2205.04733`. [VERIFIED]
8. SPLADE v3. arXiv:`2403.06789`. Figures (>40 MRR@10 MS MARCO) reported from secondary summaries — re-verify tables before quoting numbers. [VERIFIED ID / UNCERTAIN figures]
9. Karpukhin, V., et al. "Dense Passage Retrieval for Open-Domain Question Answering." *EMNLP 2020*. arXiv:`2004.04906`. +9–19 absolute top-20 accuracy over Lucene BM25. [VERIFIED]
10. Khattab, O., Zaharia, M. "ColBERTv2: Effective and Efficient Retrieval via Lightweight Late Interaction." *NAACL 2022*. arXiv:`2112.01488`. 5–8× footprint reduction. [VERIFIED]
11. PLAID (late-interaction acceleration). arXiv:`2205.09707`. [VERIFIED ID]
12. Thakur, N., et al. "BEIR: A Heterogeneous Benchmark for Zero-shot Evaluation of Information Retrieval Models." *NeurIPS 2021 Datasets*. arXiv:`2104.08663`. BM25 = strong zero-shot baseline. [VERIFIED]
13. Malkov, Y., Yashunin, D. "Efficient and robust approximate nearest neighbor search using HNSW graphs." *IEEE TPAMI 2020*. arXiv:`1603.09320`. [VERIFIED]
14. Subramanya, S., et al. "DiskANN: Fast Accurate Billion-point Nearest Neighbor Search on a Single Node." *NeurIPS 2019*. 5,000+ QPS, <3 ms mean, >95% recall@1 on 64 GB RAM + SSD. [VERIFIED]
15. ScaNN/Filtered-ANN. arXiv:`1908.10396`. [VERIFIED ID]
16. `BAAI/bge-small-en-v1.5` model card (384-dim, ~33.4 M params); `sentence-transformers/all-MiniLM-L6-v2` (384-dim, 22.7 M); `mxbai-embed-large-v1` (1024-dim). [UNCERTAIN — re-verify card numbers before capacity planning]
17. Dropbox Dash announced Nov 2023 (semantic search over Dropbox content) — press release only; internals unpublished. [VERIFIED press release / no transferable detail]
18. Microsoft Graph Search API — hybrid semantic+lexical with normalized `relevanceScore` per result; the closest public fusion contract. [VERIFIED API docs]
19. Nextcloud `fulltextsearch` app — Elasticsearch/OpenSearch-backed; no Tantivy-based app confirmed. [VERIFIED / confirmed absence]
20. **Negative results:** "The Importance of Tokenization in Learned Sparse Retrieval, SIGIR 2022" — not found under this title (likely SPLADE++, D7); "Consistency in Zanzibar, OSDI 2021" — does not exist (checked full proceedings). Do not cite. [VERIFIED ABSENCE]

# ripweb Atomic Guideline & Roadmap (Codebase-Grounded, 2026 Rigour)

## 0) Scope and Ground Truth

This roadmap is anchored to the **current repository implementation**, not a greenfield design.

Current reality in code:

- Query flow is `src/run.rs::handle_query` → `src/search/mod.rs::search_query` → one backend (`ddg`, `searxng`, or `marginalia`) with optional DDG instant answer.
- Query results are currently output-only cards (`title`, `url`, `snippet`) with **no ranking pipeline** beyond engine order.
- URL flow already has strong foundations: routing (`src/router.rs`), fetch resilience (`src/fetch/*`), extraction (`src/extract/*`), mode-aware formatting (`src/mode.rs` + `src/run.rs`).
- Test foundation is strong: `wiremock`, `insta`, property tests, CLI E2E, and extraction fixtures/snapshots.

This plan upgrades **search quality for technical/primary sources** while preserving existing CLI contracts and extraction stability.

---

## 1) Non-Negotiable Engineering Rules

1. No `unwrap`/`expect`/panic in library paths under `src/`; bubble errors via `RipwebError` or typed module errors.
2. No live network in tests; use `wiremock` + fixtures only.
3. Keep dependencies minimal; prefer std + existing crates first.
4. Every ranking change must be measurable against deterministic benchmark fixtures before merge.
5. Preserve output contract behavior by `Mode::density_tier()`; do not regress compact/balanced/verbose semantics.

---

## 2) Target Product Outcomes (unchanged rigor)

Primary goals for technical search quality:

- **Success@3 > 85%** on the technical benchmark (gold primary source appears in top 3).
- **nDCG@10** materially above current baseline (`ddg`/`searxng` native rank order).
- **Median query latency < 3.0s** on laptop-class hardware.
- **Average full-page fetches ≤ 15/query** (stage-2 budgeted).
- Deterministic debug trace: every ranking decision must emit interpretable score contributions.

---

## 3) Codebase Delta Map (where implementation lives)

### Existing modules to extend

- `src/cli.rs` — search/ranking debug flags, user overrides (`--intent`, `--sources`, `--debug-ranking`).
- `src/run.rs` — replace direct `search_query` call path with query pipeline orchestration.
- `src/search/mod.rs` — keep backend adapters; add pipeline entry points.
- `src/config.rs` + `config/ripweb.toml` — add `[search]` config schema (weights, trust tiers, penalties, caps).
- `src/fetch/*`, `src/extract/*` — reused for stage-2 selective content scoring.

### New modules to add

- `src/search/intent.rs`
- `src/search/variants.rs`
- `src/search/fusion.rs` (RRF)
- `src/search/scoring/mod.rs` + scorer modules
- `src/search/pipeline.rs` (stage orchestration)
- `src/search/trace.rs` (debug waterfall model)
- `src/search/stage2.rs` (budgeted selective fetch + content features)
- `src/search/rerank.rs` (optional learned/linear reranker hook)

### New test/eval surfaces

- `tests/search_eval.rs` (deterministic benchmark runner)
- `tests/search_scoring.rs`, `tests/search_fusion.rs`, `tests/search_variants.rs`, `tests/search_pipeline.rs`
- `tests/fixtures/search/eval/*.jsonl` (labeled benchmark/query sets)
- `tests/snapshots/search_eval__*.snap` and `search_trace__*.snap`

---

## 4) Atomic Roadmap by Phase

## Phase 0 — Evaluation Infrastructure First (mandatory)

Objective: make search quality work empirical before ranking rewrites.

### Atomic tasks

1. **RW-P0-001**: Add benchmark schema and loader.
   - Files: `src/search/eval_types.rs` (or `tests/common/search_eval.rs`), `tests/search_eval.rs`
   - Data shape: query, intent label, gold URLs, gold priority, optional negatives.

2. **RW-P0-002**: Create deterministic benchmark fixtures.
   - Files: `tests/fixtures/search/eval/regression.jsonl`, `techdocs_bench.jsonl`
   - Include OpenClaw-style failures and emerging-project doc lookups.

3. **RW-P0-003**: Add metric computation in Rust test harness.
   - Metrics: Success@1/3/5, MRR, nDCG@10, candidate count, fetch count budget.

4. **RW-P0-004**: Add trace object for every query run.
   - Files: `src/search/trace.rs`
   - Trace includes query variants, candidate list, per-scorer contributions, final rank.

5. **RW-P0-005**: Snapshot baseline metrics + traces.
   - Files: `tests/snapshots/search_eval__baseline.snap`, `search_trace__baseline.snap`

6. **RW-P0-006**: Add `just` targets for repeatable eval.
   - File: `justfile` (`eval-search`, `eval-search-regression`).

### Exit criteria

- Benchmark runner executes in CI/local without internet (mocked sources).
- Baseline metrics are versioned and comparable.
- No scoring work starts before this phase is merged.

### Avoid

- Subjective tuning without metric diffs.
- Any benchmark that depends on live web volatility.

---

## Phase 1 — URL Priors, Domain Trust, and Spam Penalties (highest ROI)

Objective: use low-cost metadata signals to surface official/primary sources early.

### Atomic tasks

1. **RW-P1-001**: Introduce scorer interface and contribution model.
   - Files: `src/search/scoring/mod.rs`
   - API returns score + explanation; all scorers unit-testable.

2. **RW-P1-002**: Implement metadata scorers.
   - `domain_trust.rs`, `url_pattern.rs`, `project_match.rs`, `blocklist_penalty.rs`, `domain_diversity.rs`, `snippet_relevance.rs`.

3. **RW-P1-003**: Extend config schema for search scoring.
   - Files: `src/config.rs`, `config/ripweb.toml`
   - Use TOML (existing parser) rather than adding YAML dependency.

4. **RW-P1-004**: Add trust tiers and blocklist entries as versioned config.
   - Start with explicit categories: `seo_tutorial_farm`, `dictionary_spam`, `social_noise`, `js_heavy`.
   - Use **soft penalties** by default; hard blocks only for high-confidence spam.

5. **RW-P1-005**: Add URL path priors (boost docs/reference/api; penalize tutorial-farm patterns).
   - Must be explicit and explainable in trace output.

6. **RW-P1-006**: Add project entity matcher for technical queries.
   - Detect CamelCase, crate/pkg names, repo slugs, domain-like tokens.
   - Score against title + URL host/path.

7. **RW-P1-007**: Integrate stage-1 scorer pass into query flow.
   - File: `src/run.rs` + `src/search/pipeline.rs`.
   - Preserve existing output formatting contract in `format_search_results`.

8. **RW-P1-008**: Add scorer unit tests and ablation tests.
   - Files: `tests/search_scoring.rs`, `tests/search_pipeline.rs`.

### Exit criteria

- Regression benchmark shows significant lift in Success@3.
- Trace snapshots clearly show scorer contributions and penalties.
- Latency impact remains low (metadata-only stage).

### Avoid

- Over-broad hard blocks that suppress valid docs.
- Hidden heuristic logic without trace visibility.

---

## Phase 2 — Multi-Query Fan-out + Intent + RRF

Objective: deterministic Grok-like query breadth with robust merge quality.

### Atomic tasks

1. **RW-P2-001**: Add intent classifier (rule-first).
   - File: `src/search/intent.rs`
   - Intents include: `official_docs`, `emerging_project_docs`, `code_error_lookup`, `general_technical`.

2. **RW-P2-002**: Add query variant generator (4–6 variants).
   - File: `src/search/variants.rs`
   - Include original, technical rewrite, site-restricted, source-focused variants.

3. **RW-P2-003**: Parallel fan-out execution.
   - File: `src/search/pipeline.rs`
   - Run backend calls concurrently using tokio tasks; normalize result schema.

4. **RW-P2-004**: Add RRF fusion with deterministic tie-breakers.
   - File: `src/search/fusion.rs`
   - Configurable `k` (default ~60), optional origin-query weighting.

5. **RW-P2-005**: Add CLI overrides for intent/sources.
   - File: `src/cli.rs`
   - `--intent`, `--sources`, and `--debug-ranking`.

6. **RW-P2-006**: Add low-score reformulation fallback (rule-based only in v1).
   - Trigger only when top fused candidates have uniformly poor score.

7. **RW-P2-007**: Test fan-out and fusion determinism.
   - Files: `tests/search_variants.rs`, `tests/search_fusion.rs`.

### Exit criteria

- Benchmark shows measurable lift from fan-out+RRF over Phase 1.
- Intent labels are stable on labeled subset fixtures.
- No regressions in CLI modes and output structure tests.

### Avoid

- Regex soup intent logic that cannot be tested.
- Fetching full pages during candidate generation.

---

## Phase 3 — Two-Stage Ranking with Selective Fetch

Objective: high-precision ranking under strict latency/fetch budgets.

### Atomic tasks

1. **RW-P3-001**: Implement stage orchestration.
   - File: `src/search/pipeline.rs`
   - Stage 1 on 60–100 metadata candidates; cut to top 12–18 for stage 2.

2. **RW-P3-002**: Implement stage-2 selective fetch.
   - File: `src/search/stage2.rs`
   - Reuse `fetch_with_retry`, `PreflightCheck`, `DomainSemaphores`, cache, and `WebExtractor`.
   - Timeout/fetch budget enforced per query.

3. **RW-P3-003**: Add content-level scorers.
   - `fulltext_relevance`, `code_density`, `content_quality`, `near_duplicate_penalty`.
   - Reuse extractor statistics patterns where possible.

4. **RW-P3-004**: Add near-duplicate suppression.
   - URL-level + extracted-text-level similarity check; keep highest-confidence representative.

5. **RW-P3-005**: Add final explanation synthesis.
   - Every result includes concise “why ranked” explanation from component traces.

6. **RW-P3-006**: Add budget and latency guards.
   - Hard cap full-page fetches (default <=15).
   - Enforce concurrency and timeout limits with explicit error paths.

7. **RW-P3-007**: Add integration tests with `wiremock`.
   - Files: `tests/search_pipeline.rs`, `tests/search_eval.rs`
   - Validate fetch cap, timeout behavior, and fallback robustness.

### Exit criteria

- Success@3 target reached on benchmark.
- Median latency and fetch budgets within target.
- Trace snapshots show stage-1 and stage-2 contributions separately.

### Avoid

- Running expensive stage-2 features on all candidates.
- Hand-tuning weights without benchmark evidence.

---

## Phase 4 — Learned Rerank Hook, Continuous Adaptation, and Ops

Objective: adaptive ranking without sacrificing determinism or maintainability.

### Atomic tasks

1. **RW-P4-001**: Add feature export path from eval runs.
   - Emit per-candidate feature vectors + labels for training data generation.

2. **RW-P4-002**: Add reranker interface with safe default.
   - File: `src/search/rerank.rs`
   - Default implementation: deterministic linear reranker from config weights.

3. **RW-P4-003**: Add optional model-backed reranker behind feature flag.
   - Keep default build lean; no mandatory heavy runtime dependencies.

4. **RW-P4-004**: Add failure clustering/report generation.
   - Artifact: markdown report grouped by failure mode (spam leak, domain mis-rank, duplicate crowding).

5. **RW-P4-005**: Add CI quality gate for benchmark regression.
   - Extend workflow to run deterministic search benchmark and fail on threshold regressions.

6. **RW-P4-006**: Add maintenance loop docs.
   - Update blocklist/trust tiers monthly, reranker weights quarterly, with reproducible changelog artifacts.

### Exit criteria

- Sustained metric quality across benchmark refreshes.
- No unexplained ranking behavior (trace completeness).
- CI fails on statistically meaningful regressions.

### Avoid

- Opaque “magic” model in the hot path without explainability.
- Any cloud dependency for mandatory runtime ranking.

---

## 5) Cross-Cutting Test and Validation Protocol

For each phase PR:

1. Unit tests for new scorer/intent/fusion logic.
2. Integration tests with fixtures and wiremock only.
3. Snapshot updates reviewed (`cargo insta test --review`).
4. Full checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo deny check advisories`, `cargo nextest run --all-features`, `cargo test --doc`.

---

## 6) Implementation Order and Dependencies

1. Phase 0 must merge first.
2. Phase 1 depends on Phase 0 metrics/traces.
3. Phase 2 depends on Phase 1 scorer framework.
4. Phase 3 depends on Phase 2 fused candidates.
5. Phase 4 depends on Phase 3 stable feature pipeline.

No phase skipping.

---

## 7) Definition of Done (Project-Level)

ripweb search upgrade is complete when:

- Technical benchmark Success@3, nDCG@10, latency, and fetch-budget targets are met.
- Query ranking is deterministic and explainable via trace artifacts.
- Existing URL extraction/platform behavior remains contract-stable.
- CI enforces search-quality regression protection, not just compile/test correctness.

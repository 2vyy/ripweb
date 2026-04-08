# Implementation Plan - Improve generic content selection heuristics and implement initial page-family detection

## Phase 1: Research and Baseline (Refinement)
- [ ] Task: Analyze current extraction failures in `tests/fixtures/torture/`
- [ ] Task: Document current scoring heuristics and identified weaknesses in `docs/EXTRACTION.md`
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Research and Baseline (Refinement)' (Protocol in workflow.md)

## Phase 2: Enhanced Content Selection Heuristics
- [ ] Task: Refine link-to-text ratio scoring in `src/extract/candidate.rs`
- [ ] Task: Implement depth-based penalization for deeply nested boilerplate in `src/extract/candidate.rs`
- [ ] Task: Add boost for high-density text nodes with semantic markers (e.g., `<article>`, `<main>`)
- [ ] Task: Update existing snapshots in `tests/snapshots/` to reflect improved extraction
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Enhanced Content Selection Heuristics' (Protocol in workflow.md)

## Phase 3: Page-Family Detection System
- [ ] Task: Create `src/extract/family.rs` with `PageFamily` enum (Article, Product, Docs, Listing, Search, Forum)
- [ ] Task: Implement `detect_family` function using meta tags (OpenGraph, Schema.org) and DOM hints
- [ ] Task: Implement URL-based family hints (e.g., `/docs/`, `/wiki/`, `/p/`)
- [ ] Task: Integrate family detection into the extraction pipeline in `src/extract/mod.rs`
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Page-Family Detection System' (Protocol in workflow.md)

## Phase 4: Initial Family-Specific Rendering
- [ ] Task: Implement specialized rendering rules for `Docs` family (e.g., preserve sidebar navigation if relevant, or strip more aggressively)
- [ ] Task: Add integration tests in `tests/extract_web.rs` for `Docs` family detection and extraction
- [ ] Task: Verify that generic extraction still works correctly for non-classified pages
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Initial Family-Specific Rendering' (Protocol in workflow.md)

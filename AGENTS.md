# Agents Guide

This file contains the foundational principles and critical instructions for working on the `ripweb` codebase. This repository adheres to **Professional 2026 Rust Standards**. Please read these instructions carefully before making any tool calls or codebase modifications.

## 1. Core Philosophy: Token Optimization & Verbosity
`ripweb` is an api-keyless Unix pipe tool designed to harvest web content into highly efficient Markdown for LLMs. 
- **The primary goal is to maximize the signal-to-noise ratio in the output.**
- Always consider how your code changes impact the final token count.
- When writing prompts or post-processing heuristics, optimize for brevity. 
- You must honor the 3-tier `verbosity` model (V1: Links/Discovery, V2: Snippets/Signal, V3: Full Context).

## 2. Professional Rust Standards

### A. Strict Error Handling
- **DO NOT** use `unwrap()`, `expect()`, or `panic!` anywhere in the library code (`src/`). All failures must be gracefully caught and bubbled up using the `RipwebError` enum. 
- Unsafe code is strictly forbidden unless absolutely necessary (which is essentially never for this tool).

### B. Linting and Formatting
- All code must pass `cargo clippy --all-targets --all-features -- -D warnings`. Do not ignore Clippy warnings.
- All code must be systematically formatted. If you create or edit Rust files, assume `cargo fmt` standards.

### C. Dependency Management
- Keep dependencies ruthlessly minimal. 
- Do not introduce bloated crates. Prioritize standard library features or lightweight alternatives. 

## 3. Testing Paradigm (Snapshot & Mocking)

### A. Snapshot Testing (`insta`)
- Manual golden file comparisons are banned.
- All extraction pipeline tests must use the `insta` crate. 
- When updating parsing logic, rely on `cargo insta test --review` to update the `.snap` files.
- Snapshot `.snap` files must be committed alongside your code changes.

### B. Network Mocking (`wiremock`)
- Live internet calls in tests are flaky and prohibited in the core test suite.
- Use `wiremock` to intercept URL requests and serve frozen HTML/JSON HTTP responses from `tests/fixtures/`.

### C. Deterministic Evaluation
- LLMs are stochastic. If you add logic that uses LLMs (e.g., local ollama integration), testing must mock the LLM response. Keep the core logic pipeline deterministic.

## 4. Development Workflow Tools

### A. The `justfile`
- We use `just` as our command runner.
- Before committing, you should mentally (or literally, via `run_command`) verify your work against the test suite.
- Use `cargo nextest run` if optimizing for test execution speed.

### B. Continuous Integration (.github/workflows)
- CI enforces formatting, clippy lints, vulnerability scanning (`cargo-deny`), and tests. 
- If you modify `.github/workflows/`, ensure that aggressive caching (`Swatinem/rust-cache`) and modern matrix strategies are preserved.

## 5. Security & Privacy
- `ripweb` impersonates browsers to bypass anti-bot walls (using `rquest`). 
- Do not introduce web telemetry or analytics.
- If writing code to handle user inputs or custom parameters, sanitize them properly.

## Summary Checklist for Agents
1. Did I increase the output token count? If yes, is the added context strictly necessary for V3?
2. Did I use `.unwrap()`? (Remove it!)
3. Are there `insta` snapshots covering this new extraction edge-case?
4. Is this network test using `wiremock` rather than hitting the live internet?

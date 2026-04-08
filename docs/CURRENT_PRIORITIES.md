# Current Priorities

This page tracks what is done, what is next, and the known weak spots to target. It is intentionally lighter than a full project tracker.

---

## Current State

- Output contract is formalized in [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md)
- Generic web extraction is Markdown-first with family-aware post-processing (Forums, Docs sidebars)
- High-efficiency platform extractors implemented for **Wikipedia, ArXiv, StackOverflow, GitHub, Reddit, and HackerNews**
- Automated **Probe Sequence** (`.md` -> `llms.txt`) avoids HTML scraping on compliant sites
- **Jina.ai Reader** integrated as a high-fidelity universal fallback
- `proptest` ensures minifier idempotency and length invariants
- Frozen corpus and bulk evaluation workflow are fully operational
- Performance benchmarks load local corpus at runtime for CI stability

---

## Immediate Next Steps

1. Implement **v0.6 Scale & Evaluation**: add structural fidelity metrics and automated golden generation
2. Expand platform support to **YouTube (metadata/transcripts)** and **Amazon (product specs)**
3. Improve generic **Listing** family extraction (result-card deduplication)
4. Audit memory usage during large bulk crawls

---

## v0.4 Foundation (Complete)

- [x] Formalize output contract
- [x] Markdown-first extraction as the primary mode
- [x] Frozen corpus and bulk evaluation workflow
- [x] Better generic content selection heuristics (depth penalization, link density)
- [x] Page-family aware extraction (Docs, Listing, Product, Forum)
- [x] Snapshot-based regression testing (`insta`) and HTML fixtures
- [x] Family-aware post-processing (Forum ranking, sidebar stripping)
- [x] End-to-end benchmarks loading local corpus at runtime
- [x] Invariant validation (output length, link integrity)
- [x] `proptest` for minifier idempotency

---

## v0.5 Platform Expansion (Complete)

- [x] **Wikipedia**: REST v1 summary API
- [x] **ArXiv**: Atom export metadata API
- [x] **StackOverflow**: SE API v2.3 with answer ranking
- [x] **Reddit**: `.json` API for structured threads
- [x] **HackerNews**: Algolia API for item retrieval
- [x] **GitHub**: REST API for Issues/Comments/READMEs
- [x] **Probe Sequence**: Support for `.md` and `llms.txt`
- [x] **Jina Fallback**: Integrated `r.jina.ai` proxy

---

## Known Weak Spots

- generic extraction heuristics are still the biggest quality risk
- the parser does not yet reason explicitly about all page families
- golden and benchmark workflows are too manual
- platform support is thin relative to the product ambition
- evaluation currently measures "contains some words" rather than "good Markdown extraction for LLM use"

---

## Research Track

- analyze comparable tools (Readability, jusText, Trafilatura, goose) and compare output contracts
- research smarter parsing approaches before adding more heuristics
- capture findings in [EXTRACTION.md](EXTRACTION.md)
- revisit token optimization experiments only after the Markdown path is stable


potential token killer format:
include all headers
include all data in first header section
include all codeblocks (remove empty lines)
keywords for each header section of all highlighted / bolded / tick marked (``) words
dictionary of all hyperlinked text to it's link
include the first sentence of paragraphs?
problems:
Duplicate links across sections - probably dedupe globally.
Should code blocks note their language tag? - yes if available

Crate insta

What are snapshot tests

Snapshots tests (also sometimes called approval tests) are tests that assert values against a reference value (the snapshot). This is similar to how assert_eq! lets you compare a value against a reference value but unlike simple string assertions, snapshot tests let you test against complex values and come with comprehensive tools to review changes.
Snapshot tests are particularly useful if your reference values are very large or change often.
asert_eq!: doc.rust-lang.org/nightly/core/macro.assert_eq.html

What it looks like:
Where are the snapshots stored? Right next to your test in a folder called snapshots as individual .snap files.
```rust
#[test]
fn test_hello_world() {
    insta::assert_debug_snapshot!(vec![1, 2, 3]);
}
```

keywords: snapshots

.snap files = insta.rs/docs/snapshot-files
Read the introduction = insta.rs/docs/quickstart
Read the main documentation = insta.rs/docs
watch the insta introduction screen = youtube.com/watch?v=rCHrMqE4JOY

Writing Tests

```rust
use insta::assert_debug_snapshot;
#[test]
fn test_snapshots() {
    assert_debug_snapshot!(vec![1, 2, 3]);
}
```
The recommended flow is to run the tests once, have them fail and check if the result is okay.
```
$ cargo test
$ cargo insta review
```

keywords = .new, cargo insta review

cargo-insta = crates.io/crates/cargo-insta

Use Without cargo-insta
Note that cargo-insta is entirely optional.
```
INSTA_UPDATE=no cargo test
INSTA_UPDATE=always cargo test
```
keywords = cargo-insta, cargo test, INSTA_UPDATE
Updating snapshots = #updating-snapshots (***NOTE***: this would be redirected from it's own link which was docs.rs/insta/latest/insta/#updating-snapshots)

Assertion Macros
This crate exports multiple macros for snapshot testing:
keywords: csv, toml, yaml, ron, json
assert_snapshot! = docs.rs/insta/latest/insta/macro.assert_snapshot.html
Display = doc.rust-lang.org/nightly/core/fmt/trait.Display.html
assert_debug_snapshot! = docs.rs/insta/latest/insta/macro.assert_debug_snapshot.html
Debug = doc.rust-lang.org/nightly/core/fmt/macros/derive.Debug.html
serde::Serialize = docs.rs/serde_core/1.0.228/x86_64-unknown-linux-gnu/serde_core/ser/trait.Serialize.html
assert_csv_snapshot! = docs.rs/insta/latest/insta/macro.assert_csv_snapshot.html
assert_toml_snapshot! = docs.rs/insta/latest/insta/macro.assert_toml_snapshot.html
assert_yaml_snapshot! = docs.rs/insta/latest/insta/macro.assert_yaml_snapshot.html
assert_ron_snapshot! = docs.rs/insta/latest/insta/macro.assert_ron_snapshot.html
assert_json_snapshot! = docs.rs/insta/latest/insta/macro.assert_json_snapshot.html
assert_compact_json_snapshot! = docs.rs/insta/latest/insta/macro.assert_compact_json_snapshot.html
serde = docs.rs/serde/1.0.228/x86_64-unknown-linux-gnu/serde/index.html
redactions in the documentation = insta.rs/docs/redactions

Updating snapshots
During test runs snapshots will be updated according to the INSTA_UPDATE environment variable. 
```
$ cargo insta review
```
keywords: INSTA_UPDATE, auto, .snap.new, no, new, always, .snap, unseen, force

cargo-insta: crates.io/crates/cargo-insta
read the cargo insta docs: insta.rs/docs/cli


...




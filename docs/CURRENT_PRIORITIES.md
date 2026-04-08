# Current Priorities

This page tracks what is done, what is next, and the known weak spots to target. It is intentionally lighter than a full project tracker.

---

## Current State

- output contract is formalized in [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md)
- generic web extraction is Markdown-first with basic candidate scoring
- curated vs generated evaluation is separated
- seed URL import, freeze review, frozen fixture workflow, and bulk parser reports are in place
- first frozen real-world batch and tokenizer audit are complete
- the main product gap is extraction quality across different page families, not infrastructure

---

## Immediate Next Steps

1. Improve generic content selection heuristics (better density and link scoring)
2. Add explicit page-family detection on top of the selected candidate
3. Add `Docs` family rendering rules (stronger sidebar stripping)
4. Add `Listing` and `Search` family detection and rendering rules
5. Add `Product` family detection and rendering rules
6. Add `Forum` / `Discussion` family detection and rendering rules
7. Revisit aggressive mode only after the Markdown path is stable

---

## v0.4 Foundation

- [x] Formalize output contract
- [x] Markdown-first extraction as the primary mode
- [x] Frozen corpus and bulk evaluation workflow
- [ ] Better generic content selection heuristics
- [ ] Page-family aware extraction for all planned families
- [ ] Less manual golden workflow (scripted generation)
- [ ] End-to-end benchmarks (fetch → extract → render on real corpora)
- [ ] Metrics for content selection quality, signal retention, structural fidelity

---

## v0.5 Platform Expansion

Add site-specific extractors in this order:

1. `wikipedia.org`
2. `arxiv.org`
3. `youtube.com`
4. `reddit.com` improvements
5. `github.com` improvements
6. `x.com` / `amazon.com`

Each new extractor ships with: routing, output contract, fixtures, snapshot/golden coverage, no live-network tests.

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
include all codeblocks (remove empty lines
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




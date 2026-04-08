# Configuration

`ripweb` now uses a single project config file:

- `config/ripweb.toml`

This keeps the project easier to reason about than scattering configuration across multiple internal modules.

## Why One File

Right now the repo is still one crate with internal modules, not a true multi-crate workspace.

That makes one top-level config the better fit:

- easier to discover
- easier to review
- easier to document
- still easy to organize by subsystem using sections

If the project later becomes a real workspace, we can keep one user-facing config and subdivide it internally by section.

## Current Shape

The current config is intentionally small and focused on extraction.

### `[extract]`

Top-level extraction settings.

Current field:

- `default_family`

### `[extract.domain_exact]`

Exact host-to-family mappings.

Examples:

- `docs.rs = "docs"`
- `developer.mozilla.org = "docs"`
- `amazon.com = "product"`
- `walmart.com = "product"`
- `bestbuy.com = "product"`

### `[[extract.suffix_rules]]`

Suffix-based family hints for domain clusters.

Examples:

- `*.readthedocs.io -> docs`
- `*.github.io -> docs`

## How It Is Used

The extractor remains heuristic-first, but URL/domain hints are now consulted before generic family heuristics when a source URL is available.

That means:

- crawler-based extraction can use domain-family hints
- direct fixture tests that call the raw extractor without a URL still exercise generic heuristics

This keeps the generic extractor testable without making everything depend on URL context.

## Current Limitation

At the moment, config-backed family hints influence the families the extractor actually understands:

- `docs`
- `article`
- `product`
- fallback `generic`

Entries like `forum` or `knowledge` are still future-facing metadata and will not change extraction behavior until those families are implemented in the parser.

## Editing Guidance

Use the config for:

- common, obvious site-family hints
- stable domain clusters
- behavior reviewers should be able to discover quickly
- broad page-family routing like `docs` or `product`

Do not use it yet for:

- large brittle CSS-selector rule sets
- per-site scraping logic
- behavior that belongs in proper parser code instead of a lookup table

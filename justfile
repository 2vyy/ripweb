# Default task to run on just
default: check

# Check formatting, lints, and tests
check: format lint test

# Run tests (can be swapped to cargo-nextest later)
test:
    cargo test --all-features
    cargo test --doc
    cargo insta test

# Format code
format:
    cargo fmt --all

# Run clippy with strict warnings
lint:
    cargo clippy --all-targets --all-features -- -D warnings
    cargo deny check

# Update insta snapshots interactively
update-snapshots:
    cargo insta test --review

# Run criterion token efficiency benchmarks
bench:
    cargo bench

# Remove unused dependencies and clean up
prune:
    cargo machete

# Run search quality evaluation against all benchmark fixtures
eval-search:
    cargo test --test search_eval -- --nocapture

# Run search eval in CI mode — fails if snapshots changed unexpectedly
eval-search-regression:
    INSTA_UPDATE=unseen cargo test --test search_eval

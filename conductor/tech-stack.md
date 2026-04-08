# Technology Stack: ripweb

This document outlines the core technology stack and dependencies for `ripweb`.

## Core Language & Runtime
- **Rust (Edition 2024)**: Memory-safe, high-performance systems language.
- **Tokio**: Asynchronous runtime for high-concurrency network tasks.

## Network & HTTP
- **rquest**: Advanced HTTP client with TLS/JA3/JA4 fingerprint impersonation for robust fetching.
- **url**: Comprehensive URL manipulation and parsing.

## Extraction & Parsing
- **tl**: Fast, dependency-efficient HTML parser for DOM manipulation.
- **encoding_rs**: Robust charset transcoding (Shift-JIS/ISO-8859-1).

## CLI & Interfaces
- **Clap**: Powerful CLI argument parsing with `derive` macros.
- **Tracing / Tracing-Subscriber**: Structured logging and diagnostics.
- **indicatif**: Rich progress reporting and status indicators.
- **is-terminal**: TTY detection for environment-aware output.
- **arboard**: Clipboard interaction for CLI-to-clipboard workflows.

## Data & Configuration
- **Serde / Serde JSON / TOML**: Universal serialization and configuration formats.
- **directories**: Cross-platform XDG config/cache management.
- **tiktoken-rs**: BPE token counting for LLM context estimation.

## Testing & Benchmarks
- **insta**: Snapshot testing for extraction pipeline integrity.
- **criterion**: Micro-benchmarking for the minification state machine.
- **wiremock**: Mock HTTP servers for reliable network layer tests.
- **proptest**: Property-based testing for high-assurance code.

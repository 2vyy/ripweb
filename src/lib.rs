//! ripweb: High-efficiency, privacy-respecting Unix pipe for hauling web content into Markdown for LLMs.
//!
//! This crate provides the core orchestration, routing, extraction, and minification
//! logic for the ripweb CLI. It exposes high-level fetcher traits and platform-specific
//! search modules.

pub mod cli;
pub mod cli_utils;
pub mod config;
pub mod error;
pub mod extract;
pub mod fetch;
pub mod minify;
pub mod research;
pub mod router;
pub mod run;
pub mod search;
pub mod verbosity;

pub use error::RipwebError;

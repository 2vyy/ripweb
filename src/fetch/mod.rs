//! Fetch Orchestration
//!
//! Pulls together caching, client impersonation, crawling, and
//! safety preflights into a unified fetching interface.

pub mod cache;
pub mod client;
pub mod crawler;
pub mod error;
pub mod llms_txt;
pub mod normalize;
pub mod politeness;
pub mod preflight;
pub mod probe;

pub use client::{FetchError, RetryConfig};

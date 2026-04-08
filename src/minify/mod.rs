//! Token Efficiency & Minification
//!
//! Provides utilities for reducing the string footprint of extracted
//! content, including whitespace collapsing and URL parameter stripping.

mod state_machine;
mod urls;

pub use state_machine::collapse;
pub use urls::strip_tracking;

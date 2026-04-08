//! HTML Extraction Orchestration
//!
//! Pulls together the candidate scorer, boilerplate nuke-list,
//! and markdown renderer to produce clean text from raw HTML bytes.

pub mod boilerplate;
pub mod candidate;
pub mod family;
pub mod jina;
pub mod links;
pub mod postprocess;
pub mod render;
pub mod web;

use crate::error::RipwebError;

pub trait Extractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError>;
}

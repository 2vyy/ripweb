pub mod boilerplate;
pub mod family;
pub mod links;
pub mod web;

use crate::error::RipwebError;

/// Core extraction interface. Each implementor handles a specific site type.
/// Receives raw network bytes plus the HTTP Content-Type header (if any) so
/// that charset decoding happens inside the extractor before DOM parsing.
pub trait Extractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError>;
}

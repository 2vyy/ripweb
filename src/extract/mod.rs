pub mod boilerplate;
pub mod family;
pub mod links;
pub mod render;
pub mod web;

use crate::error::RipwebError;

pub trait Extractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError>;
}

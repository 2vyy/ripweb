//! Safety Preflights
//!
//! Validates `Content-Length` and `Content-Type` before downloading
//! bodies to avoid large files (PDFs, ZIPs) or memory exhaustion.

/// Hard ceiling on response body size (5 MiB).  Responses that declare a
/// `Content-Length` exceeding this are rejected before the body is streamed,
/// preventing OOM panics from rogue binary links.
pub const MAX_PAGE_SIZE: u64 = 5 * 1024 * 1024;

/// Errors surfaced by a failed pre-flight check.
#[derive(Debug)]
pub enum PreflightError {
    /// No `Content-Type` header was present — we cannot safely decide to parse.
    MissingContentType,
    /// The MIME type is not a text format we can extract from.
    NonTextMime(String),
    /// `Content-Length` exceeds [`MAX_PAGE_SIZE`].
    TooLarge(u64),
}

impl std::fmt::Display for PreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingContentType => write!(f, "missing Content-Type header"),
            Self::NonTextMime(m) => write!(f, "non-text MIME type: {m}"),
            Self::TooLarge(n) => write!(f, "Content-Length {n} exceeds {MAX_PAGE_SIZE} byte limit"),
        }
    }
}

/// Stateless pre-flight validator.  Call [`PreflightCheck::validate`] with the
/// response headers before streaming the body.
pub struct PreflightCheck;

impl PreflightCheck {
    /// Validate `Content-Type` and `Content-Length` response headers.
    ///
    /// * `content_type` — the raw `Content-Type` header value, if present.
    /// * `content_length` — the parsed `Content-Length` in bytes, if present.
    ///
    /// Returns `Ok(())` when the response is safe to download, or a
    /// [`PreflightError`] that describes why it was rejected.
    pub fn validate(
        content_type: Option<&str>,
        content_length: Option<u64>,
    ) -> Result<(), PreflightError> {
        let ct = content_type.ok_or(PreflightError::MissingContentType)?;

        // Extract just the MIME type, ignoring parameters like `; charset=…`
        let mime = ct.split(';').next().unwrap_or(ct).trim();

        if !is_text_mime(mime) {
            return Err(PreflightError::NonTextMime(mime.to_owned()));
        }

        if let Some(len) = content_length
            && len > MAX_PAGE_SIZE
        {
            return Err(PreflightError::TooLarge(len));
        }

        Ok(())
    }
}

/// Returns `true` for MIME types we can meaningfully extract text from.
fn is_text_mime(mime: &str) -> bool {
    // Allow anything under the `text/` top-level type, plus JSON and XML
    // application sub-types that may contain extractable content.
    mime.starts_with("text/")
        || mime == "application/json"
        || mime == "application/xml"
        || mime == "application/xhtml+xml"
}

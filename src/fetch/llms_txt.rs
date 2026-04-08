use url::Url;

const CANDIDATE_PATHS: &[&str] = &["/llms.txt", "/.well-known/llms.txt"];

/// Attempt to retrieve an `llms.txt` file for the given URL's origin.
///
/// Tries `<origin>/llms.txt` first, then `<origin>/.well-known/llms.txt`.
/// Returns the body text on HTTP 200, or `None` on any non-200 or error.
///
/// This is intentionally lightweight — no retry, no preflight — because a 404
/// is the expected common case and the file is always small plain text.
pub async fn fetch_llms_txt(client: &rquest::Client, origin: &Url) -> Option<String> {
    for candidate in CANDIDATE_PATHS {
        let url = origin.join(candidate).ok()?;
        if let Some(body) = try_fetch(client, url.as_str()).await {
            return Some(body);
        }
    }
    None
}

async fn try_fetch(client: &rquest::Client, url: &str) -> Option<String> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.text().await.ok()
}

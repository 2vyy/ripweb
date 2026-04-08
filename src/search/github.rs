use url::Url;

#[derive(Debug)]
#[non_exhaustive]
pub enum GithubError {
    Network(rquest::Error),
    NotFound,
    Utf8,
}

impl std::fmt::Display for GithubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::NotFound => write!(f, "README not found (tried HEAD branch)"),
            Self::Utf8 => write!(f, "README body is not valid UTF-8"),
        }
    }
}

/// Build the raw.githubusercontent.com URL for a repo's README.
///
/// Uses the `HEAD` ref so it works regardless of whether the default branch
/// is named `main`, `master`, or anything else.
pub fn github_raw_url(owner: &str, repo: &str) -> Url {
    Url::parse(&format!(
        "https://raw.githubusercontent.com/{owner}/{repo}/HEAD/README.md"
    ))
    .expect("statically-constructed URL is always valid")
}

/// Fetch the README for `owner/repo`, attaching a Bearer token from the
/// `GITHUB_TOKEN` environment variable if set.
pub async fn fetch_readme(
    client: &rquest::Client,
    owner: &str,
    repo: &str,
) -> Result<String, GithubError> {
    let url = github_raw_url(owner, repo);
    let token = std::env::var("GITHUB_TOKEN").ok();

    let mut req = client.get(url.as_str());
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {t}"));
    }

    let resp = req.send().await.map_err(GithubError::Network)?;
    if resp.status() == 404 {
        return Err(GithubError::NotFound);
    }

    resp.text().await.map_err(|_| GithubError::Utf8)
}

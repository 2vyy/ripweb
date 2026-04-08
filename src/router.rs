use url::Url;

#[derive(Debug)]
#[non_exhaustive]
pub enum PlatformRoute {
    GitHub { owner: String, repo: String },
    Reddit { url: String },
    HackerNews { item_id: String },
    Generic(Url),
}

#[derive(Debug)]
pub enum Route {
    Url(PlatformRoute),
    Query(String),
}

/// Route raw input to a platform handler or a search query.
///
/// A string starting with `http://` or `https://` is treated as a URL;
/// everything else becomes a DuckDuckGo search query.
pub fn route(input: &str) -> Route {
    if (input.starts_with("http://") || input.starts_with("https://"))
        && let Ok(url) = Url::parse(input)
    {
        return Route::Url(classify_url(url));
    }
    Route::Query(input.to_owned())
}

fn classify_url(url: Url) -> PlatformRoute {
    match url.host_str() {
        Some("github.com") => classify_github(url),
        Some("www.reddit.com") | Some("reddit.com") | Some("old.reddit.com") => {
            PlatformRoute::Reddit { url: url.into() }
        }
        Some("news.ycombinator.com") => classify_hn(url),
        _ => PlatformRoute::Generic(url),
    }
}

fn classify_github(url: Url) -> PlatformRoute {
    // Require exactly /owner/repo as the first two non-empty path segments.
    let mut segments = url.path_segments().into_iter().flatten().filter(|s| !s.is_empty());
    if let (Some(owner), Some(repo)) = (segments.next(), segments.next()) {
        PlatformRoute::GitHub {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
        }
    } else {
        PlatformRoute::Generic(url)
    }
}

fn classify_hn(url: Url) -> PlatformRoute {
    // Only route item pages: /item?id=<number>
    let item_id = url
        .query_pairs()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| v.into_owned());

    if let Some(id) = item_id {
        PlatformRoute::HackerNews { item_id: id }
    } else {
        PlatformRoute::Generic(url)
    }
}

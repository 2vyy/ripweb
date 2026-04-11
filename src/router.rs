//! Routing & Input Classification
//!
//! The Router is responsible for classifying raw inputs (phrases vs. URLs)
//! and mapping URLs to specific platform handlers (Wikipedia, Reddit, etc.)
//! or the generic web crawler.

use url::Url;

#[derive(Debug, PartialEq, Eq)]
pub enum GitHubRouteType {
    Readme,
    Issues,
    Issue(u64),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum PlatformRoute {
    GitHub {
        owner: String,
        repo: String,
        route_type: GitHubRouteType,
    },
    Reddit {
        url: String,
    },
    HackerNews {
        item_id: String,
    },
    Wikipedia {
        title: String,
    },
    StackOverflow {
        question_id: u64,
    },
    ArXiv {
        paper_id: String,
    },
    YouTube {
        video_id: String,
        original_url: String,
    },
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
        Some("en.wikipedia.org") | Some("wikipedia.org") => classify_wikipedia(url),
        Some("stackoverflow.com") | Some("www.stackoverflow.com") => classify_stackoverflow(url),
        Some("arxiv.org") => classify_arxiv(url),
        Some("www.youtube.com") | Some("youtube.com") | Some("youtu.be") => classify_youtube(url),
        _ => PlatformRoute::Generic(url),
    }
}

fn classify_github(url: Url) -> PlatformRoute {
    let mut segments = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty());
    if let (Some(owner), Some(repo)) = (segments.next(), segments.next()) {
        let mut route_type = GitHubRouteType::Readme;
        if let Some(next) = segments.next()
            && (next == "issues" || next == "pulls")
        {
            if let Some(id_str) = segments.next() {
                if let Ok(id) = id_str.parse::<u64>() {
                    route_type = GitHubRouteType::Issue(id);
                } else {
                    route_type = GitHubRouteType::Issues;
                }
            } else {
                route_type = GitHubRouteType::Issues;
            }
        }
        PlatformRoute::GitHub {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
            route_type,
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

fn classify_wikipedia(url: Url) -> PlatformRoute {
    use crate::search::platforms::wikipedia::wiki_title_from_url;
    if let Some(title) = wiki_title_from_url(&url) {
        PlatformRoute::Wikipedia { title }
    } else {
        PlatformRoute::Generic(url)
    }
}

fn classify_stackoverflow(url: Url) -> PlatformRoute {
    use crate::search::platforms::stackoverflow::so_question_id_from_url;
    if let Some(id) = so_question_id_from_url(&url) {
        PlatformRoute::StackOverflow { question_id: id }
    } else {
        PlatformRoute::Generic(url)
    }
}

fn classify_arxiv(url: Url) -> PlatformRoute {
    use crate::search::platforms::arxiv::arxiv_id_from_url;
    if let Some(id) = arxiv_id_from_url(&url) {
        PlatformRoute::ArXiv { paper_id: id }
    } else {
        PlatformRoute::Generic(url)
    }
}

fn classify_youtube(url: Url) -> PlatformRoute {
    use crate::search::platforms::youtube::youtube_video_id;
    let original_url = url.to_string();
    if let Some(video_id) = youtube_video_id(&url) {
        PlatformRoute::YouTube {
            video_id,
            original_url,
        }
    } else {
        PlatformRoute::Generic(url)
    }
}

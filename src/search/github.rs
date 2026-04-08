//! GitHub Platform Integration
//!
//! Uses the GitHub REST API to fetch issues, pull requests, and
//! comments. Falls back to raw content for README files.

use serde::Deserialize;
use url::Url;

use crate::router::GitHubRouteType;

#[derive(Debug)]
#[non_exhaustive]
pub enum GithubError {
    Network(String),
    NotFound,
    Parse(String),
}

impl std::fmt::Display for GithubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::NotFound => write!(f, "not found"),
            Self::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}

pub async fn handle_github(
    client: &rquest::Client,
    owner: &str,
    repo: &str,
    route: &GitHubRouteType,
    verbosity: u8,
) -> Result<String, GithubError> {
    match route {
        GitHubRouteType::Readme => {
            let text = fetch_readme(client, owner, repo).await?;
            Ok(format_readme(&text, verbosity))
        }
        GitHubRouteType::Issues => {
            let issues = fetch_issues_list(client, owner, repo).await?;
            Ok(format_issues_list(owner, repo, &issues, verbosity))
        }
        GitHubRouteType::Issue(id) => {
            // Need two parallel requests for issue and comments
            let (issue, comments) = tokio::try_join!(
                fetch_issue(client, owner, repo, *id),
                fetch_issue_comments(client, owner, repo, *id),
            )?;
            Ok(format_issue(&issue, &comments, verbosity))
        }
    }
}

// ── README ──────────────────────────────────────────────────────────────────

fn github_raw_url(owner: &str, repo: &str) -> Url {
    Url::parse(&format!(
        "https://raw.githubusercontent.com/{owner}/{repo}/HEAD/README.md"
    ))
    .expect("statically-constructed URL is always valid")
}

async fn fetch_readme(
    client: &rquest::Client,
    owner: &str,
    repo: &str,
) -> Result<String, GithubError> {
    let url = github_raw_url(owner, repo);
    let mut req = client.get(url.as_str());
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        req = req.header("Authorization", format!("Bearer {t}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| GithubError::Network(e.to_string()))?;
    if resp.status() == 404 {
        return Err(GithubError::NotFound);
    }
    resp.text()
        .await
        .map_err(|_| GithubError::Parse("UTF-8".into()))
}

pub(crate) fn format_readme(text: &str, verbosity: u8) -> String {
    match verbosity {
        1 => {
            // V1: List of H1/H2/H3
            let lines: Vec<&str> = text
                .lines()
                .filter(|line| {
                    line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ")
                })
                .collect();
            lines.join("\n")
        }
        2 => {
            // V2: Try to strip out non-code text significantly (focus on code blocks/examples)
            let mut out = String::new();
            let mut in_code = false;
            for line in text.lines() {
                if line.starts_with("```") {
                    in_code = !in_code;
                    out.push_str(line);
                    out.push('\n');
                } else if in_code || line.starts_with('#') {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out
        }
        _ => text.to_owned(),
    }
}

// ── ISSUES ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GithubIssue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub labels: Vec<GithubLabel>,
    pub user: GithubUser,
    pub html_url: String,
}

#[derive(Deserialize)]
pub struct GithubLabel {
    pub name: String,
}

#[derive(Deserialize)]
pub struct GithubUser {
    pub login: String,
}

#[derive(Deserialize)]
pub struct GithubComment {
    pub body: Option<String>,
    pub user: GithubUser,
}

async fn github_api_get(client: &rquest::Client, url: &str) -> Result<String, GithubError> {
    let mut req = client.get(url).header("User-Agent", "ripweb-cli");
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        req = req.header("Authorization", format!("Bearer {t}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| GithubError::Network(e.to_string()))?;
    if resp.status() == 404 {
        return Err(GithubError::NotFound);
    }
    resp.text()
        .await
        .map_err(|e| GithubError::Network(e.to_string()))
}

async fn fetch_issues_list(
    client: &rquest::Client,
    owner: &str,
    repo: &str,
) -> Result<Vec<GithubIssue>, GithubError> {
    let url =
        format!("https://api.github.com/repos/{owner}/{repo}/issues?state=open&sort=comments");
    let text = github_api_get(client, &url).await?;
    serde_json::from_str(&text).map_err(|e| GithubError::Parse(e.to_string()))
}

async fn fetch_issue(
    client: &rquest::Client,
    owner: &str,
    repo: &str,
    id: u64,
) -> Result<GithubIssue, GithubError> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{id}");
    let text = github_api_get(client, &url).await?;
    serde_json::from_str(&text).map_err(|e| GithubError::Parse(e.to_string()))
}

async fn fetch_issue_comments(
    client: &rquest::Client,
    owner: &str,
    repo: &str,
    id: u64,
) -> Result<Vec<GithubComment>, GithubError> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{id}/comments");
    let text = github_api_get(client, &url).await?;
    serde_json::from_str(&text).map_err(|e| GithubError::Parse(e.to_string()))
}

fn format_issues_list(owner: &str, repo: &str, issues: &[GithubIssue], _verbosity: u8) -> String {
    // A broader query that returns multiple issues is always treated close to V1 (List)
    // unless we iterate and fetch their bodies, but the API already gives us bodies.
    let mut out = format!("# Issues for {owner}/{repo}\n\n");
    for issue in issues.iter().take(20) {
        let labels = issue
            .labels
            .iter()
            .map(|l| l.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "- [#{}] {} ({}) [Labels: {}]\n",
            issue.number, issue.title, issue.html_url, labels
        ));
    }
    out
}

pub fn format_issue(issue: &GithubIssue, comments: &[GithubComment], verbosity: u8) -> String {
    let mut out = String::new();
    let labels = issue
        .labels
        .iter()
        .map(|l| l.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    match verbosity {
        1 => {
            // V1: List of Issue Titles + Numbers + Labels.
            out.push_str(&format!(
                "- [#{}] {} [Labels: {}]\n",
                issue.number, issue.title, labels
            ));
        }
        2 => {
            // V2: Issue Title + OP's Description.
            out.push_str(&format!(
                "# [#{}] {}\n**Labels**: {}\n**Author**: {}\n\n",
                issue.number, issue.title, labels, issue.user.login
            ));
            if let Some(body) = &issue.body {
                out.push_str(body);
            }
        }
        _ => {
            // V3: Issue Title + OP's Description + All Comments.
            out.push_str(&format!(
                "# [#{}] {}\n**Labels**: {}\n**Author**: {}\n\n",
                issue.number, issue.title, labels, issue.user.login
            ));
            if let Some(body) = &issue.body {
                out.push_str(body);
            }
            out.push_str("\n\n## Comments\n\n");
            for comment in comments {
                out.push_str(&format!("### {}\n", comment.user.login));
                if let Some(b) = &comment.body {
                    out.push_str(b);
                }
                out.push_str("\n\n---\n\n");
            }
        }
    }
    out
}

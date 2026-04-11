//! GitHub Platform Integration
//!
//! Uses the GitHub REST API to fetch issues, pull requests, and
//! comments. Falls back to raw content for README files.

use std::fmt::Write;

use serde::Deserialize;
use url::Url;

use crate::router::GitHubRouteType;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GithubError {
    #[error("network error: {0}")]
    Network(String),
    #[error("not found")]
    NotFound,
    #[error("parse error: {0}")]
    Parse(String),
}

pub async fn handle_github(
    client: &rquest::Client,
    owner: &str,
    repo: &str,
    route: &GitHubRouteType,
    mode: crate::verbosity::Verbosity,
) -> Result<String, GithubError> {
    match route {
        GitHubRouteType::Readme => {
            let text = fetch_readme(client, owner, repo).await?;
            Ok(format_readme(&text, mode))
        }
        GitHubRouteType::Issues => {
            let issues = fetch_issues_list(client, owner, repo).await?;
            Ok(format_issues_list(owner, repo, &issues, mode))
        }
        GitHubRouteType::Issue(id) => {
            // Need two parallel requests for issue and comments
            let (issue, comments) = tokio::try_join!(
                fetch_issue(client, owner, repo, *id),
                fetch_issue_comments(client, owner, repo, *id),
            )?;
            Ok(format_issue(&issue, &comments, mode))
        }
    }
}

// ── README ──────────────────────────────────────────────────────────────────

fn github_raw_url(owner: &str, repo: &str) -> Result<Url, url::ParseError> {
    Url::parse(&format!(
        "https://raw.githubusercontent.com/{owner}/{repo}/HEAD/README.md"
    ))
}

async fn fetch_readme(
    client: &rquest::Client,
    owner: &str,
    repo: &str,
) -> Result<String, GithubError> {
    let url = github_raw_url(owner, repo).map_err(|e| GithubError::Parse(e.to_string()))?;
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

pub(crate) fn format_readme(text: &str, mode: crate::verbosity::Verbosity) -> String {
    match mode.density_tier() {
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

fn format_issues_list(
    owner: &str,
    repo: &str,
    issues: &[GithubIssue],
    _mode: crate::verbosity::Verbosity,
) -> String {
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
        let _ = writeln!(
            out,
            "- [#{}] {} ({}) [Labels: {}]",
            issue.number, issue.title, issue.html_url, labels
        );
    }
    out
}

pub fn format_issue(
    issue: &GithubIssue,
    comments: &[GithubComment],
    mode: crate::verbosity::Verbosity,
) -> String {
    let mut out = String::new();
    let labels = issue
        .labels
        .iter()
        .map(|l| l.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    match mode.density_tier() {
        1 => {
            // V1: List of Issue Titles + Numbers + Labels.
            let _ = writeln!(
                out,
                "- [#{}] {} [Labels: {}]",
                issue.number, issue.title, labels
            );
        }
        2 => {
            // V2: Issue Title + OP's Description.
            let _ = write!(
                out,
                "# [#{}] {}\n**Labels**: {}\n**Author**: {}\n\n",
                issue.number, issue.title, labels, issue.user.login
            );
            if let Some(body) = &issue.body {
                out.push_str(body);
            }
        }
        _ => {
            // V3: Issue Title + OP's Description + All Comments.
            let _ = write!(
                out,
                "# [#{}] {}\n**Labels**: {}\n**Author**: {}\n\n",
                issue.number, issue.title, labels, issue.user.login
            );
            if let Some(body) = &issue.body {
                out.push_str(body);
            }
            out.push_str("\n\n## Comments\n\n");
            for comment in comments {
                let _ = writeln!(out, "### {}", comment.user.login);
                if let Some(b) = &comment.body {
                    out.push_str(b);
                }
                out.push_str("\n\n---\n\n");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_issue(number: u64, body: Option<&str>) -> GithubIssue {
        GithubIssue {
            number,
            title: format!("Issue {number}"),
            body: body.map(ToOwned::to_owned),
            labels: vec![
                GithubLabel { name: "bug".into() },
                GithubLabel {
                    name: "help wanted".into(),
                },
            ],
            user: GithubUser {
                login: "alice".into(),
            },
            html_url: format!("https://github.com/org/repo/issues/{number}"),
        }
    }

    #[test]
    fn github_raw_url_points_to_head_readme() {
        let url = github_raw_url("rust-lang", "rust").unwrap();
        assert_eq!(
            url.as_str(),
            "https://raw.githubusercontent.com/rust-lang/rust/HEAD/README.md"
        );
    }

    #[test]
    fn format_readme_compact_keeps_only_headings() {
        let readme = "# Title\ntext\n## Install\nmore text\n### Deep\n";
        let compact = format_readme(readme, crate::verbosity::Verbosity::Compact);
        assert_eq!(compact, "# Title\n## Install\n### Deep");
    }

    #[test]
    fn format_readme_balanced_keeps_headings_and_code_blocks() {
        let readme = "# Title\nParagraph\n```rust\nfn main() {}\n```\nTrailing text";
        let balanced = format_readme(readme, crate::verbosity::Verbosity::Standard);
        assert!(balanced.contains("# Title"));
        assert!(balanced.contains("```rust"));
        assert!(balanced.contains("fn main() {}"));
        assert!(!balanced.contains("Paragraph"));
        assert!(!balanced.contains("Trailing text"));
    }

    #[test]
    fn format_readme_verbose_returns_input_verbatim() {
        let readme = "# Title\nParagraph\n";
        assert_eq!(
            format_readme(readme, crate::verbosity::Verbosity::Full),
            readme
        );
    }

    #[test]
    fn format_issues_list_limits_output_to_top_twenty() {
        let issues: Vec<GithubIssue> = (1..=25).map(|n| sample_issue(n, Some("body"))).collect();
        let rendered =
            format_issues_list("org", "repo", &issues, crate::verbosity::Verbosity::Compact);
        assert!(rendered.starts_with("# Issues for org/repo"));
        assert!(rendered.contains("[#1] Issue 1"));
        assert!(rendered.contains("[#20] Issue 20"));
        assert!(!rendered.contains("[#21] Issue 21"));
        assert!(rendered.contains("[Labels: bug, help wanted]"));
    }

    #[test]
    fn format_issue_handles_missing_bodies_across_modes() {
        let issue = sample_issue(42, None);
        let comments = vec![
            GithubComment {
                body: None,
                user: GithubUser {
                    login: "bob".into(),
                },
            },
            GithubComment {
                body: Some("Looks good".into()),
                user: GithubUser {
                    login: "carol".into(),
                },
            },
        ];

        let compact = format_issue(&issue, &comments, crate::verbosity::Verbosity::Compact);
        assert!(compact.contains("- [#42] Issue 42 [Labels: bug, help wanted]"));

        let balanced = format_issue(&issue, &comments, crate::verbosity::Verbosity::Standard);
        assert!(balanced.contains("**Author**: alice"));
        assert!(!balanced.contains("## Comments"));
        assert!(!balanced.contains("Looks good"));

        let verbose = format_issue(&issue, &comments, crate::verbosity::Verbosity::Full);
        assert!(verbose.contains("## Comments"));
        assert!(verbose.contains("### bob"));
        assert!(verbose.contains("### carol"));
        assert!(verbose.contains("Looks good"));
    }
}

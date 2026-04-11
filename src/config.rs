//! Extraction and Search Configuration
//!
//! Loads `config/ripweb.toml` (project-local) or the XDG config path.
//! All config types implement `Default` so no config file is required.

use directories::ProjectDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::search::scoring::ScoringWeights;

static CONFIG: OnceLock<RipwebConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RipwebConfig {
    #[serde(default)]
    pub extract: ExtractConfig,
    #[serde(default)]
    pub search: SearchConfig,
}

// ── Extract config (unchanged) ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractConfig {
    #[serde(default = "default_family")]
    pub default_family: String,
    #[serde(default)]
    pub domain_exact: HashMap<String, String>,
    #[serde(default)]
    pub suffix_rules: Vec<SuffixRule>,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            default_family: default_family(),
            domain_exact: HashMap::new(),
            suffix_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SuffixRule {
    pub suffix: String,
    pub family: String,
}

fn default_family() -> String {
    "generic".to_owned()
}

// ── Search config (new) ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SearchConfig {
    #[serde(default)]
    pub trust: TrustConfig,
    #[serde(default)]
    pub blocklist: BlocklistConfig,
    #[serde(default, alias = "weights")]
    pub scoring: ScoringWeights,
}

/// Domain trust tiers. Domains in `high` get a strong score boost;
/// domains in `medium` get a small boost; domains in `low` get a small penalty.
#[derive(Debug, Clone, Deserialize)]
pub struct TrustConfig {
    #[serde(default = "default_high_trust")]
    pub high: Vec<String>,
    #[serde(default = "default_medium_trust")]
    pub medium: Vec<String>,
    #[serde(default)]
    pub low: Vec<String>,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            high: default_high_trust(),
            medium: default_medium_trust(),
            low: Vec::new(),
        }
    }
}

fn default_high_trust() -> Vec<String> {
    [
        "docs.rs",
        "doc.rust-lang.org",
        "crates.io",
        "github.com",
        "pypi.org",
        "npmjs.com",
        "pkg.go.dev",
        "developer.mozilla.org",
        "developer.apple.com",
        "docs.python.org",
        "cppreference.com",
        "rust-lang.org",
        "golang.org",
        "python.org",
        "nodejs.org",
        "kotlinlang.org",
        "swift.org",
        "tokio.rs",
        "serde.rs",
        "hyper.rs",
        "clap.rs",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn default_medium_trust() -> Vec<String> {
    [
        "stackoverflow.com",
        "reddit.com",
        "news.ycombinator.com",
        "unix.stackexchange.com",
        "superuser.com",
        "serverfault.com",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Domains and categories that receive a hard score penalty.
#[derive(Debug, Clone, Deserialize)]
pub struct BlocklistConfig {
    /// Exact domain matches (and subdomains) to penalize heavily.
    #[serde(default = "default_blocklist_domains")]
    pub domains: Vec<String>,
}

impl Default for BlocklistConfig {
    fn default() -> Self {
        Self {
            domains: default_blocklist_domains(),
        }
    }
}

fn default_blocklist_domains() -> Vec<String> {
    [
        "w3schools.com",
        "geeksforgeeks.org",
        "tutorialspoint.com",
        "javatpoint.com",
        "programiz.com",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

// ── Config loading (unchanged logic) ─────────────────────────────────────────

pub fn get_config() -> &'static RipwebConfig {
    CONFIG.get_or_init(load_config)
}

fn load_config() -> RipwebConfig {
    for path in config_search_paths() {
        if !path.exists() {
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(text) => match toml::from_str::<RipwebConfig>(&text) {
                Ok(cfg) => return cfg,
                Err(err) => tracing::warn!("failed to parse config {}: {}", path.display(), err),
            },
            Err(err) => tracing::warn!("failed to read config {}: {}", path.display(), err),
        }
    }
    RipwebConfig::default()
}

fn config_search_paths() -> Vec<PathBuf> {
    let mut paths = vec![Path::new("config").join("ripweb.toml")];
    if let Some(dirs) = ProjectDirs::from("", "", "ripweb") {
        paths.push(dirs.config_dir().join("ripweb.toml"));
    }
    paths
}

pub fn family_hint_for_host(host: &str) -> Option<&str> {
    let host = host.to_ascii_lowercase();
    let cfg = get_config();

    if let Some(family) = cfg.extract.domain_exact.get(&host) {
        return Some(family.as_str());
    }
    if let Some(stripped) = host.strip_prefix("www.")
        && let Some(family) = cfg.extract.domain_exact.get(stripped)
    {
        return Some(family.as_str());
    }

    for rule in &cfg.extract.suffix_rules {
        if host.ends_with(&rule.suffix) {
            return Some(rule.family.as_str());
        }
    }

    Some(cfg.extract.default_family.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_domain_hint_matches() {
        assert_eq!(family_hint_for_host("docs.rs"), Some("docs"));
    }

    #[test]
    fn suffix_domain_hint_matches() {
        assert_eq!(family_hint_for_host("foo.readthedocs.io"), Some("docs"));
    }

    #[test]
    fn strips_www_for_exact_domain_hint_matches() {
        assert_eq!(family_hint_for_host("www.walmart.com"), Some("product"));
    }

    #[test]
    fn search_config_default_has_docs_rs_in_high_trust() {
        let cfg = RipwebConfig::default();
        assert!(cfg.search.trust.high.iter().any(|d| d == "docs.rs"));
    }

    #[test]
    fn search_config_default_weights_are_positive() {
        let w = ScoringWeights::default();
        assert!(w.domain_trust > 0.0);
        assert!(w.url_pattern > 0.0);
        assert!(w.project_match > 0.0);
        assert!(w.snippet_relevance > 0.0);
        assert!(w.domain_diversity > 0.0);
        assert!(w.blocklist_penalty > 0.0);
        assert!(w.rrf_k > 0.0);
    }

    #[test]
    fn blocklist_default_includes_w3schools() {
        let b = BlocklistConfig::default();
        assert!(b.domains.iter().any(|d| d == "w3schools.com"));
    }
}

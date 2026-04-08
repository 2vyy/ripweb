use directories::ProjectDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static CONFIG: OnceLock<RipwebConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RipwebConfig {
    #[serde(default)]
    pub extract: ExtractConfig,
}

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
    if let Some(stripped) = host.strip_prefix("www.") {
        if let Some(family) = cfg.extract.domain_exact.get(stripped) {
            return Some(family.as_str());
        }
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
}

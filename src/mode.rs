//! Output Mode
//!
//! The canonical 7-mode enum from the Output Contract. Controls information
//! density, Jina cloud proxy usage, and token budget per result.

use clap::ValueEnum;

/// Output mode controlling information density per the Output Contract.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Ultra-compact: link only. ~40–80 tokens per result.
    #[value(name = "omega-compact")]
    OmegaCompact,
    /// Compact: link + minimal platform metadata. ~120–250 tokens.
    Compact,
    /// Balanced: title + URL + first ~2000 chars. ~350–650 tokens. (default)
    #[default]
    Balanced,
    /// Detailed: full snippet + key excerpts. ~800–1800 tokens.
    Detailed,
    /// Verbose: full structured Markdown extraction. ~2k–6k tokens.
    Verbose,
    /// Omega-verbose: max signal on every top result + Jina always. ~8k–25k tokens.
    #[value(name = "omega-verbose")]
    OmegaVerbose,
    /// Aggressive: Jina forced, JS-heavy pages, max density. 15k+ tokens.
    Aggressive,
}

impl Mode {
    /// Internal density tier used to dispatch format branches.
    ///
    /// - `1` = link-only (omega-compact, compact)
    /// - `2` = snippet / first 2000 chars (balanced, detailed)
    /// - `3` = full content (verbose, omega-verbose, aggressive)
    pub fn density_tier(self) -> u8 {
        match self {
            Mode::OmegaCompact | Mode::Compact => 1,
            Mode::Balanced | Mode::Detailed => 2,
            Mode::Verbose | Mode::OmegaVerbose | Mode::Aggressive => 3,
        }
    }

    /// Whether this mode always routes through the Jina cloud proxy.
    pub fn jina_required(self) -> bool {
        matches!(self, Mode::OmegaVerbose | Mode::Aggressive)
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::OmegaCompact => write!(f, "omega-compact"),
            Mode::Compact => write!(f, "compact"),
            Mode::Balanced => write!(f, "balanced"),
            Mode::Detailed => write!(f, "detailed"),
            Mode::Verbose => write!(f, "verbose"),
            Mode::OmegaVerbose => write!(f, "omega-verbose"),
            Mode::Aggressive => write!(f, "aggressive"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_balanced() {
        assert_eq!(Mode::default(), Mode::Balanced);
    }

    #[test]
    fn density_tier_mapping() {
        assert_eq!(Mode::OmegaCompact.density_tier(), 1);
        assert_eq!(Mode::Compact.density_tier(), 1);
        assert_eq!(Mode::Balanced.density_tier(), 2);
        assert_eq!(Mode::Detailed.density_tier(), 2);
        assert_eq!(Mode::Verbose.density_tier(), 3);
        assert_eq!(Mode::OmegaVerbose.density_tier(), 3);
        assert_eq!(Mode::Aggressive.density_tier(), 3);
    }

    #[test]
    fn jina_required_only_for_high_density_modes() {
        assert!(!Mode::Balanced.jina_required());
        assert!(!Mode::Verbose.jina_required());
        assert!(Mode::OmegaVerbose.jina_required());
        assert!(Mode::Aggressive.jina_required());
    }

    #[test]
    fn display_produces_cli_names() {
        assert_eq!(Mode::OmegaCompact.to_string(), "omega-compact");
        assert_eq!(Mode::Compact.to_string(), "compact");
        assert_eq!(Mode::Balanced.to_string(), "balanced");
        assert_eq!(Mode::Detailed.to_string(), "detailed");
        assert_eq!(Mode::Verbose.to_string(), "verbose");
        assert_eq!(Mode::OmegaVerbose.to_string(), "omega-verbose");
        assert_eq!(Mode::Aggressive.to_string(), "aggressive");
    }
}

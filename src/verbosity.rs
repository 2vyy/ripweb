//! Output Density and Format
//!
//! Defines the canonical `--verbosity` and `--format` CLI enums from the
//! design document.

use clap::ValueEnum;

/// Output verbosity controlling information density.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Verbosity {
    /// Titles and URLs only.
    Compact,
    /// Title + snippet/key content, capped for token efficiency.
    #[default]
    Standard,
    /// Full extracted content.
    Full,
}

/// Output shaping mode.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Markdown output.
    #[default]
    Md,
    /// Plain text output (Markdown stripped, code preserved).
    Plain,
    /// Markdown plus per-source metadata header.
    Structured,
}

impl Verbosity {
    /// Internal density tier used by extractors/formatters.
    ///
    /// - `1` = link-only (compact)
    /// - `2` = snippet / key content (standard)
    /// - `3` = full content (full)
    pub fn density_tier(self) -> u8 {
        match self {
            Verbosity::Compact => 1,
            Verbosity::Standard => 2,
            Verbosity::Full => 3,
        }
    }
}

impl std::fmt::Display for Verbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verbosity::Compact => write!(f, "compact"),
            Verbosity::Standard => write!(f, "standard"),
            Verbosity::Full => write!(f, "full"),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Md => write!(f, "md"),
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::Structured => write!(f, "structured"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_standard() {
        assert_eq!(Verbosity::default(), Verbosity::Standard);
    }

    #[test]
    fn density_tier_mapping() {
        assert_eq!(Verbosity::Compact.density_tier(), 1);
        assert_eq!(Verbosity::Standard.density_tier(), 2);
        assert_eq!(Verbosity::Full.density_tier(), 3);
    }

    #[test]
    fn mode_display_produces_cli_names() {
        assert_eq!(Verbosity::Compact.to_string(), "compact");
        assert_eq!(Verbosity::Standard.to_string(), "standard");
        assert_eq!(Verbosity::Full.to_string(), "full");
    }

    #[test]
    fn output_format_display_produces_cli_names() {
        assert_eq!(OutputFormat::Md.to_string(), "md");
        assert_eq!(OutputFormat::Plain.to_string(), "plain");
        assert_eq!(OutputFormat::Structured.to_string(), "structured");
    }
}

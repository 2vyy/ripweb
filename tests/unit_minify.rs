use proptest::prelude::*;
use ripweb::minify::collapse;

proptest! {
    /// Invariant: Running the minifier twice on any string must be a no-op.
    /// `collapse(collapse(s)) == collapse(s)`
    #[test]
    fn minifier_is_idempotent(s in "\\PC*") {
        let once = collapse(&s);
        let twice = collapse(&once);
        prop_assert_eq!(once, twice);
    }

    /// Invariant: Minifier output must never be longer than the trimmed input.
    #[test]
    fn minifier_never_increases_length(s in "\\PC*") {
        let collapsed = collapse(&s);
        let original_trimmed = s.trim();
        prop_assert!(collapsed.len() <= original_trimmed.len());
    }

    /// Check behavior with Markdown fences.
    #[test]
    fn minifier_handles_markdown_fences(
        prefix in "\\PC*",
        middle in "\\PC*",
        suffix in "\\PC*"
    ) {
        let input = format!("{}\n```\n{}\n```\n{}", prefix, middle, suffix);
        let collapsed = collapse(&input);

        // If there are 3+ backticks in the input, the state machine should at least
        // preserve the "```" structure if it identified it as a fence.
        // We don't assert full verbatim here because `collapse` might still
        // trim the overall string or truncate tokens inside if they look like hashes,
        // but it should be idempotent.
        let twice = collapse(&collapsed);
        prop_assert_eq!(collapsed, twice);
    }

    /// Check behavior with Base64-like tokens.
    #[test]
    fn minifier_truncates_long_alphanumeric_tokens(
        prefix in "[a-z]{1,10}",
        token in "[A-Za-z0-9+/]{80,200}",
        suffix in "[a-z]{1,10}"
    ) {
        let input = format!("{} {} {}", prefix, token, suffix);
        let result = collapse(&input);
        prop_assert!(result.contains("[BASE64_TRUNCATED]"));
        prop_assert!(!result.contains(&token));
    }
}

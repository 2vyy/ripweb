/// Shared fixture corpus for examples, metrics, and golden generation tools.
///
/// The paths are relative to the repository root. Keeping this in one place
/// reduces drift between ad hoc scripts and evaluation harnesses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureReviewTier {
    GeneratedOnly,
    CuratedReference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedMode {
    Markdown,
    Aggressive,
}

#[derive(Clone, Copy, Debug)]
pub struct WebFixture {
    pub name: &'static str,
    pub html_path: &'static str,
    pub review_tier: FixtureReviewTier,
    pub curated_reference_path: Option<&'static str>,
    pub include_in_metrics: bool,
    pub generate_expected_outputs: bool,
}

impl WebFixture {
    pub fn generated_output_path(&self, mode: GeneratedMode) -> String {
        let mode_dir = match mode {
            GeneratedMode::Markdown => "markdown",
            GeneratedMode::Aggressive => "aggressive",
        };
        format!("tests/expected/generated/{mode_dir}/{}.md", self.name)
    }
}


pub const WEB_FIXTURES: &[WebFixture] = &[
    WebFixture {
        name: "react_dev_usestate",
        html_path: "corpus/web/react_dev_usestate.html",
        review_tier: FixtureReviewTier::CuratedReference,
        curated_reference_path: Some("tests/expected/curated/react_dev_usestate.golden.md"),
        include_in_metrics: true,
        generate_expected_outputs: true,
    },
    WebFixture {
        name: "stackoverflow_11227809",
        html_path: "corpus/web/stackoverflow_11227809.html",
        review_tier: FixtureReviewTier::CuratedReference,
        curated_reference_path: Some("tests/expected/curated/stackoverflow_11227809.golden.md"),
        include_in_metrics: true,
        generate_expected_outputs: true,
    },
    WebFixture {
        name: "docs_rs_axum",
        html_path: "corpus/web/docs_rs_axum.html",
        review_tier: FixtureReviewTier::CuratedReference,
        curated_reference_path: Some("tests/expected/curated/docs_rs_axum.golden.md"),
        include_in_metrics: true,
        generate_expected_outputs: true,
    },
    WebFixture {
        name: "paulgraham_essay",
        html_path: "corpus/web/paulgraham_essay.html",
        review_tier: FixtureReviewTier::CuratedReference,
        curated_reference_path: Some("tests/expected/curated/paulgraham_essay.golden.md"),
        include_in_metrics: true,
        generate_expected_outputs: true,
    },
    WebFixture {
        name: "mdnwebdocs_fetch",
        html_path: "corpus/web/mdnwebdocs_fetch.html",
        review_tier: FixtureReviewTier::CuratedReference,
        curated_reference_path: Some("tests/expected/curated/mdnwebdocs_fetch.golden.md"),
        include_in_metrics: true,
        generate_expected_outputs: true,
    },
    WebFixture {
        name: "rustblog_post",
        html_path: "corpus/web/rustblog_post.html",
        review_tier: FixtureReviewTier::CuratedReference,
        curated_reference_path: Some("tests/expected/curated/rustblog_post.golden.md"),
        include_in_metrics: true,
        generate_expected_outputs: true,
    },
    WebFixture {
        name: "devto_article",
        html_path: "corpus/web/devto_article.html",
        review_tier: FixtureReviewTier::CuratedReference,
        curated_reference_path: Some("tests/expected/curated/devto_article.golden.md"),
        include_in_metrics: true,
        generate_expected_outputs: true,
    },
];

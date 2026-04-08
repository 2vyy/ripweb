use ripweb::corpus::{FixtureReviewTier, GeneratedMode, WEB_FIXTURES};
use std::path::Path;

fn exists(path: &str) -> &'static str {
    if Path::new(path).exists() {
        "yes"
    } else {
        "no"
    }
}

fn tier_name(tier: FixtureReviewTier) -> &'static str {
    match tier {
        FixtureReviewTier::GeneratedOnly => "generated",
        FixtureReviewTier::CuratedReference => "curated",
    }
}

fn main() {
    println!("\nFixture review manifest\n");
    println!(
        "{:<24} {:<10} {:<8} {:<8} {:<8} {:<8}",
        "fixture", "tier", "curated", "markdown", "aggr", "metrics"
    );
    println!(
        "{:<24} {:<10} {:<8} {:<8} {:<8} {:<8}",
        "------------------------",
        "----------",
        "--------",
        "--------",
        "--------",
        "--------"
    );

    for fixture in WEB_FIXTURES {
        let curated = fixture.curated_reference_path.map(exists).unwrap_or("n/a");
        let markdown = if fixture.generate_expected_outputs {
            exists(&fixture.generated_output_path(GeneratedMode::Markdown))
        } else {
            "n/a"
        };
        let aggressive = if fixture.generate_expected_outputs {
            exists(&fixture.generated_output_path(GeneratedMode::Aggressive))
        } else {
            "n/a"
        };
        let metrics = if fixture.include_in_metrics { "yes" } else { "no" };

        println!(
            "{:<24} {:<10} {:<8} {:<8} {:<8} {:<8}",
            fixture.name,
            tier_name(fixture.review_tier),
            curated,
            markdown,
            aggressive,
            metrics
        );
    }

    println!("\nLegend:");
    println!("  curated  = human-maintained reference exists");
    println!("  markdown = generated markdown artifact exists");
    println!("  aggr     = generated aggressive artifact exists");
}

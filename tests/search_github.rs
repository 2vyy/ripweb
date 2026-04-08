use ripweb::search::github::github_raw_url;

#[test]
fn raw_url_points_to_raw_githubusercontent() {
    let url = github_raw_url("tokio-rs", "tokio");
    assert!(url.host_str() == Some("raw.githubusercontent.com"), "host: {url}");
}

#[test]
fn raw_url_contains_owner_and_repo() {
    let url = github_raw_url("rust-lang", "rust");
    assert!(url.path().contains("/rust-lang/"), "path: {}", url.path());
    assert!(url.path().contains("/rust/"), "path: {}", url.path());
}

#[test]
fn raw_url_targets_head_ref_not_hardcoded_main() {
    let url = github_raw_url("tokio-rs", "tokio");
    assert!(
        url.path().contains("/HEAD/"),
        "URL should use /HEAD/ ref, got path: {}",
        url.path()
    );
    assert!(
        !url.path().contains("/main/"),
        "/main/ hard-codes the branch name, use /HEAD/ instead"
    );
}

#[test]
fn raw_url_ends_with_readme() {
    let url = github_raw_url("tokio-rs", "tokio");
    assert!(url.path().ends_with("README.md"), "path: {}", url.path());
}

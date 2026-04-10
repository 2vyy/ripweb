use criterion::{Criterion, criterion_group, criterion_main};
use ripweb::mode::Mode;
use ripweb::search::github::{GithubComment, GithubIssue, GithubLabel, GithubUser, format_issue};
use std::hint::black_box;

fn bench_format_issue(c: &mut Criterion) {
    let issue = GithubIssue {
        number: 123,
        title: "Test Issue".into(),
        body: Some("This is a test issue body with some details. It goes on and on to simulate a realistic issue description.".into()),
        labels: vec![GithubLabel { name: "bug".into() }, GithubLabel { name: "help wanted".into() }],
        user: GithubUser { login: "octocat".into() },
        html_url: "https://github.com/octocat/Hello-World/issues/123".into(),
    };

    let comments = vec![
        GithubComment {
            user: GithubUser { login: "user1".into() },
            body: Some("Comment 1 with some text.".into()),
        },
        GithubComment {
            user: GithubUser { login: "user2".into() },
            body: Some("Comment 2 is a bit longer and has more text to simulate a real comment on an issue in GitHub.".into()),
        },
        GithubComment {
            user: GithubUser { login: "user3".into() },
            body: Some("Comment 3".into()),
        },
    ];

    c.bench_function("format_issue_v3", |b| {
        b.iter(|| {
            format_issue(
                black_box(&issue),
                black_box(&comments),
                black_box(Mode::Verbose),
            )
        })
    });
}

criterion_group!(benches, bench_format_issue);
criterion_main!(benches);

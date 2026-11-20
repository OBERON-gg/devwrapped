use devwrapped_card::{Clock, Theme, html};
use devwrapped_core::{AgentStats, Stats, Tally, sample};

const THEMES: [Theme; 3] = [Theme::Night, Theme::Poster, Theme::Paper];

fn without_data_uris(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find("data:") {
        out += &rest[..start + 5];
        let end = rest[start..].find(')').unwrap();
        rest = &rest[start + end..];
    }
    out + rest
}

fn assert_offline(html: &str) {
    let page = without_data_uris(html).to_lowercase();
    for needle in [
        "://",
        "\"//",
        "'//",
        "(//",
        "src=",
        "href=",
        "<link",
        "<iframe",
        "<object",
        "<embed",
        "@import",
        "fetch(",
        "xmlhttprequest",
        "websocket",
        "eventsource",
        "sendbeacon",
        "import(",
    ] {
        assert!(!page.contains(needle), "found {needle}");
    }
    for (index, _) in page.match_indices("url(") {
        let target = &page[index + 4..];
        assert!(
            target.starts_with("data:") || target.starts_with('#'),
            "{}",
            &target[..target.len().min(40)]
        );
    }
}

#[test]
fn sample_page_has_no_external_url() {
    for theme in THEMES {
        assert_offline(&html(&sample(), theme, Clock::H24));
    }
}

#[test]
fn empty_page_has_no_external_url() {
    for theme in THEMES {
        assert_offline(&html(&Stats::default(), theme, Clock::H24));
    }
}

#[test]
fn names_cannot_add_a_tag() {
    let stats = Stats {
        agents: Some(AgentStats {
            sessions: vec![Tally {
                name: "Claude Code".to_string(),
                count: 3,
            }],
            top_models: vec![Tally {
                name: "<script src=x.js></script><img src=y>".to_string(),
                count: 3,
            }],
            ..AgentStats::default()
        }),
        ..Stats::default()
    };
    for theme in THEMES {
        let page = html(&stats, theme, Clock::H24);
        assert!(page.contains("&lt;script"));
        assert_eq!(page.matches("<script").count(), 1);
        assert!(!page.contains("<img"));
    }
}

#[test]
fn finds_a_planted_url() {
    let page = html(&sample(), Theme::Night, Clock::H24);
    let planted = page.replacen("</style>", "@import \"x.css\";</style>", 1);
    let result = std::panic::catch_unwind(|| assert_offline(&planted));
    assert!(result.is_err());
}

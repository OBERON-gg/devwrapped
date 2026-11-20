use devwrapped_card::{Clock, Theme, slides};
use devwrapped_core::sample;

fn snapshot(theme: Theme, name: &str) {
    for (index, slide) in slides(&sample(), theme, Clock::H24).iter().enumerate() {
        insta::assert_snapshot!(format!("{name}-{:02}-{}", index + 1, slide.name), slide.svg);
    }
}

#[test]
fn night() {
    snapshot(Theme::Night, "night");
}

#[test]
fn poster() {
    snapshot(Theme::Poster, "poster");
}

#[test]
fn paper() {
    snapshot(Theme::Paper, "paper");
}

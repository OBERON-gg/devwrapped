#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used)]

mod afterglow;
mod clock;
mod draw;
pub mod fonts;
mod html;
mod measure;
mod png;
mod slides;
mod story;
mod swiss;
mod theme;
mod zine;

pub use clock::Clock;
pub use html::html;
pub use png::{PngError, png};
pub use slides::{Slide, slides};
pub use theme::Theme;

use devwrapped_core::Stats;

// slides show a range with an en dash, while file names keep the plain hyphen
pub(crate) fn year_text(stats: &Stats) -> String {
    year_label(stats).replace('-', "\u{2013}")
}

/// gives both years when the range crosses new year, such as `2025-2026`
pub fn year_label(stats: &Stats) -> String {
    let (since, until) = (stats.since.year(), stats.until.year());
    if since == until {
        since.to_string()
    } else {
        format!("{since}-{until}")
    }
}

pub fn thousands(number: u64) -> String {
    let digits = number.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// shortens numbers of 100,000 or more and rounds down, so a value is never overstated
pub fn compact(number: u64) -> String {
    let (unit, suffix) = match number {
        0..100_000 => return thousands(number),
        100_000..1_000_000 => (1_000, "K"),
        1_000_000..1_000_000_000 => (1_000_000, "M"),
        _ => (1_000_000_000, "B"),
    };
    let tenths = u128::from(number) * 10 / unit;
    if tenths >= 100 || tenths % 10 == 0 {
        format!("{}{suffix}", thousands((tenths / 10) as u64))
    } else {
        format!("{}.{}{suffix}", tenths / 10, tenths % 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_digits_by_thousands() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(812_000_000), "812,000,000");
        assert_eq!(thousands(u64::MAX), "18,446,744,073,709,551,615");
    }

    #[test]
    fn shortens_large_numbers_without_rounding_up() {
        assert_eq!(compact(48_213), "48,213");
        assert_eq!(compact(100_000), "100K");
        assert_eq!(compact(999_999), "999K");
        assert_eq!(compact(1_000_000), "1M");
        assert_eq!(compact(13_200_000), "13M");
        assert_eq!(compact(3_450_000), "3.4M");
        assert_eq!(compact(812_000_000), "812M");
        assert_eq!(compact(u64::MAX), "18,446,744,073B");
    }
}

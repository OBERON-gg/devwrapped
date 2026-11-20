use std::sync::{Arc, OnceLock};

use resvg::usvg::fontdb::Database;
use resvg::usvg::{Options, Tree};

use crate::draw::escape;
use crate::fonts;

pub(crate) fn database() -> Arc<Database> {
    static DATABASE: OnceLock<Arc<Database>> = OnceLock::new();
    DATABASE
        .get_or_init(|| {
            let mut database = Database::new();
            for font in [
                fonts::MONO_REGULAR,
                fonts::MONO_BOLD,
                fonts::UNBOUNDED,
                fonts::SPACE_GROTESK,
            ] {
                database.load_font_data(font.to_vec());
            }
            database.set_monospace_family(fonts::MONO_FAMILY);
            Arc::new(database)
        })
        .clone()
}

pub(crate) fn options() -> Options<'static> {
    Options {
        font_family: fonts::MONO_FAMILY.to_string(),
        fontdb: database(),
        ..Options::default()
    }
}

/// the inked width of one line of text, as resvg lays it out with the bundled fonts
pub(crate) fn width(text: &str, family: &str, weight: u16, size: f32, spacing: f32) -> f32 {
    if text.trim().is_empty() {
        return 0.0;
    }
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20000\" height=\"2000\">\
         <text x=\"0\" y=\"1000\" font-family=\"{family}\" font-weight=\"{weight}\" \
         font-size=\"{size}\" letter-spacing=\"{spacing}\">{}</text></svg>",
        escape(text)
    );
    Tree::from_str(&svg, &options())
        .ok()
        .and_then(|tree| tree.root().abs_bounding_box().map(|bounds| bounds.right()))
        // a glyph that cannot be laid out still takes room, so guess wide
        .unwrap_or(text.chars().count() as f32 * size * 0.8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_mono_text_at_its_fixed_advance() {
        let measured = width("0000000000", fonts::MONO_FAMILY, 400, 100.0, 0.0);
        assert!((560.0..=600.0).contains(&measured), "{measured}");
    }

    #[test]
    fn measures_each_weight_on_its_own() {
        for family in [fonts::UNBOUNDED_FAMILY, fonts::SPACE_GROTESK_FAMILY] {
            let light = width("wrapped", family, 300, 100.0, 0.0);
            let bold = width("wrapped", family, 800, 100.0, 0.0);
            assert!((bold - light).abs() > 1.0, "{family} {light} {bold}");
        }
    }

    #[test]
    fn adds_letter_spacing() {
        let plain = width("abcd", fonts::MONO_FAMILY, 400, 40.0, 0.0);
        let spaced = width("abcd", fonts::MONO_FAMILY, 400, 40.0, 10.0);
        assert!(spaced - plain >= 29.0, "{plain} {spaced}");
    }

    #[test]
    fn measures_nothing_as_zero() {
        assert_eq!(width("  ", fonts::MONO_FAMILY, 400, 40.0, 0.0), 0.0);
    }
}

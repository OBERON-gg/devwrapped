use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::Tree;

use crate::measure;

#[derive(Debug, thiserror::Error)]
pub enum PngError {
    #[error("could not read the card svg")]
    Svg,
    #[error("could not allocate the card image")]
    Pixmap,
    #[error("could not encode the card png")]
    Encode,
}

// the source errors are dropped because they can quote card text
pub fn png(svg: &str) -> Result<Vec<u8>, PngError> {
    let tree = Tree::from_str(svg, &measure::options()).map_err(|_| PngError::Svg)?;
    let size = tree.size().to_int_size();
    let mut pixmap = Pixmap::new(size.width(), size.height()).ok_or(PngError::Pixmap)?;
    resvg::render(&tree, Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|_| PngError::Encode)
}

#[cfg(test)]
mod tests {
    use devwrapped_core::sample;

    use super::*;
    use crate::{Clock, Theme, slides};

    fn chunks(png: &[u8]) -> Vec<(&[u8], &[u8])> {
        let mut chunks = Vec::new();
        let mut rest = &png[8..];
        while rest.len() >= 12 {
            let length = u32::from_be_bytes(rest[..4].try_into().unwrap()) as usize;
            chunks.push((&rest[4..8], &rest[8..8 + length]));
            rest = &rest[12 + length..];
        }
        chunks
    }

    fn first_slide(theme: Theme) -> Vec<u8> {
        png(&slides(&sample(), theme, Clock::H24)[0].svg).unwrap()
    }

    #[test]
    fn has_the_slide_size() {
        let png = first_slide(Theme::Night);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let (kind, header) = chunks(&png)[0];
        assert_eq!(kind, b"IHDR");
        assert_eq!(u32::from_be_bytes(header[..4].try_into().unwrap()), 1080);
        assert_eq!(u32::from_be_bytes(header[4..8].try_into().unwrap()), 1350);
    }

    #[test]
    fn writes_no_metadata() {
        for theme in [Theme::Night, Theme::Poster, Theme::Paper] {
            for (kind, _) in chunks(&first_slide(theme)) {
                assert!([b"IHDR", b"IDAT", b"IEND"].contains(&kind.try_into().unwrap()));
            }
        }
    }

    // only the bundled fonts are loaded, so any white text pixel comes from them
    #[test]
    fn draws_the_year_with_the_bundled_font() {
        let pixmap = resvg::tiny_skia::Pixmap::decode_png(&first_slide(Theme::Night)).unwrap();
        let lit = (440..600)
            .flat_map(|y| (80..400).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let pixel = pixmap.pixel(x, y).unwrap();
                pixel.red() > 230 && pixel.green() > 230 && pixel.blue() > 230
            })
            .count();
        assert!(lit > 4000, "{lit}");
    }

    #[test]
    fn refuses_broken_svg_without_quoting_it() {
        let error = png("<svg hunter2").unwrap_err();
        assert!(!error.to_string().contains("hunter2"));
    }
}

pub const MONO_FAMILY: &str = "JetBrains Mono";
pub const MONO_REGULAR: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");
pub const MONO_BOLD: &[u8] = include_bytes!("../fonts/JetBrainsMono-Bold.ttf");
pub const UNBOUNDED_FAMILY: &str = "Unbounded";
pub const UNBOUNDED: &[u8] = include_bytes!("../fonts/Unbounded-Variable.ttf");
pub const SPACE_GROTESK_FAMILY: &str = "Space Grotesk";
pub const SPACE_GROTESK: &[u8] = include_bytes!("../fonts/SpaceGrotesk-Variable.ttf");

#[cfg(test)]
mod tests {
    use super::*;

    fn table<'a>(font: &'a [u8], tag: &[u8]) -> &'a [u8] {
        let count = usize::from(u16::from_be_bytes([font[4], font[5]]));
        let record = (0..count)
            .map(|index| &font[12 + 16 * index..28 + 16 * index])
            .find(|record| &record[..4] == tag)
            .unwrap();
        let offset = u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(record[12..16].try_into().unwrap()) as usize;
        &font[offset..offset + length]
    }

    fn u16_at(bytes: &[u8], offset: usize) -> u16 {
        u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
    }

    #[test]
    fn names_the_family_the_card_asks_for() {
        let fonts = [
            (MONO_FAMILY, MONO_REGULAR),
            (MONO_FAMILY, MONO_BOLD),
            (UNBOUNDED_FAMILY, UNBOUNDED),
            (SPACE_GROTESK_FAMILY, SPACE_GROTESK),
        ];
        for (family, font) in fonts {
            let utf16: Vec<u8> = family.encode_utf16().flat_map(u16::to_be_bytes).collect();
            let names = table(font, b"name");
            assert!(
                names.windows(utf16.len()).any(|window| window == utf16),
                "{family}"
            );
        }
    }

    // the slides pick bold and light weights from one file
    #[test]
    fn varies_the_display_fonts_by_weight() {
        for font in [UNBOUNDED, SPACE_GROTESK] {
            assert!(table(font, b"fvar").windows(4).any(|tag| tag == b"wght"));
        }
    }

    // the card layout measures text as 0.6 em per character
    #[test]
    fn gives_every_glyph_the_same_advance() {
        for font in [MONO_REGULAR, MONO_BOLD] {
            assert_eq!(u16_at(table(font, b"head"), 18), 1000);
            let metrics = usize::from(u16_at(table(font, b"hhea"), 34));
            let advances = table(font, b"hmtx");
            for index in 0..metrics {
                let advance = u16_at(advances, 4 * index);
                assert!(advance == 0 || advance == 600, "{advance}");
            }
        }
    }
}

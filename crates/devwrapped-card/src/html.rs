use devwrapped_core::Stats;

use crate::{Clock, Theme, fonts, slides};

struct Stagger {
    selector: &'static str,
    variable: &'static str,
    count: usize,
    step_ms: usize,
    first_ms: usize,
}

// sections, lines, bars, and streak cells each enter a little after the one before
const STAGGERS: [Stagger; 4] = [
    Stagger {
        selector: ".part",
        variable: "--delay",
        count: 8,
        step_ms: 70,
        first_ms: 40,
    },
    Stagger {
        selector: ".part text",
        variable: "--line",
        count: 8,
        step_ms: 35,
        first_ms: 0,
    },
    Stagger {
        selector: ".bars rect, .hbars rect",
        variable: "--item",
        count: 24,
        step_ms: 18,
        first_ms: 0,
    },
    Stagger {
        selector: ".cells rect",
        variable: "--item",
        count: 28,
        step_ms: 12,
        first_ms: 0,
    },
];
const STYLE: &str = include_str!("card.css");
const SCRIPT: &str = include_str!("card.js");

pub fn html(stats: &Stats, theme: Theme, clock: Clock) -> String {
    let palette = theme.palette();
    let slides: String = slides(stats, theme, clock)
        .iter()
        .enumerate()
        .map(|(index, slide)| {
            // inline svg needs no namespace, and the page should hold no url at all
            let svg = slide
                .svg
                .replacen(" xmlns=\"http://www.w3.org/2000/svg\"", "", 1);
            let current = if index == 0 { " current" } else { "" };
            format!(
                "<section class=\"slide{current}\" aria-label=\"{}\">\n{svg}</section>\n",
                slide.name
            )
        })
        .collect();
    let delays: String = STAGGERS
        .iter()
        .flat_map(|stagger| {
            (1..=stagger.count).map(move |index| {
                let rules: Vec<String> = stagger
                    .selector
                    .split(", ")
                    .map(|one| format!("{one}:nth-of-type({index})"))
                    .collect();
                format!(
                    "{} {{ {}: {}ms; }}\n",
                    rules.join(", "),
                    stagger.variable,
                    stagger.first_ms + (index - 1) * stagger.step_ms
                )
            })
        })
        .collect();
    format!(
        "<!doctype html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>devwrapped</title>\n\
         <style>\n\
         {regular}\
         {bold}\
         {display}\
         :root {{ --edge: {edge}; }}\n\
         body {{ background: {background}; }}\n\
         {STYLE}\
         {delays}\
         </style>\n\
         </head>\n\
         <body>\n\
         <main>\n\
         {slides}\
         </main>\n\
         <script>\n\
         {SCRIPT}\
         </script>\n\
         </body>\n\
         </html>\n",
        regular = font_face(fonts::MONO_FAMILY, fonts::MONO_REGULAR, "400"),
        bold = font_face(fonts::MONO_FAMILY, fonts::MONO_BOLD, "700"),
        display = font_face(
            theme.display(400).family,
            theme.display_font(),
            match theme {
                Theme::Night => "200 900",
                Theme::Poster | Theme::Paper => "300 700",
            },
        ),
        background = palette.background,
        edge = match theme {
            Theme::Night => "oklch(1 0 0 / 0.1)",
            Theme::Poster | Theme::Paper => "oklch(0 0 0 / 0.1)",
        },
    )
}

fn font_face(family: &str, font: &[u8], weight: &str) -> String {
    format!(
        "@font-face {{ font-family: \"{family}\"; font-weight: {weight}; \
         src: url(data:font/ttf;base64,{}) format(\"truetype\"); }}\n",
        base64(font)
    )
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = chunk.iter().enumerate().fold(0u32, |acc, (index, &byte)| {
            acc | u32::from(byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(char::from(
                    ALPHABET[(triple >> (18 - 6 * index) & 63) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use devwrapped_core::sample;

    use super::*;

    #[test]
    fn encodes_base64() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (input, output) in cases {
            assert_eq!(base64(input.as_bytes()), output);
        }
        assert_eq!(base64(&[0xff, 0xfe, 0x00]), "//4A");
    }

    #[test]
    fn holds_every_slide_in_one_page() {
        let html = html(&sample(), Theme::Night, Clock::H24);
        let count = slides(&sample(), Theme::Night, Clock::H24).len();
        assert!(html.starts_with("<!doctype html>\n"));
        assert!(html.ends_with("</html>\n"));
        assert_eq!(html.matches("<svg ").count(), count);
        assert_eq!(html.matches("<section class=\"slide").count(), count);
        assert_eq!(html.matches(" current\"").count(), 1);
        assert_eq!(html.matches("<style>").count(), 1);
        assert_eq!(html.matches("<script>").count(), 1);
        assert!(!html.contains("xmlns"));
    }

    #[test]
    fn embeds_only_the_fonts_of_the_theme() {
        let night = html(&sample(), Theme::Night, Clock::H24);
        assert!(night.contains(&base64(fonts::UNBOUNDED)));
        assert!(!night.contains(&base64(fonts::SPACE_GROTESK)));
        let paper = html(&sample(), Theme::Paper, Clock::H24);
        assert!(paper.contains(&base64(fonts::SPACE_GROTESK)));
        assert!(!paper.contains(&base64(fonts::UNBOUNDED)));
        for page in [night, paper] {
            assert!(page.contains(&base64(fonts::MONO_REGULAR)));
            assert!(page.contains(&base64(fonts::MONO_BOLD)));
        }
    }

    #[test]
    fn keeps_glow_ids_unique_across_slides() {
        let html = html(&sample(), Theme::Night, Clock::H24);
        let mut ids: Vec<&str> = html
            .match_indices("id=\"")
            .map(|(index, _)| {
                let rest = &html[index + 4..];
                &rest[..rest.find('"').unwrap()]
            })
            .collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert!(count > 0);
        assert_eq!(ids.len(), count);
    }
}

use std::fmt::Write;

use crate::measure;
use crate::story::Page;
use crate::theme::{Palette, Theme};

pub(crate) const WIDTH: f32 = 1080.0;
pub(crate) const HEIGHT: f32 = 1350.0;
pub(crate) const MARGIN: f32 = 88.0;
pub(crate) const CONTENT: f32 = WIDTH - 2.0 * MARGIN;

pub(crate) struct Scene<'a> {
    pub theme: Theme,
    pub palette: &'a Palette,
    pub page: &'a Page,
    pub number: usize,
    pub year: &'a str,
    pub total: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Font {
    pub family: &'static str,
    pub weight: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Anchor {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Style<'a> {
    pub font: Font,
    pub size: f32,
    pub fill: &'a str,
    pub anchor: Anchor,
    pub spacing: f32,
    pub class: Option<&'static str>,
}

impl<'a> Style<'a> {
    pub fn new(font: Font, size: f32, fill: &'a str) -> Self {
        Style {
            font,
            size,
            fill,
            anchor: Anchor::Start,
            spacing: 0.0,
            class: None,
        }
    }

    pub fn anchor(self, anchor: Anchor) -> Self {
        Style { anchor, ..self }
    }

    pub fn spacing(self, spacing: f32) -> Self {
        Style { spacing, ..self }
    }

    pub fn class(self, class: &'static str) -> Self {
        Style {
            class: Some(class),
            ..self
        }
    }

    pub fn size(self, size: f32) -> Self {
        Style { size, ..self }
    }

    pub fn fill(self, fill: &'a str) -> Self {
        Style { fill, ..self }
    }

    fn width(&self, text: &str) -> f32 {
        measure::width(
            text,
            self.font.family,
            self.font.weight,
            self.size,
            self.spacing,
        )
    }
}

/// where a line of text landed, so tests can check that nothing runs off the slide
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextBox {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub size: f32,
    pub text: String,
}

pub(crate) struct Canvas {
    out: String,
    pub boxes: Vec<TextBox>,
}

impl Canvas {
    pub fn new() -> Self {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH}\" height=\"{HEIGHT}\" \
             viewBox=\"0 0 {WIDTH} {HEIGHT}\">"
        );
        Canvas {
            out,
            boxes: Vec::new(),
        }
    }

    pub fn raw(&mut self, svg: &str) {
        self.out += svg;
        self.out.push('\n');
    }

    pub fn group(&mut self, class: &str, draw: impl FnOnce(&mut Self)) {
        let _ = writeln!(self.out, "<g class=\"{class}\">");
        draw(self);
        self.out += "</g>\n";
    }

    /// writes one line and returns its width
    pub fn text(&mut self, x: f32, y: f32, text: &str, style: Style) -> f32 {
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        let width = style.width(&text);
        let (anchor, left) = match style.anchor {
            Anchor::Start => ("start", x),
            Anchor::Middle => ("middle", x - width / 2.0),
            Anchor::End => ("end", x - width),
        };
        let class = style
            .class
            .map(|class| format!(" class=\"{class}\""))
            .unwrap_or_default();
        let _ = writeln!(
            self.out,
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-weight=\"{}\" font-size=\"{}\" \
             fill=\"{}\" text-anchor=\"{anchor}\" letter-spacing=\"{}\"{class}>{}</text>",
            round(x),
            round(y),
            style.font.family,
            style.font.weight,
            round(style.size),
            style.fill,
            round(style.spacing),
            escape(&text)
        );
        self.boxes.push(TextBox {
            left,
            right: left + width,
            top: y - style.size * 0.72,
            bottom: y + style.size * 0.12,
            size: style.size,
            text,
        });
        width
    }

    /// shrinks the text toward `min_size` to fit `max_width`, then cuts it, and returns the size used
    pub fn fit(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        style: Style,
        max_width: f32,
        min_size: f32,
    ) -> f32 {
        let (text, size) = fit(text, style, max_width, min_size);
        self.text(x, y, &text, style.size(size));
        size
    }

    /// writes `text` over as many lines as it needs, breaking at each `\n`, and returns the baseline after the last one
    pub fn wrap(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        style: Style,
        max_width: f32,
        line: f32,
    ) -> f32 {
        let mut y = y;
        for row in wrap(text, style, max_width) {
            self.text(x, y, &row, style);
            y += line;
        }
        y
    }

    pub fn finish(mut self) -> String {
        self.out += "</svg>\n";
        self.out
    }
}

pub(crate) fn fit(text: &str, style: Style, max_width: f32, min_size: f32) -> (String, f32) {
    let width = style.width(text);
    if width <= max_width {
        return (text.to_string(), style.size);
    }
    let size = (style.size * max_width / width).floor().max(min_size);
    let style = style.size(size);
    if style.width(text) <= max_width {
        return (text.to_string(), size);
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let cut: String = chars.iter().collect::<String>() + "…";
        if style.width(&cut) <= max_width {
            return (cut, size);
        }
    }
    ("…".to_string(), size)
}

pub(crate) fn wrap(text: &str, style: Style, max_width: f32) -> Vec<String> {
    text.lines()
        .flat_map(|paragraph| wrap_paragraph(paragraph, style, max_width))
        .collect()
}

fn wrap_paragraph(text: &str, style: Style, max_width: f32) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    // only ascii spaces break, so a no-break space keeps "8 a.m." on one line
    for word in text.split_ascii_whitespace() {
        let joined = match rows.last() {
            Some(row) => format!("{row} {word}"),
            None => word.to_string(),
        };
        match rows.last_mut() {
            Some(row) if style.width(&joined) <= max_width => *row = joined,
            _ => rows.push(fit(word, style, max_width, style.size).0),
        }
    }
    // pull a word down so the last line never holds one word alone, like text-wrap: pretty
    if let [.., before, last] = rows.as_mut_slice()
        && !last.contains(' ')
        && let Some((head, tail)) = before.rsplit_once(' ')
    {
        let joined = format!("{tail} {last}");
        if style.width(&joined) <= max_width {
            *before = head.to_string();
            *last = joined;
        }
    }
    rows
}

pub(crate) fn hour_bars(
    canvas: &mut Canvas,
    by_hour: &[u64; 24],
    peak: usize,
    bottom: f32,
    tall: f32,
    accent: &str,
    other: &str,
) {
    let max = by_hour.iter().copied().max().unwrap_or(1).max(1);
    let slot = CONTENT / 24.0;
    for (hour, &count) in by_hour.iter().enumerate() {
        let height = bar(count, max, tall);
        let fill = if hour == peak { accent } else { other };
        canvas.raw(&rect(
            MARGIN + hour as f32 * slot + 4.0,
            bottom - height,
            slot - 8.0,
            height,
            3.0,
            fill,
        ));
    }
}

// the first label starts at its bar, so a wide one such as "midnight" stays inside the margin
pub(crate) fn axis_labels(canvas: &mut Canvas, axis: &[&str; 4], inset: f32, y: f32, style: Style) {
    let slot = CONTENT / 24.0;
    for (index, label) in axis.iter().enumerate() {
        if index == 0 {
            canvas.text(MARGIN + inset, y, label, style.anchor(Anchor::Start));
        } else {
            let x = MARGIN + (index * 6) as f32 * slot + slot / 2.0;
            canvas.text(x, y, label, style.anchor(Anchor::Middle));
        }
    }
}

pub(crate) fn bar(count: u64, max: u64, tall: f32) -> f32 {
    ((count as f64 / max as f64) as f32 * tall).max(4.0)
}

pub(crate) fn rect(x: f32, y: f32, width: f32, height: f32, radius: f32, fill: &str) -> String {
    format!(
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{width:.1}\" height=\"{height:.1}\" rx=\"{radius}\" fill=\"{fill}\"/>"
    )
}

fn round(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

// control characters are not allowed in xml and would make the slide unreadable
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out += "&amp;",
            '<' => out += "&lt;",
            '>' => out += "&gt;",
            '"' => out += "&quot;",
            '\'' => out += "&apos;",
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts;

    const MONO: Font = Font {
        family: fonts::MONO_FAMILY,
        weight: 400,
    };

    #[test]
    fn escapes_markup_and_drops_control_characters() {
        assert_eq!(escape("<b>&\"x'\u{1b}"), "&lt;b&gt;&amp;&quot;x&apos;");
    }

    #[test]
    fn shrinks_then_cuts_long_text() {
        let style = Style::new(MONO, 100.0, "#000");
        assert_eq!(fit("git", style, 900.0, 40.0), ("git".to_string(), 100.0));
        let (text, size) = fit("abcdefghijklmnopqrst", style, 900.0, 40.0);
        assert_eq!(text, "abcdefghijklmnopqrst");
        assert!((40.0..100.0).contains(&size), "{size}");
        let (cut, size) = fit(&"a".repeat(200), style, 900.0, 40.0);
        assert_eq!(size, 40.0);
        assert!(cut.ends_with('…'));
        assert!(style.size(40.0).width(&cut) <= 900.0);
    }

    #[test]
    fn wraps_words_into_lines_that_fit() {
        let style = Style::new(MONO, 30.0, "#000");
        let rows = wrap("Your keyboard has filed a complaint.", style, 400.0);
        assert!(rows.len() >= 2, "{rows:?}");
        assert_eq!(rows.join(" "), "Your keyboard has filed a complaint.");
        for row in &rows {
            assert!(style.width(row) <= 400.0, "{row}");
        }
        let long = wrap(&"x".repeat(100), style, 400.0);
        assert_eq!(long.len(), 1);
        assert!(long[0].ends_with('…'));
    }

    #[test]
    fn starts_a_new_line_at_each_break() {
        let style = Style::new(MONO, 30.0, "#000");
        assert_eq!(
            wrap("Small commits.\nBig energy.", style, 900.0),
            ["Small commits.", "Big energy."]
        );
        let held = wrap("between 5 and 8\u{a0}a.m.", style, 260.0);
        assert_eq!(held.last().map(String::as_str), Some("and 8\u{a0}a.m."));
    }

    #[test]
    fn keeps_a_lone_word_off_the_last_line() {
        let style = Style::new(MONO, 30.0, "#000");
        let rows = wrap("Rust ran your year.", style, 260.0);
        assert_eq!(rows, ["Rust ran", "your year."]);
    }

    #[test]
    fn records_where_each_line_lands() {
        let mut canvas = Canvas::new();
        let style = Style::new(MONO, 50.0, "#000").anchor(Anchor::End);
        let width = canvas.text(992.0, 200.0, "ab\u{7}c", style);
        let placed = &canvas.boxes[0];
        assert_eq!(placed.text, "abc");
        assert!((placed.right - 992.0).abs() < 0.01);
        assert!((placed.right - placed.left - width).abs() < 0.01);
        assert!(canvas.finish().contains(">abc</text>"));
    }
}

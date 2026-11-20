use std::fmt::Write;

use crate::draw::{self, Anchor, CONTENT, Canvas, HEIGHT, MARGIN, Scene, Style, WIDTH, bar, rect};
use crate::story::{Content, WEEKDAYS};
use crate::theme::{MONO, MONO_BOLD};

const RIGHT: f32 = WIDTH - MARGIN;
const BOTTOM: f32 = 1220.0;

#[derive(Clone, Copy)]
struct Glow {
    x: f32,
    y: f32,
    radius: f32,
    color: usize,
    opacity: f32,
}

const fn glow(x: f32, y: f32, radius: f32, color: usize, opacity: f32) -> Glow {
    Glow {
        x,
        y,
        radius,
        color,
        opacity,
    }
}

// each slide moves the glow, so a run of slides does not look the same
const GLOWS: [[Glow; 3]; 4] = [
    [
        glow(900.0, 260.0, 620.0, 0, 0.75),
        glow(160.0, 1150.0, 700.0, 1, 0.8),
        glow(980.0, 1250.0, 420.0, 2, 0.5),
    ],
    [
        glow(1000.0, 420.0, 560.0, 1, 0.85),
        glow(60.0, 1300.0, 600.0, 2, 0.55),
        glow(100.0, 80.0, 300.0, 0, 0.3),
    ],
    [
        glow(540.0, 700.0, 760.0, 0, 0.55),
        glow(1000.0, 120.0, 420.0, 2, 0.35),
        glow(40.0, 1300.0, 380.0, 1, 0.6),
    ],
    [
        glow(980.0, 200.0, 520.0, 0, 0.6),
        glow(100.0, 1250.0, 620.0, 1, 0.75),
        glow(900.0, 1300.0, 300.0, 2, 0.4),
    ],
];

pub(crate) fn draw(slide: &Scene, canvas: &mut Canvas) {
    frame(slide, canvas);
    let p = slide.palette;
    let lead = Style::new(slide.theme.display(400), 44.0, p.muted);
    let heavy = |size: f32, fill: &'static str| Style::new(slide.theme.display(900), size, fill);
    let body = Style::new(MONO, 34.0, p.text);
    let small = Style::new(MONO, 28.0, p.muted).spacing(1.0);

    match &slide.page.content {
        Content::Intro {
            year,
            lead: text,
            sections,
        } => {
            canvas.group("part", |c| {
                c.fit(
                    80.0,
                    520.0,
                    year,
                    heavy(250.0, p.text).spacing(-6.0),
                    CONTENT,
                    110.0,
                );
                c.text(
                    88.0,
                    700.0,
                    "wrapped.",
                    heavy(150.0, p.accent).spacing(-4.0),
                );
            });
            canvas.group("part", |c| {
                c.wrap(92.0, 820.0, text, body, CONTENT, 48.0);
                let names: Vec<&str> = sections.iter().map(|(name, _)| *name).collect();
                c.text(92.0, BOTTOM, &names.join("  ·  "), small.spacing(2.0));
            });
        }
        Content::Commits {
            commits,
            rate,
            line,
            week,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 240.0, "This year you shipped", lead);
                c.fit(
                    76.0,
                    500.0,
                    commits,
                    heavy(260.0, p.text).spacing(-10.0).class("value"),
                    CONTENT,
                    110.0,
                );
                c.text(
                    MARGIN,
                    640.0,
                    "commits.",
                    heavy(120.0, p.accent).spacing(-3.0),
                );
            });
            canvas.group("part", |c| {
                pill(c, MARGIN, 700.0, rate, slide);
                let mut y = c.wrap(92.0, 840.0, line, body, CONTENT, 48.0);
                if let Some(week) = week {
                    y += 6.0;
                    y = c.wrap(92.0, y, &week.busiest_line, small, CONTENT, 38.0);
                    c.wrap(92.0, y, &week.quietest_line, small, CONTENT, 38.0);
                }
            });
            if let Some(week) = week {
                canvas.group("part bars", |c| {
                    let max = week.counts.iter().copied().max().unwrap_or(1).max(1);
                    for (day, &count) in week.counts.iter().enumerate() {
                        let height = bar(count, max, 190.0);
                        let x = MARGIN + day as f32 * 132.0;
                        let busiest = day == week.busiest;
                        glass(
                            c,
                            x,
                            BOTTOM - 40.0 - height,
                            100.0,
                            height,
                            12.0,
                            if busiest { p.accent } else { p.text },
                            if busiest { 1.0 } else { 0.28 },
                        );
                        c.text(
                            x + 50.0,
                            BOTTOM,
                            &WEEKDAYS[day][..1],
                            small.anchor(Anchor::Middle),
                        );
                    }
                });
            }
        }
        Content::Streak {
            days,
            unit,
            line,
            length,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 240.0, "Your longest streak", lead);
                c.fit(
                    76.0,
                    590.0,
                    days,
                    heavy(360.0, p.accent).spacing(-12.0).class("value"),
                    CONTENT,
                    140.0,
                );
                c.fit(
                    MARGIN,
                    720.0,
                    unit,
                    heavy(80.0, p.text).spacing(-2.0),
                    CONTENT,
                    40.0,
                );
            });
            canvas.group("part", |c| {
                c.wrap(92.0, 830.0, line, body, CONTENT, 48.0);
            });
            canvas.group("part cells", |c| {
                let shown = (*length).min(28) as usize;
                for index in 0..28 {
                    let x = MARGIN + (index % 14) as f32 * 64.5;
                    let y = 990.0 + (index / 14) as f32 * 64.0;
                    let lit = index < shown;
                    glass(
                        c,
                        x,
                        y,
                        52.0,
                        52.0,
                        12.0,
                        if lit { p.accent } else { p.text },
                        if lit { 1.0 } else { 0.12 },
                    );
                }
            });
        }
        Content::Languages {
            headline,
            line,
            bars,
        } => {
            let mut y = 0.0;
            canvas.group("part", |c| {
                y = c.wrap(
                    MARGIN,
                    250.0,
                    headline,
                    heavy(92.0, p.text).spacing(-3.0),
                    CONTENT,
                    104.0,
                );
                y = c.wrap(
                    92.0,
                    y + 6.0,
                    line,
                    body.size(30.0).fill(p.muted),
                    CONTENT,
                    44.0,
                );
            });
            canvas.group("part hbars", |c| {
                let top = y + 70.0;
                for (index, (name, share)) in bars.iter().enumerate() {
                    let row = top + index as f32 * 100.0;
                    c.fit(
                        MARGIN,
                        row,
                        name,
                        Style::new(slide.theme.display(700), 38.0, p.text),
                        290.0,
                        28.0,
                    );
                    glass(c, 400.0, row - 34.0, 470.0, 40.0, 20.0, p.text, 0.12);
                    let width = (*share as f32 / 100.0 * 470.0).max(40.0);
                    let fill = if index == 0 { p.accent } else { p.text };
                    glass(
                        c,
                        400.0,
                        row - 34.0,
                        width,
                        40.0,
                        20.0,
                        fill,
                        if index == 0 { 1.0 } else { 0.6 },
                    );
                    c.text(
                        RIGHT,
                        row,
                        &format!("{share}%"),
                        Style::new(MONO_BOLD, 28.0, p.text).anchor(Anchor::End),
                    );
                }
            });
        }
        Content::PeakHour {
            hour,
            axis,
            line,
            by_hour,
            peak,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 240.0, "You peaked at", lead);
                c.fit(
                    76.0,
                    530.0,
                    hour,
                    heavy(280.0, p.text).spacing(-10.0),
                    CONTENT,
                    120.0,
                );
            });
            canvas.group("part", |c| {
                c.wrap(
                    MARGIN,
                    650.0,
                    line,
                    Style::new(slide.theme.display(700), 42.0, p.accent),
                    CONTENT,
                    56.0,
                );
            });
            canvas.group("part bars", |c| {
                let max = by_hour.iter().copied().max().unwrap_or(1).max(1);
                let slot = CONTENT / 24.0;
                for (hour, &count) in by_hour.iter().enumerate() {
                    let height = bar(count, max, 310.0);
                    let lit = hour == *peak;
                    glass(
                        c,
                        MARGIN + hour as f32 * slot + 5.0,
                        BOTTOM - 40.0 - height,
                        slot - 10.0,
                        height,
                        6.0,
                        if lit { p.accent } else { p.text },
                        if lit { 1.0 } else { 0.28 },
                    );
                }
                draw::axis_labels(c, axis, 5.0, BOTTOM, small);
            });
        }
        Content::Shell {
            commands,
            favorite,
            extra,
            rows,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 240.0, "You ran", lead);
                c.fit(
                    76.0,
                    490.0,
                    commands,
                    heavy(250.0, p.text).spacing(-10.0).class("value"),
                    CONTENT,
                    110.0,
                );
                c.text(
                    MARGIN,
                    620.0,
                    "commands.",
                    heavy(110.0, p.accent).spacing(-3.0),
                );
            });
            const PILL: f32 = 50.0;
            const ROW: f32 = 58.0;
            const GAP: f32 = 20.0;
            let rows_fit = |text_end: f32| {
                let room = BOTTOM - text_end - GAP - PILL;
                if room < 0.0 {
                    0
                } else {
                    (room / ROW).floor() as usize + 1
                }
            };
            let mut y = 700.0;
            canvas.group("part", |c| {
                if let Some(favorite) = favorite {
                    y = c.wrap(
                        MARGIN,
                        y,
                        favorite,
                        Style::new(slide.theme.display(700), 38.0, p.text),
                        CONTENT,
                        48.0,
                    );
                }
                if let Some(extra) = extra {
                    let extra_style = small.size(28.0);
                    let below = y + draw::wrap(extra, extra_style, CONTENT).len() as f32 * 36.0;
                    // the joke line gives way before a ranked row does
                    if rows_fit(below) >= rows.len().min(rows_fit(y)) {
                        y = c.wrap(92.0, y, extra, extra_style, CONTENT, 36.0);
                    }
                }
            });
            canvas.group("part", |c| {
                let top = y + GAP;
                for (index, (name, count)) in rows.iter().take(rows_fit(y)).enumerate() {
                    let row = top + index as f32 * ROW;
                    glass(c, MARGIN, row, CONTENT, PILL, PILL / 2.0, p.text, 0.07);
                    let width = c.text(
                        RIGHT - 28.0,
                        row + 35.0,
                        count,
                        Style::new(MONO_BOLD, 28.0, p.accent).anchor(Anchor::End),
                    );
                    c.fit(
                        MARGIN + 28.0,
                        row + 35.0,
                        name,
                        Style::new(MONO, 28.0, p.text),
                        CONTENT - width - 96.0,
                        28.0,
                    );
                }
            });
        }
        Content::Typo {
            typo,
            note,
            footnote,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 240.0, "The typo of the year", lead);
            });
            canvas.group("part", |c| {
                let size = c.fit(540.0, 720.0, typo, heavy(400.0, p.accent).spacing(-12.0).anchor(Anchor::Middle), CONTENT, 90.0);
                let half = c.boxes.last().map_or(300.0, |placed| (placed.right - placed.left) / 2.0);
                let y = 720.0 - size * 0.27;
                c.raw(&format!(
                    "<path d=\"M {:.1} {:.1} Q 540 {:.1} {:.1} {:.1}\" stroke=\"{}\" stroke-width=\"12\" fill=\"none\" stroke-linecap=\"round\"/>",
                    540.0 - half - 20.0,
                    y + 20.0,
                    y - 60.0,
                    540.0 + half + 20.0,
                    y + 30.0,
                    p.text
                ));
            });
            canvas.group("part", |c| {
                c.wrap(
                    540.0,
                    890.0,
                    note,
                    body.size(36.0).anchor(Anchor::Middle),
                    CONTENT,
                    50.0,
                );
                c.wrap(
                    540.0,
                    1060.0,
                    footnote,
                    small.anchor(Anchor::Middle),
                    CONTENT,
                    38.0,
                );
            });
        }
        Content::Agents {
            sessions,
            line,
            split,
            model,
            tools,
            models,
        } => {
            canvas.group("part", |c| {
                c.fit(
                    76.0,
                    400.0,
                    sessions,
                    heavy(280.0, p.text).spacing(-10.0).class("value"),
                    CONTENT,
                    110.0,
                );
            });
            let mut y = 500.0;
            canvas.group("part", |c| {
                y = c.wrap(
                    MARGIN,
                    y,
                    "sessions with your AI.",
                    heavy(64.0, p.accent).spacing(-2.0),
                    CONTENT,
                    74.0,
                );
                y = c.wrap(MARGIN, y + 4.0, line, lead.size(36.0), CONTENT, 48.0);
                y = c.wrap(92.0, y + 6.0, split, small, CONTENT, 40.0);
                if let Some(model) = model {
                    y = c.wrap(92.0, y, model, small, CONTENT, 40.0);
                }
            });
            let half = (CONTENT - 24.0) / 2.0;
            canvas.group("part", |c| {
                column(c, MARGIN, y + 30.0, half, "TOOLS", tools, slide);
                column(
                    c,
                    MARGIN + half + 24.0,
                    y + 30.0,
                    half,
                    "MODELS",
                    models,
                    slide,
                );
            });
        }
        Content::Tokens {
            total,
            cache,
            novels,
            bars,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 240.0, "Your AI went through", lead);
                c.fit(
                    76.0,
                    500.0,
                    total,
                    heavy(260.0, p.text).spacing(-10.0).class("value"),
                    CONTENT,
                    110.0,
                );
                c.text(
                    MARGIN,
                    640.0,
                    "tokens.",
                    heavy(120.0, p.accent).spacing(-3.0),
                );
            });
            let mut y = 0.0;
            canvas.group("part", |c| {
                y = c.wrap(92.0, 760.0, cache, body, CONTENT, 48.0);
                y = c.wrap(92.0, y, novels, small.size(28.0), CONTENT, 40.0);
            });
            canvas.group("part hbars", |c| {
                let largest = bars
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, (_, _, share))| *share)
                    .map_or(0, |(index, _)| index);
                let top = (y + 60.0).max(920.0);
                for (index, (label, count, share)) in bars.iter().enumerate() {
                    let row = top + index as f32 * 64.0;
                    if row > BOTTOM {
                        break;
                    }
                    c.text(MARGIN, row, label, Style::new(MONO, 28.0, p.text));
                    glass(c, 340.0, row - 26.0, 440.0, 32.0, 16.0, p.text, 0.12);
                    let width = (*share as f32 / 100.0 * 440.0).max(32.0);
                    let lit = index == largest;
                    glass(
                        c,
                        340.0,
                        row - 26.0,
                        width,
                        32.0,
                        16.0,
                        if lit { p.accent } else { p.text },
                        if lit { 1.0 } else { 0.6 },
                    );
                    c.text(
                        RIGHT,
                        row,
                        count,
                        Style::new(MONO_BOLD, 28.0, p.accent).anchor(Anchor::End),
                    );
                }
            });
        }
        Content::Award {
            label,
            name,
            reason,
            also,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 240.0, label, lead);
            });
            canvas.group("part", |c| {
                c.raw(&rect(MARGIN, 330.0, CONTENT, 290.0, 145.0, p.accent));
                c.text(
                    MARGIN + 90.0,
                    432.0,
                    "THE AWARD GOES TO",
                    Style::new(MONO_BOLD, 28.0, p.background).spacing(4.0),
                );
                c.fit(
                    MARGIN + 86.0,
                    548.0,
                    name,
                    heavy(100.0, p.background).spacing(-2.0),
                    CONTENT - 172.0,
                    40.0,
                );
            });
            canvas.group("part", |c| {
                let y = c.wrap(
                    MARGIN,
                    740.0,
                    reason,
                    Style::new(slide.theme.display(400), 40.0, p.text),
                    CONTENT,
                    56.0,
                );
                if let Some(also) = also {
                    c.wrap(92.0, y + 30.0, also, small.size(30.0), CONTENT, 42.0);
                }
            });
        }
        Content::Repositories { line, rows } => {
            let mut y = 0.0;
            canvas.group("part", |c| {
                y = c.wrap(
                    MARGIN,
                    250.0,
                    "Where your commits lived",
                    heavy(84.0, p.text).spacing(-3.0),
                    CONTENT,
                    96.0,
                );
                y = c.wrap(
                    92.0,
                    y + 6.0,
                    line,
                    body.size(30.0).fill(p.muted),
                    CONTENT,
                    44.0,
                );
            });
            canvas.group("part", |c| {
                for (index, (name, count)) in rows.iter().enumerate() {
                    let row = y + 50.0 + index as f32 * 110.0;
                    glass(c, MARGIN, row, CONTENT, 92.0, 46.0, p.text, 0.07);
                    let width = c.text(
                        RIGHT - 44.0,
                        row + 60.0,
                        count,
                        Style::new(MONO_BOLD, 32.0, p.accent).anchor(Anchor::End),
                    );
                    c.fit(
                        MARGIN + 44.0,
                        row + 62.0,
                        name,
                        Style::new(slide.theme.display(700), 40.0, p.text),
                        CONTENT - width - 128.0,
                        28.0,
                    );
                }
            });
        }
        Content::Summary { year, award, rows } => {
            canvas.group("part", |c| {
                let title = heavy(84.0, p.text).spacing(-3.0);
                let (year_text, size) = draw::fit(year, title, CONTENT * 0.5, 40.0);
                let width = c.text(MARGIN, 250.0, &year_text, title.size(size));
                c.text(
                    MARGIN + width + 30.0,
                    250.0,
                    "wrapped",
                    heavy(size, p.accent).spacing(-3.0),
                );
            });
            canvas.group("part", |c| {
                c.raw(&rect(MARGIN, 300.0, CONTENT, 150.0, 75.0, p.accent));
                c.text(
                    MARGIN + 60.0,
                    356.0,
                    "HEADLINE AWARD",
                    Style::new(MONO_BOLD, 28.0, p.background).spacing(4.0),
                );
                c.fit(
                    MARGIN + 58.0,
                    418.0,
                    award,
                    heavy(50.0, p.background).spacing(-1.0),
                    CONTENT - 116.0,
                    28.0,
                );
            });
            canvas.group("part", |c| {
                for (index, (label, value)) in rows.iter().enumerate() {
                    let x = MARGIN + (index % 2) as f32 * 460.0;
                    let y = 500.0 + (index / 2) as f32 * 226.0;
                    glass(c, x, y, 444.0, 192.0, 28.0, p.text, 0.07);
                    c.fit(
                        x + 36.0,
                        y + 93.0,
                        value,
                        Style::new(slide.theme.display(800), 62.0, p.text)
                            .spacing(-2.0)
                            .class("value"),
                        372.0,
                        28.0,
                    );
                    c.text(x + 38.0, y + 145.0, label, Style::new(MONO, 28.0, p.muted));
                }
            });
        }
    }
}

fn frame(slide: &Scene, canvas: &mut Canvas) {
    let p = slide.palette;
    let number = slide.number;
    let preset = &GLOWS[(number - 1) % GLOWS.len()];
    let mut defs = String::from("<defs>");
    let mut blobs = String::new();
    for (index, glow) in preset.iter().enumerate() {
        let Glow {
            x: cx,
            y: cy,
            radius: r,
            color,
            opacity,
        } = *glow;
        let color = p.glows[color];
        let _ = write!(
            defs,
            "<radialGradient id=\"s{number}-glow{index}\"><stop offset=\"0\" stop-color=\"{color}\" stop-opacity=\"{opacity}\"/>\
             <stop offset=\"1\" stop-color=\"{color}\" stop-opacity=\"0\"/></radialGradient>"
        );
        let _ = write!(
            blobs,
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" fill=\"url(#s{number}-glow{index})\"/>"
        );
    }
    defs += "</defs>";
    canvas.raw(&defs);
    canvas.raw(&rect(0.0, 0.0, WIDTH, HEIGHT, 0.0, p.background));
    canvas.raw(&blobs);

    let label = Style::new(MONO, 28.0, p.muted).spacing(2.0);
    canvas.text(
        MARGIN,
        104.0,
        &format!("devwrapped · {}", slide.year),
        label,
    );
    canvas.text(
        RIGHT,
        104.0,
        &format!("{:02} / {:02}", slide.number, slide.total),
        label.anchor(Anchor::End),
    );
    let gap = 12.0;
    let segment = (CONTENT - gap * (slide.total as f32 - 1.0)) / slide.total as f32;
    for index in 0..slide.total {
        let x = MARGIN + index as f32 * (segment + gap);
        let opacity = if index + 1 < slide.number { 0.9 } else { 0.18 };
        glass(canvas, x, 132.0, segment, 6.0, 3.0, p.text, opacity);
        // the html story grows this segment as the slide enters
        if index + 1 == slide.number {
            canvas.raw(&format!(
                "<rect class=\"progress\" x=\"{x:.1}\" y=\"132\" width=\"{segment:.1}\" height=\"6\" rx=\"3\" \
                 fill=\"{}\" fill-opacity=\"0.9\"/>",
                p.text
            ));
        }
    }
    canvas.text(
        WIDTH / 2.0,
        HEIGHT - 56.0,
        "cargo install devwrapped",
        label.anchor(Anchor::Middle),
    );
}

#[allow(clippy::too_many_arguments)]
fn column(
    canvas: &mut Canvas,
    x: f32,
    top: f32,
    width: f32,
    title: &str,
    rows: &[(String, String)],
    slide: &Scene,
) {
    if rows.is_empty() {
        return;
    }
    let p = slide.palette;
    canvas.text(
        x + 4.0,
        top,
        title,
        Style::new(MONO_BOLD, 28.0, p.muted).spacing(3.0),
    );
    for (index, (name, count)) in rows.iter().enumerate() {
        let y = top + 24.0 + index as f32 * 64.0;
        if y + 54.0 > BOTTOM + 30.0 {
            break;
        }
        glass(canvas, x, y, width, 54.0, 27.0, p.text, 0.07);
        let counted = canvas.text(
            x + width - 22.0,
            y + 37.0,
            count,
            Style::new(MONO_BOLD, 28.0, p.accent).anchor(Anchor::End),
        );
        canvas.fit(
            x + 22.0,
            y + 37.0,
            name,
            Style::new(MONO, 28.0, p.text),
            width - counted - 64.0,
            28.0,
        );
    }
}

fn pill(canvas: &mut Canvas, x: f32, y: f32, text: &str, slide: &Scene) {
    let style = Style::new(slide.theme.display(700), 30.0, slide.palette.background);
    let width = draw::fit(text, style, CONTENT - 60.0, 28.0);
    let measured =
        crate::measure::width(&width.0, style.font.family, style.font.weight, width.1, 0.0);
    canvas.raw(&rect(
        x,
        y,
        measured + 60.0,
        62.0,
        31.0,
        slide.palette.accent,
    ));
    canvas.text(x + 30.0, y + 42.0, &width.0, style.size(width.1));
}

#[allow(clippy::too_many_arguments)]
fn glass(
    canvas: &mut Canvas,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    fill: &str,
    opacity: f32,
) {
    canvas.raw(&format!(
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{width:.1}\" height=\"{height:.1}\" rx=\"{radius}\" fill=\"{fill}\" fill-opacity=\"{opacity}\"/>"
    ));
}

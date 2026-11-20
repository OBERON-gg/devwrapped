use crate::draw::{
    Anchor, CONTENT, Canvas, HEIGHT, MARGIN, Scene, Style, WIDTH, axis_labels, bar, hour_bars, rect,
};
use crate::story::{Content, WEEKDAYS};
use crate::theme::{MONO, MONO_BOLD};

const RIGHT: f32 = WIDTH - MARGIN;
const FOOTER_RULE: f32 = HEIGHT - 120.0;

pub(crate) fn draw(slide: &Scene, canvas: &mut Canvas) {
    frame(slide, canvas);
    let p = slide.palette;
    let light = Style::new(slide.theme.display(300), 64.0, p.text).spacing(-2.0);
    let bold = |size: f32, fill: &'static str| Style::new(slide.theme.display(700), size, fill);
    let note = Style::new(MONO, 28.0, p.muted);

    match &slide.page.content {
        Content::Intro {
            year,
            lead,
            sections,
        } => {
            canvas.group("part", |c| {
                c.text(
                    80.0,
                    320.0,
                    "Your",
                    Style::new(slide.theme.display(300), 150.0, p.text).spacing(-6.0),
                );
                c.fit(
                    80.0,
                    490.0,
                    &format!("{year},"),
                    bold(190.0, p.text).spacing(-8.0),
                    CONTENT,
                    90.0,
                );
            });
            canvas.group("part", |c| {
                let width = c.text(80.0, 660.0, "wrapped.", bold(190.0, p.accent).spacing(-8.0));
                c.raw(&squiggle(92.0, 695.0, width.min(CONTENT), p.second));
            });
            let mut y = canvas.wrap(MARGIN, 770.0, lead, note, CONTENT, 38.0);
            y += 24.0;
            canvas.group("part", |c| {
                c.text(
                    MARGIN,
                    y,
                    "IN THIS ISSUE",
                    Style::new(MONO_BOLD, 28.0, p.muted).spacing(3.0),
                );
                for (index, (name, about)) in sections.iter().enumerate() {
                    let row = y + 58.0 + index as f32 * 62.0;
                    c.text(
                        MARGIN,
                        row,
                        &format!("{:02}", index + 1),
                        Style::new(MONO_BOLD, 28.0, p.accent),
                    );
                    c.text(MARGIN + 70.0, row, name, bold(32.0, p.text));
                    c.text(RIGHT, row, about, note.anchor(Anchor::End));
                    c.raw(&hairline(row + 22.0, p.text));
                }
            });
        }
        Content::Commits {
            commits,
            rate,
            line,
            week,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 220.0, "You made", light);
                c.fit(
                    70.0,
                    480.0,
                    commits,
                    bold(280.0, p.text).spacing(-14.0).class("value"),
                    CONTENT,
                    120.0,
                );
            });
            canvas.group("part", |c| {
                let width = c.text(
                    MARGIN,
                    600.0,
                    "commits*",
                    bold(110.0, p.accent).spacing(-4.0),
                );
                let cx = (MARGIN + width + 190.0).min(RIGHT - 150.0);
                c.raw(&ellipse(cx, 570.0, 150.0, 64.0, -8.0, p.second));
                c.fit(
                    cx,
                    582.0,
                    rate,
                    bold(34.0, p.second).anchor(Anchor::Middle),
                    250.0,
                    28.0,
                );
            });
            canvas.group("part", |c| {
                let mut lines = vec![*line];
                if let Some(week) = week {
                    lines.extend([week.busiest_line.as_str(), week.quietest_line.as_str()]);
                }
                footnote(c, 700.0, &lines, note);
            });
            if let Some(week) = week {
                canvas.group("part bars", |c| {
                    let max = week.counts.iter().copied().max().unwrap_or(1).max(1);
                    for (day, &count) in week.counts.iter().enumerate() {
                        let height = bar(count, max, 260.0);
                        let x = MARGIN + day as f32 * 132.0;
                        let fill = if day == week.busiest {
                            p.accent
                        } else {
                            p.second
                        };
                        c.raw(&rect(x, 1180.0 - height, 100.0, height, 0.0, fill));
                        let letter = &WEEKDAYS[day][..1];
                        c.text(
                            x + 50.0,
                            1218.0,
                            letter,
                            Style::new(MONO_BOLD, 28.0, p.text).anchor(Anchor::Middle),
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
                c.text(MARGIN, 220.0, "Your longest streak", light);
                c.fit(
                    70.0,
                    580.0,
                    days,
                    bold(380.0, p.accent).spacing(-16.0).class("value"),
                    CONTENT,
                    140.0,
                );
            });
            canvas.group("part", |c| {
                c.fit(
                    MARGIN,
                    710.0,
                    unit,
                    bold(96.0, p.text).spacing(-3.0),
                    CONTENT,
                    48.0,
                );
                c.wrap(MARGIN, 800.0, line, note.size(30.0), CONTENT, 42.0);
            });
            canvas.group("part cells", |c| {
                let shown = (*length).min(28) as usize;
                for index in 0..28 {
                    let x = MARGIN + (index % 14) as f32 * 64.5;
                    let y = 1000.0 + (index / 14) as f32 * 64.0;
                    let fill = if index < shown { p.second } else { p.text };
                    let opacity = if index < shown { "1" } else { "0.12" };
                    c.raw(&format!(
                        "<rect x=\"{x}\" y=\"{y}\" width=\"52\" height=\"52\" fill=\"{fill}\" fill-opacity=\"{opacity}\"/>"
                    ));
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
                    bold(110.0, p.text).spacing(-4.0),
                    CONTENT,
                    112.0,
                );
            });
            canvas.group("part", |c| {
                y = c.wrap(MARGIN, y + 10.0, line, note.size(30.0), CONTENT, 42.0);
            });
            canvas.group("part hbars", |c| {
                let top = y + 60.0;
                for (index, (name, share)) in bars.iter().enumerate() {
                    let row = top + index as f32 * 96.0;
                    c.fit(MARGIN, row, name, bold(40.0, p.text), 300.0, 28.0);
                    let width = ((*share as f32 / 100.0) * 470.0).max(6.0);
                    let fill = if index == 0 { p.accent } else { p.second };
                    c.raw(&rect(400.0, row - 34.0, width, 40.0, 0.0, fill));
                    bar_value(
                        c,
                        400.0 + width,
                        row,
                        &format!("{share}%"),
                        Style::new(MONO_BOLD, 30.0, p.text),
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
                c.text(MARGIN, 220.0, "You peaked at", light);
                c.fit(
                    70.0,
                    520.0,
                    hour,
                    bold(300.0, p.accent).spacing(-12.0),
                    CONTENT,
                    120.0,
                );
            });
            canvas.group("part", |c| {
                c.wrap(
                    MARGIN,
                    640.0,
                    line,
                    bold(44.0, p.text).spacing(-1.0),
                    CONTENT,
                    54.0,
                );
            });
            canvas.group("part bars", |c| {
                hour_bars(c, by_hour, *peak, 1170.0, 300.0, p.accent, p.second);
                axis_labels(c, axis, 4.0, 1210.0, Style::new(MONO_BOLD, 28.0, p.text));
            });
        }
        Content::Shell {
            commands,
            favorite,
            extra,
            rows,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 220.0, "You ran", light);
                c.fit(
                    70.0,
                    470.0,
                    commands,
                    bold(260.0, p.text).spacing(-12.0).class("value"),
                    CONTENT,
                    120.0,
                );
                c.text(
                    MARGIN,
                    590.0,
                    "commands.",
                    bold(110.0, p.accent).spacing(-4.0),
                );
            });
            let mut y = 690.0;
            canvas.group("part", |c| {
                if let Some(favorite) = favorite {
                    y = c.wrap(
                        MARGIN,
                        y,
                        favorite,
                        bold(40.0, p.text).spacing(-1.0),
                        CONTENT,
                        50.0,
                    );
                }
                if let Some(extra) = extra {
                    y = c.wrap(MARGIN, y, extra, note, CONTENT, 40.0);
                }
            });
            canvas.group("part", |c| {
                table(c, y + 40.0, rows, 54.0, slide, 30.0, false)
            });
        }
        Content::Typo {
            typo,
            note: meant,
            footnote,
        } => {
            canvas.group("part", |c| {
                c.text(MARGIN, 220.0, "The typo of the year", light);
            });
            canvas.group("part", |c| {
                let size = c.fit(
                    540.0,
                    690.0,
                    typo,
                    bold(420.0, p.accent).spacing(-16.0).anchor(Anchor::Middle),
                    720.0,
                    90.0,
                );
                let half = c
                    .boxes
                    .last()
                    .map_or(300.0, |placed| (placed.right - placed.left) / 2.0);
                c.raw(&loop_mark(
                    540.0,
                    690.0 - size * 0.3,
                    half + 70.0,
                    size * 0.55,
                    p.second,
                ));
            });
            canvas.group("part", |c| {
                c.raw(&format!(
                    "<path d=\"M 880 830 L 790 770\" stroke=\"{}\" stroke-width=\"6\" fill=\"none\" stroke-linecap=\"round\"/>",
                    p.second
                ));
                c.wrap(560.0, 890.0, meant, bold(36.0, p.second), RIGHT - 560.0, 46.0);
                c.wrap(MARGIN, 1040.0, footnote, note.size(30.0), CONTENT, 42.0);
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
                    70.0,
                    400.0,
                    sessions,
                    bold(280.0, p.text).spacing(-12.0).class("value"),
                    CONTENT,
                    120.0,
                );
            });
            let mut y = 500.0;
            canvas.group("part", |c| {
                y = c.wrap(
                    MARGIN,
                    y,
                    "sessions with your AI.",
                    bold(80.0, p.accent).spacing(-3.0),
                    CONTENT,
                    86.0,
                );
                y = c.wrap(MARGIN, y + 4.0, line, light.size(42.0), CONTENT, 52.0);
                y = c.wrap(MARGIN, y + 6.0, split, note, CONTENT, 40.0);
                if let Some(model) = model {
                    y = c.wrap(MARGIN, y, model, note, CONTENT, 40.0);
                }
            });
            let half = (CONTENT - 44.0) / 2.0;
            canvas.group("part", |c| {
                column(c, MARGIN, y + 40.0, half, "TOOLS", tools, slide);
                column(
                    c,
                    MARGIN + half + 44.0,
                    y + 40.0,
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
                c.text(MARGIN, 220.0, "Your AI went through", light);
                c.fit(
                    70.0,
                    480.0,
                    total,
                    bold(280.0, p.text).spacing(-14.0).class("value"),
                    CONTENT,
                    120.0,
                );
            });
            let mut y = 0.0;
            canvas.group("part", |c| {
                c.text(
                    MARGIN,
                    600.0,
                    "tokens*",
                    bold(110.0, p.accent).spacing(-4.0),
                );
                y = footnote(c, 700.0, &[cache.as_str(), novels.as_str()], note);
            });
            canvas.group("part hbars", |c| {
                let largest = bars
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, (_, _, share))| *share)
                    .map_or(0, |(index, _)| index);
                let top = (y + 70.0).max(880.0);
                for (index, (label, count, share)) in bars.iter().enumerate() {
                    let row = top + index as f32 * 70.0;
                    if row > FOOTER_RULE - 30.0 {
                        break;
                    }
                    c.text(MARGIN, row, label, Style::new(MONO, 28.0, p.text));
                    let width = (*share as f32 / 100.0 * 400.0).max(6.0);
                    let fill = if index == largest { p.accent } else { p.second };
                    c.raw(&rect(380.0, row - 28.0, width, 34.0, 0.0, fill));
                    bar_value(
                        c,
                        380.0 + width,
                        row,
                        count,
                        Style::new(MONO_BOLD, 28.0, p.text),
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
                c.text(MARGIN, 220.0, label, light);
            });
            canvas.group("part", |c| stamp(c, 540.0, 580.0, 290.0, name, slide, 74.0));
            canvas.group("part", |c| {
                let y = c.wrap(
                    MARGIN,
                    950.0,
                    reason,
                    light.size(40.0).spacing(-1.0),
                    CONTENT,
                    56.0,
                );
                if let Some(also) = also {
                    c.wrap(MARGIN, y + 20.0, also, note, CONTENT, 40.0);
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
                    bold(100.0, p.text).spacing(-4.0),
                    CONTENT,
                    104.0,
                );
                y = c.wrap(MARGIN, y + 10.0, line, note.size(30.0), CONTENT, 42.0);
            });
            canvas.group("part", |c| {
                table(c, y + 90.0, rows, 92.0, slide, 56.0, true)
            });
        }
        Content::Summary { year, award, rows } => {
            canvas.group("part", |c| {
                c.fit(
                    80.0,
                    270.0,
                    &format!("{year},"),
                    bold(150.0, p.text).spacing(-6.0),
                    600.0,
                    70.0,
                );
                c.text(80.0, 410.0, "wrapped.", bold(150.0, p.accent).spacing(-6.0));
            });
            canvas.group("part", |c| {
                stamp(c, 870.0, 276.0, 124.0, award, slide, 38.0)
            });
            let named: Vec<(String, String)> = rows
                .iter()
                .map(|(label, value)| (label.to_string(), value.clone()))
                .collect();
            canvas.group("part", |c| {
                table(c, 580.0, &named, 100.0, slide, 56.0, true)
            });
        }
    }
}

fn frame(slide: &Scene, canvas: &mut Canvas) {
    let p = slide.palette;
    canvas.raw(&rect(0.0, 0.0, WIDTH, HEIGHT, 0.0, p.background));
    let label = Style::new(MONO, 28.0, p.muted).spacing(3.0);
    let number = canvas.text(
        MARGIN,
        104.0,
        &format!("No. {:02}", slide.number),
        Style::new(MONO_BOLD, 28.0, p.text).spacing(1.0),
    );
    canvas.text(
        MARGIN + number + 14.0,
        104.0,
        &format!("/ {:02}", slide.total),
        Style::new(MONO, 28.0, p.muted).spacing(1.0),
    );
    canvas.text(
        RIGHT,
        104.0,
        &slide.page.section.to_uppercase(),
        label.anchor(Anchor::End),
    );
    canvas.raw(&format!(
        "<line x1=\"{MARGIN}\" y1=\"132\" x2=\"{RIGHT}\" y2=\"132\" stroke=\"{}\" stroke-width=\"2\"/>\n\
         <line x1=\"{MARGIN}\" y1=\"{FOOTER_RULE}\" x2=\"{RIGHT}\" y2=\"{FOOTER_RULE}\" stroke=\"{}\" stroke-width=\"2\"/>",
        p.text, p.text
    ));
    let footer = Style::new(MONO, 28.0, p.muted).spacing(1.0);
    canvas.text(
        MARGIN,
        HEIGHT - 72.0,
        &format!("devwrapped / {}", slide.year),
        footer,
    );
    canvas.text(
        RIGHT,
        HEIGHT - 72.0,
        "cargo install devwrapped",
        footer.anchor(Anchor::End),
    );
}

fn table(
    canvas: &mut Canvas,
    top: f32,
    rows: &[(String, String)],
    step: f32,
    slide: &Scene,
    size: f32,
    display: bool,
) {
    let p = slide.palette;
    for (index, (label, value)) in rows.iter().enumerate() {
        let y = top + index as f32 * step;
        let value_style = if display {
            Style::new(slide.theme.display(700), size, p.text)
                .spacing(-2.0)
                .anchor(Anchor::End)
        } else {
            Style::new(MONO_BOLD, size, p.text).anchor(Anchor::End)
        };
        let label_style = Style::new(MONO, 30.0, if display { p.muted } else { p.text });
        let value_width =
            crate::draw::fit(value, value_style, CONTENT * 0.45, (size * 0.6).max(28.0));
        canvas.text(RIGHT, y, &value_width.0, value_style.size(value_width.1));
        let room = CONTENT
            - (canvas
                .boxes
                .last()
                .map_or(0.0, |placed| placed.right - placed.left))
            - 30.0;
        canvas.fit(MARGIN, y, label, label_style, room, 28.0);
        canvas.raw(&hairline(y + size * 0.45, p.text));
    }
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
        x,
        top,
        title,
        Style::new(MONO_BOLD, 28.0, p.muted).spacing(3.0),
    );
    for (index, (name, count)) in rows.iter().enumerate() {
        let y = top + 54.0 + index as f32 * 54.0;
        if y > FOOTER_RULE - 30.0 {
            break;
        }
        let counted = canvas.text(
            x + width,
            y,
            count,
            Style::new(MONO_BOLD, 28.0, p.text).anchor(Anchor::End),
        );
        canvas.fit(
            x,
            y,
            name,
            Style::new(MONO, 28.0, p.text),
            width - counted - 24.0,
            28.0,
        );
        canvas.raw(&format!(
            "<line x1=\"{x:.1}\" y1=\"{0:.1}\" x2=\"{1:.1}\" y2=\"{0:.1}\" stroke=\"{2}\" stroke-opacity=\"0.22\" stroke-width=\"2\"/>",
            y + 18.0,
            x + width,
            p.text
        ));
    }
}

// the marker hangs at the margin and every line of the note starts after it
fn footnote(canvas: &mut Canvas, top: f32, lines: &[&str], style: Style) -> f32 {
    // two monospace advances, the width of "* "
    let indent = style.size * 1.2;
    canvas.text(MARGIN, top, "*", style);
    lines.iter().fold(top, |y, line| {
        canvas.wrap(MARGIN + indent, y, line, style, CONTENT - indent, 40.0)
    })
}

fn bar_value(canvas: &mut Canvas, bar_end: f32, y: f32, text: &str, style: Style) {
    let width = crate::measure::width(
        text,
        style.font.family,
        style.font.weight,
        style.size,
        style.spacing,
    );
    canvas.text((bar_end + 20.0).min(RIGHT - width), y, text, style);
}

fn stamp(canvas: &mut Canvas, cx: f32, cy: f32, radius: f32, name: &str, slide: &Scene, size: f32) {
    let p = slide.palette;
    canvas.raw(&format!(
        "<g transform=\"rotate(-10 {cx} {cy})\">\n\
         <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\" fill=\"none\" stroke=\"{0}\" stroke-width=\"{1}\"/>\n\
         <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{2}\" fill=\"none\" stroke=\"{0}\" stroke-width=\"2\"/>",
        p.accent,
        (radius / 22.0).max(4.0),
        radius * 0.85
    ));
    let words: Vec<&str> = name.split_whitespace().collect();
    let room = radius * 1.4;
    let word_style = Style::new(slide.theme.display(700), size, p.accent).anchor(Anchor::Middle);
    let line = size * 1.0;
    let top = cy - (words.len() as f32 - 1.0) * line / 2.0 + size * 0.45;
    canvas.text(
        cx,
        top - size * 1.05,
        "AWARDED",
        Style::new(MONO_BOLD, (size * 0.55).max(28.0), p.accent)
            .spacing(4.0)
            .anchor(Anchor::Middle),
    );
    for (index, word) in words.iter().enumerate() {
        canvas.fit(
            cx,
            top + index as f32 * line,
            word,
            word_style,
            room,
            (size * 0.5).max(28.0),
        );
    }
    canvas.raw("</g>");
}

fn hairline(y: f32, color: &str) -> String {
    format!(
        "<line x1=\"{MARGIN}\" y1=\"{y:.1}\" x2=\"{RIGHT}\" y2=\"{y:.1}\" stroke=\"{color}\" stroke-opacity=\"0.22\" stroke-width=\"2\"/>"
    )
}

fn squiggle(x: f32, y: f32, width: f32, color: &str) -> String {
    format!(
        "<path d=\"M {x} {y} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}\" stroke=\"{color}\" stroke-width=\"10\" fill=\"none\" stroke-linecap=\"round\"/>",
        x + width * 0.3,
        y - 22.0,
        x + width * 0.6,
        y + 20.0,
        x + width * 0.95,
        y - 8.0
    )
}

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, angle: f32, color: &str) -> String {
    format!(
        "<ellipse cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"5\" transform=\"rotate({angle} {cx:.1} {cy:.1})\"/>"
    )
}

// a hand-drawn loop that does not quite close, like a pen circling a word
fn loop_mark(cx: f32, cy: f32, rx: f32, ry: f32, color: &str) -> String {
    format!(
        "<path d=\"M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}\" \
         stroke=\"{color}\" stroke-width=\"8\" fill=\"none\" stroke-linecap=\"round\"/>",
        cx - rx * 0.8,
        cy - ry * 0.9,
        cx - rx * 1.25,
        cy - ry * 0.1,
        cx - rx * 0.6,
        cy + ry * 1.1,
        cx + rx * 0.1,
        cy + ry * 1.0,
        cx + rx * 1.2,
        cy + ry * 0.9,
        cx + rx * 1.15,
        cy - ry * 0.8,
        cx + rx * 0.3,
        cy - ry * 1.05,
        cx - rx * 0.3,
        cy - ry * 1.15,
        cx - rx * 0.75,
        cy - ry * 0.95,
        cx - rx * 0.95,
        cy - ry * 0.6
    )
}

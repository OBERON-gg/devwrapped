use crate::draw::{
    Anchor, CONTENT, Canvas, HEIGHT, MARGIN, Scene, Style, WIDTH, axis_labels, bar, hour_bars, rect,
};
use crate::story::{Content, WEEKDAYS};
use crate::theme::{MONO, MONO_BOLD};

const RIGHT: f32 = WIDTH - MARGIN;
const WHITE: &str = "#ffffff";
const HEADER: f32 = 96.0;
const LEAD: f32 = 226.0;
const FOOTER_RULE: f32 = HEIGHT - 110.0;

pub(crate) fn draw(slide: &Scene, canvas: &mut Canvas) {
    let p = slide.palette;
    canvas.raw(&rect(0.0, 0.0, WIDTH, HEIGHT, 0.0, p.background));
    footer(slide, canvas);
    let heavy = |size: f32, fill: &'static str| {
        Style::new(slide.theme.display(700), size, fill).spacing(-size * 0.045)
    };
    let lead = Style::new(slide.theme.display(500), 56.0, WHITE).spacing(-1.5);
    let note = Style::new(MONO, 28.0, p.text);

    match &slide.page.content {
        Content::Intro {
            year,
            lead: intro,
            sections,
        } => {
            panel(slide, canvas, 640.0, true);
            canvas.group("part", |c| {
                c.text(MARGIN, 270.0, "Your", lead.size(80.0));
                c.fit(
                    76.0,
                    560.0,
                    &format!("{year},"),
                    heavy(290.0, WHITE),
                    CONTENT,
                    120.0,
                );
            });
            canvas.group("part", |c| {
                c.text(MARGIN, 790.0, "wrapped.", heavy(170.0, p.accent));
            });
            let y = canvas.wrap(MARGIN, 870.0, intro, note, CONTENT, 38.0);
            canvas.group("part", |c| {
                let top = y + 20.0;
                c.text(
                    MARGIN,
                    top,
                    "IN THIS ISSUE",
                    Style::new(MONO_BOLD, 28.0, p.text).spacing(3.0),
                );
                for (index, (name, about)) in sections.iter().enumerate() {
                    let row = top + 56.0 + index as f32 * 56.0;
                    rule(c, row - 38.0, p.text, 2.0);
                    c.text(
                        MARGIN,
                        row,
                        &format!("{:02}", index + 1),
                        Style::new(MONO_BOLD, 28.0, p.accent),
                    );
                    c.text(MARGIN + 70.0, row, name, heavy(32.0, p.text).spacing(0.0));
                    c.text(RIGHT, row, about, note.anchor(Anchor::End));
                }
            });
        }
        Content::Commits {
            commits,
            rate,
            line,
            week,
        } => {
            panel(slide, canvas, 590.0, true);
            canvas.group("part", |c| {
                c.text(MARGIN, LEAD, "You made", lead);
                c.fit(
                    76.0,
                    510.0,
                    commits,
                    heavy(290.0, WHITE).class("value"),
                    CONTENT,
                    120.0,
                );
            });
            canvas.group("part", |c| {
                let width = c.text(MARGIN, 720.0, "commits.", heavy(120.0, p.accent));
                tag(c, MARGIN + width + 36.0, 720.0, rate, slide);
            });
            canvas.group("part", |c| {
                let mut lines = vec![*line];
                if let Some(week) = week {
                    lines.extend([week.busiest_line.as_str(), week.quietest_line.as_str()]);
                }
                lines.iter().fold(810.0, |y, line| {
                    c.wrap(MARGIN, y, line, note, CONTENT, 40.0)
                });
            });
            if let Some(week) = week {
                canvas.group("part bars", |c| {
                    let max = week.counts.iter().copied().max().unwrap_or(1).max(1);
                    let slot = CONTENT / 7.0;
                    for (day, &count) in week.counts.iter().enumerate() {
                        let height = bar(count, max, 170.0);
                        let x = MARGIN + day as f32 * slot;
                        let fill = if day == week.busiest {
                            p.accent
                        } else {
                            p.text
                        };
                        c.raw(&rect(x, 1160.0 - height, slot - 16.0, height, 0.0, fill));
                        c.text(
                            x,
                            1200.0,
                            &WEEKDAYS[day][..3].to_uppercase(),
                            Style::new(MONO_BOLD, 28.0, p.text),
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
            panel(slide, canvas, 660.0, true);
            canvas.group("part", |c| {
                c.text(MARGIN, LEAD, "Your longest streak", lead);
                c.fit(
                    76.0,
                    590.0,
                    days,
                    heavy(380.0, WHITE).class("value"),
                    CONTENT,
                    140.0,
                );
            });
            canvas.group("part", |c| {
                c.fit(MARGIN, 790.0, unit, heavy(110.0, p.accent), CONTENT, 48.0);
                c.wrap(MARGIN, 870.0, line, note.size(30.0), CONTENT, 42.0);
            });
            canvas.group("part cells", |c| {
                let shown = (*length).min(28) as usize;
                for index in 0..28 {
                    let x = MARGIN + (index % 14) as f32 * 64.5;
                    let y = 1020.0 + (index / 14) as f32 * 64.0;
                    let (fill, opacity) = if index < shown {
                        (p.accent, "1")
                    } else {
                        (p.text, "0.1")
                    };
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
            let rows = crate::draw::wrap(headline, heavy(110.0, WHITE), CONTENT);
            let bottom = 290.0 + (rows.len() as f32 - 1.0) * 112.0 + 80.0;
            panel(slide, canvas, bottom, false);
            canvas.group("part", |c| {
                c.wrap(MARGIN, 290.0, headline, heavy(110.0, WHITE), CONTENT, 112.0);
            });
            let mut y = 0.0;
            canvas.group("part", |c| {
                y = c.wrap(MARGIN, bottom + 80.0, line, note.size(30.0), CONTENT, 42.0);
            });
            canvas.group("part hbars", |c| {
                let top = y + 70.0;
                for (index, (name, share)) in bars.iter().enumerate() {
                    let row = top + index as f32 * 96.0;
                    c.fit(MARGIN, row, name, heavy(40.0, p.text), 300.0, 28.0);
                    let width = ((*share as f32 / 100.0) * 470.0).max(6.0);
                    let fill = if index == 0 { p.accent } else { p.text };
                    c.raw(&rect(400.0, row - 36.0, width, 44.0, 0.0, fill));
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
            panel(slide, canvas, 600.0, true);
            canvas.group("part", |c| {
                c.text(MARGIN, LEAD, "You peaked at", lead);
                c.fit(76.0, 520.0, hour, heavy(300.0, WHITE), CONTENT, 120.0);
            });
            canvas.group("part", |c| {
                c.wrap(
                    MARGIN,
                    730.0,
                    line,
                    heavy(48.0, p.text).spacing(-1.0),
                    CONTENT,
                    58.0,
                );
            });
            canvas.group("part bars", |c| {
                hour_bars(c, by_hour, *peak, 1150.0, 270.0, p.accent, p.text);
                axis_labels(c, axis, 4.0, 1196.0, Style::new(MONO_BOLD, 28.0, p.text));
            });
        }
        Content::Shell {
            commands,
            favorite,
            extra,
            rows,
        } => {
            panel(slide, canvas, 560.0, true);
            canvas.group("part", |c| {
                c.text(MARGIN, LEAD, "You ran", lead);
                c.fit(
                    76.0,
                    480.0,
                    commands,
                    heavy(270.0, WHITE).class("value"),
                    CONTENT,
                    120.0,
                );
            });
            let mut y = 0.0;
            canvas.group("part", |c| {
                c.text(MARGIN, 690.0, "commands.", heavy(120.0, p.accent));
                y = 780.0;
                if let Some(favorite) = favorite {
                    y = c.wrap(
                        MARGIN,
                        y,
                        favorite,
                        heavy(40.0, p.text).spacing(-1.0),
                        CONTENT,
                        50.0,
                    );
                }
                if let Some(extra) = extra {
                    y = c.wrap(MARGIN, y, extra, note, CONTENT, 40.0);
                }
            });
            canvas.group("part", |c| table(c, y + 40.0, rows, 56.0, slide));
        }
        Content::Typo {
            typo,
            note: meant,
            footnote,
        } => {
            header(slide, canvas, p.text);
            canvas.group("part", |c| {
                c.text(MARGIN, LEAD, "The typo of the year", lead.fill(p.text));
            });
            canvas.group("part", |c| {
                c.raw(&circle(540.0, 640.0, 330.0, p.accent));
                c.fit(
                    540.0,
                    720.0,
                    typo,
                    heavy(300.0, WHITE).anchor(Anchor::Middle),
                    520.0,
                    90.0,
                );
            });
            canvas.group("part", |c| {
                let y = c.wrap(MARGIN, 1050.0, meant, heavy(40.0, p.text), CONTENT, 50.0);
                c.wrap(MARGIN, y + 10.0, footnote, note, CONTENT, 40.0);
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
            panel(slide, canvas, 520.0, true);
            canvas.group("part", |c| {
                c.text(MARGIN, LEAD, "You had", lead);
                c.fit(
                    76.0,
                    440.0,
                    sessions,
                    heavy(230.0, WHITE).class("value"),
                    CONTENT,
                    110.0,
                );
            });
            let mut y = 640.0;
            canvas.group("part", |c| {
                y = c.wrap(
                    MARGIN,
                    y,
                    "sessions with your AI.",
                    heavy(76.0, p.accent),
                    CONTENT,
                    82.0,
                );
                y = c.wrap(
                    MARGIN,
                    y + 4.0,
                    line,
                    heavy(40.0, p.text).spacing(-1.0),
                    CONTENT,
                    50.0,
                );
                y = c.wrap(MARGIN, y + 6.0, split, note, CONTENT, 40.0);
                if let Some(model) = model {
                    y = c.wrap(MARGIN, y, model, note, CONTENT, 40.0);
                }
            });
            let half = (CONTENT - 44.0) / 2.0;
            canvas.group("part", |c| {
                column(c, MARGIN, y + 34.0, half, "TOOLS", tools, slide);
                column(
                    c,
                    MARGIN + half + 44.0,
                    y + 34.0,
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
            panel(slide, canvas, 590.0, true);
            canvas.group("part", |c| {
                c.text(MARGIN, LEAD, "Your AI went through", lead);
                c.fit(
                    76.0,
                    510.0,
                    total,
                    heavy(290.0, WHITE).class("value"),
                    CONTENT,
                    120.0,
                );
            });
            let mut y = 0.0;
            canvas.group("part", |c| {
                c.text(MARGIN, 720.0, "tokens.", heavy(120.0, p.accent));
                y = [cache.as_str(), novels.as_str()]
                    .iter()
                    .fold(810.0, |y, line| {
                        c.wrap(MARGIN, y, line, note, CONTENT, 40.0)
                    });
            });
            canvas.group("part hbars", |c| {
                let largest = bars
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, (_, _, share))| *share)
                    .map_or(0, |(index, _)| index);
                let top = (y + 70.0).max(960.0);
                for (index, (label, count, share)) in bars.iter().enumerate() {
                    let row = top + index as f32 * 64.0;
                    if row > FOOTER_RULE - 30.0 {
                        break;
                    }
                    c.text(MARGIN, row, label, note);
                    let width = (*share as f32 / 100.0 * 400.0).max(6.0);
                    let fill = if index == largest { p.accent } else { p.text };
                    c.raw(&rect(380.0, row - 30.0, width, 38.0, 0.0, fill));
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
            header(slide, canvas, p.text);
            canvas.group("part", |c| {
                c.text(MARGIN, LEAD, label, lead.fill(p.text));
            });
            canvas.group("part", |c| {
                c.raw(&circle(540.0, 620.0, 300.0, p.accent));
                words(c, 540.0, 620.0, name, heavy(96.0, WHITE), 470.0);
            });
            canvas.group("part", |c| {
                let y = c.wrap(
                    MARGIN,
                    1010.0,
                    reason,
                    heavy(40.0, p.text).spacing(-1.0),
                    CONTENT,
                    52.0,
                );
                if let Some(also) = also {
                    c.wrap(MARGIN, y + 16.0, also, note, CONTENT, 40.0);
                }
            });
        }
        Content::Repositories { line, rows } => {
            let headline = "Where your commits lived";
            let wrapped = crate::draw::wrap(headline, heavy(100.0, WHITE), CONTENT);
            let bottom = 290.0 + (wrapped.len() as f32 - 1.0) * 104.0 + 80.0;
            panel(slide, canvas, bottom, false);
            canvas.group("part", |c| {
                c.wrap(MARGIN, 290.0, headline, heavy(100.0, WHITE), CONTENT, 104.0);
            });
            let mut y = 0.0;
            canvas.group("part", |c| {
                y = c.wrap(MARGIN, bottom + 80.0, line, note.size(30.0), CONTENT, 42.0);
            });
            canvas.group("part", |c| big_table(c, y + 90.0, rows, slide));
        }
        Content::Summary { year, award, rows } => {
            panel(slide, canvas, 560.0, false);
            canvas.group("part", |c| {
                c.fit(
                    76.0,
                    330.0,
                    &format!("{year},"),
                    heavy(160.0, WHITE),
                    560.0,
                    70.0,
                );
                c.fit(76.0, 480.0, "wrapped.", heavy(160.0, WHITE), 560.0, 70.0);
            });
            canvas.group("part", |c| {
                c.raw(&circle(850.0, 560.0, 150.0, p.text));
                c.text(
                    850.0,
                    500.0,
                    "AWARDED",
                    Style::new(MONO_BOLD, 28.0, WHITE)
                        .spacing(4.0)
                        .anchor(Anchor::Middle),
                );
                words(c, 850.0, 590.0, award, heavy(48.0, WHITE), 230.0);
            });
            let named: Vec<(String, String)> = rows
                .iter()
                .map(|(label, value)| (label.to_string(), value.clone()))
                .collect();
            canvas.group("part", |c| big_table(c, 800.0, &named, slide));
        }
    }
}

// the red field runs to the top edge, so the header sits inside it
fn panel(slide: &Scene, canvas: &mut Canvas, bottom: f32, mark: bool) {
    let p = slide.palette;
    canvas.raw(&rect(0.0, 0.0, WIDTH, bottom, 0.0, p.accent));
    if mark {
        canvas.raw(&circle(WIDTH - 96.0, bottom, 64.0, p.text));
    }
    header(slide, canvas, WHITE);
}

fn header(slide: &Scene, canvas: &mut Canvas, color: &'static str) {
    canvas.text(
        MARGIN,
        HEADER,
        &format!("devwrapped · {}", slide.year),
        Style::new(MONO_BOLD, 28.0, color).spacing(1.0),
    );
    canvas.text(
        RIGHT,
        HEADER,
        &format!("{:02} / {:02}", slide.number, slide.total),
        Style::new(MONO_BOLD, 28.0, color)
            .spacing(1.0)
            .anchor(Anchor::End),
    );
    canvas.raw(&format!(
        "<line x1=\"{MARGIN}\" y1=\"124\" x2=\"{RIGHT}\" y2=\"124\" stroke=\"{color}\" stroke-width=\"3\"/>"
    ));
}

fn footer(slide: &Scene, canvas: &mut Canvas) {
    let p = slide.palette;
    rule(canvas, FOOTER_RULE, p.text, 3.0);
    let style = Style::new(MONO_BOLD, 28.0, p.text).spacing(1.0);
    canvas.text(MARGIN, HEIGHT - 62.0, "cargo install devwrapped", style);
    canvas.text(
        RIGHT,
        HEIGHT - 62.0,
        &slide.page.section.to_uppercase(),
        style.anchor(Anchor::End),
    );
}

// a black block with the value reversed out, like a label on a poster
fn tag(canvas: &mut Canvas, x: f32, baseline: f32, text: &str, slide: &Scene) {
    let p = slide.palette;
    let style = Style::new(slide.theme.display(700), 36.0, WHITE);
    let room = RIGHT - x - 40.0;
    if room < 120.0 {
        return;
    }
    let (shown, size) = crate::draw::fit(text, style, room, 28.0);
    let width = crate::measure::width(&shown, style.font.family, 700, size, 0.0);
    canvas.raw(&rect(x, baseline - 76.0, width + 40.0, 72.0, 0.0, p.text));
    canvas.text(x + 20.0, baseline - 26.0, &shown, style.size(size));
}

fn table(canvas: &mut Canvas, top: f32, rows: &[(String, String)], step: f32, slide: &Scene) {
    let p = slide.palette;
    for (index, (label, value)) in rows.iter().enumerate() {
        let y = top + index as f32 * step;
        if index == 0 {
            rule(canvas, y - 38.0, p.text, 3.0);
        }
        let style = Style::new(MONO_BOLD, 30.0, if index == 0 { p.accent } else { p.text });
        let (value, size) = crate::draw::fit(value, style, CONTENT * 0.4, 28.0);
        let width = canvas.text(RIGHT, y, &value, style.size(size).anchor(Anchor::End));
        canvas.fit(
            MARGIN,
            y,
            label,
            Style::new(MONO, 30.0, p.text),
            CONTENT - width - 30.0,
            28.0,
        );
        rule(canvas, y + 18.0, p.text, 1.0);
    }
}

fn big_table(canvas: &mut Canvas, top: f32, rows: &[(String, String)], slide: &Scene) {
    let p = slide.palette;
    for (index, (label, value)) in rows.iter().enumerate() {
        let y = top + index as f32 * 76.0;
        if index == 0 {
            rule(canvas, y - 52.0, p.text, 3.0);
        }
        let style = Style::new(slide.theme.display(700), 48.0, p.text)
            .spacing(-1.5)
            .anchor(Anchor::End);
        let (value, size) = crate::draw::fit(value, style, CONTENT * 0.5, 30.0);
        let width = canvas.text(RIGHT, y, &value, style.size(size));
        canvas.fit(
            MARGIN,
            y,
            label,
            Style::new(MONO, 30.0, p.text),
            CONTENT - width - 30.0,
            28.0,
        );
        rule(canvas, y + 22.0, p.text, 1.0);
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
        Style::new(MONO_BOLD, 28.0, p.accent).spacing(3.0),
    );
    canvas.raw(&format!(
        "<line x1=\"{x:.1}\" y1=\"{0:.1}\" x2=\"{1:.1}\" y2=\"{0:.1}\" stroke=\"{2}\" stroke-width=\"3\"/>",
        top + 16.0,
        x + width,
        p.text
    ));
    for (index, (name, count)) in rows.iter().enumerate() {
        let y = top + 58.0 + index as f32 * 52.0;
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
    }
}

// centers the words of a name on separate lines inside a circle
fn words(canvas: &mut Canvas, cx: f32, cy: f32, name: &str, style: Style, room: f32) {
    let words: Vec<&str> = name.split_whitespace().collect();
    let line = style.size * 0.95;
    let first = cy - (words.len() as f32 - 1.0) * line / 2.0 + style.size * 0.35;
    for (index, word) in words.iter().enumerate() {
        canvas.fit(
            cx,
            first + index as f32 * line,
            word,
            style.anchor(Anchor::Middle),
            room,
            (style.size * 0.5).max(28.0),
        );
    }
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

fn rule(canvas: &mut Canvas, y: f32, color: &str, width: f32) {
    canvas.raw(&format!(
        "<line x1=\"{MARGIN}\" y1=\"{y:.1}\" x2=\"{RIGHT}\" y2=\"{y:.1}\" stroke=\"{color}\" stroke-width=\"{width}\"/>"
    ));
}

fn circle(cx: f32, cy: f32, radius: f32, fill: &str) -> String {
    format!("<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{radius:.1}\" fill=\"{fill}\"/>")
}

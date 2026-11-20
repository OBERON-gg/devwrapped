#![forbid(unsafe_code)]

use std::fs::{self, File};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::{Parser, ValueEnum};
use devwrapped_card::{Clock, Theme};
use devwrapped_core::agents::{self, Agent};
use devwrapped_core::git::CommitScan;
use devwrapped_core::shell::files::find_history_files;
use devwrapped_core::shell::{HistoryScan, Installed};
use devwrapped_core::{Range, Sources, Tally, combine, git};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use term::{Report, Run, Status};

mod clock;
mod term;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// use built-in synthetic data and read no source
    #[arg(long, conflicts_with_all = ["email", "since", "until", "root", "skip"])]
    sample: bool,

    /// count commits by this author email as yours, can be repeated
    #[arg(long, value_name = "EMAIL")]
    email: Vec<String>,

    /// first day to include, the default is January 1 of this year
    #[arg(long, value_name = "YYYY-MM-DD")]
    since: Option<Date>,

    /// last day to include, the default is today
    #[arg(long, value_name = "YYYY-MM-DD")]
    until: Option<Date>,

    /// folder to scan for git repositories in place of the home folder, can be repeated
    #[arg(long, value_name = "PATH")]
    root: Vec<PathBuf>,

    /// source to leave out and never open, can be repeated
    #[arg(long, value_enum, value_name = "SOURCE")]
    skip: Vec<Source>,

    /// outputs to make, separated by commas
    #[arg(long, value_enum, value_delimiter = ',', default_value = "term")]
    format: Vec<Format>,

    /// visual style of the card
    #[arg(long, value_enum, default_value = "night")]
    theme: CardTheme,

    /// 12-hour or 24-hour times, the default follows the system setting
    #[arg(long, value_enum, value_name = "12|24")]
    clock: Option<ClockArg>,

    /// show the names of your most active repositories, which are hidden by default
    #[arg(long)]
    show_repo_names: bool,

    /// folder for the card files, created when missing
    #[arg(long, value_name = "PATH", default_value = ".")]
    out_dir: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Term,
    Svg,
    Png,
    Html,
    All,
}

#[derive(Clone, Copy, ValueEnum)]
enum CardTheme {
    Night,
    Poster,
    Paper,
}

impl From<CardTheme> for Theme {
    fn from(theme: CardTheme) -> Self {
        match theme {
            CardTheme::Night => Theme::Night,
            CardTheme::Poster => Theme::Poster,
            CardTheme::Paper => Theme::Paper,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ClockArg {
    #[value(name = "12")]
    Twelve,
    #[value(name = "24")]
    TwentyFour,
}

impl From<ClockArg> for Clock {
    fn from(clock: ClockArg) -> Self {
        match clock {
            ClockArg::Twelve => Clock::H12,
            ClockArg::TwentyFour => Clock::H24,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Source {
    Git,
    Shell,
    Claude,
    Codex,
}

impl Source {
    fn name(self) -> &'static str {
        match self {
            Source::Git => "git",
            Source::Shell => "shell",
            Source::Claude => "claude",
            Source::Codex => "codex",
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut report = if args.sample {
        Report {
            stats: sample_for_this_year(),
            run: None,
        }
    } else {
        read_sources(&args)?
    };
    if !args.show_repo_names
        && let Some(git) = &mut report.stats.git
    {
        git.top_repositories.clear();
    }
    let clock = args
        .clock
        .map_or_else(|| clock::detect(|name| std::env::var_os(name)), Clock::from);
    let wants = |format| args.format.contains(&format) || args.format.contains(&Format::All);
    if wants(Format::Term) {
        let color = term::use_color(
            std::env::var_os("NO_COLOR").as_deref(),
            std::io::stdout().is_terminal(),
        );
        print!("{}", term::render(&report, color, clock));
    }
    let wanted: Vec<Format> = [Format::Svg, Format::Png, Format::Html]
        .into_iter()
        .filter(|&format| wants(format))
        .collect();
    if !wanted.is_empty() {
        write_cards(
            &report.stats,
            &wanted,
            args.theme.into(),
            clock,
            &args.out_dir,
        )?;
    }
    Ok(())
}

fn write_cards(
    stats: &devwrapped_core::Stats,
    formats: &[Format],
    theme: Theme,
    clock: Clock,
    out_dir: &Path,
) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("could not create {}", out_dir.display()))?;
    let year = devwrapped_card::year_label(stats);
    let slides = devwrapped_card::slides(stats, theme, clock);
    let write = |name: String, bytes: Vec<u8>| -> anyhow::Result<PathBuf> {
        let path = out_dir.join(name);
        fs::write(&path, bytes).with_context(|| format!("could not write {}", path.display()))?;
        Ok(path)
    };
    for &format in formats {
        if format == Format::Html {
            let path = write(
                format!("devwrapped-{year}.html"),
                devwrapped_card::html(stats, theme, clock).into_bytes(),
            )?;
            println!("wrote {}", path.display());
            continue;
        }
        let extension = if format == Format::Png { "png" } else { "svg" };
        let mut names = Vec::new();
        for (index, slide) in slides.iter().enumerate() {
            let bytes = if format == Format::Png {
                devwrapped_card::png(&slide.svg)?
            } else {
                slide.svg.clone().into_bytes()
            };
            let name = format!(
                "devwrapped-{year}-{:02}-{}.{extension}",
                index + 1,
                slide.name
            );
            write(name.clone(), bytes)?;
            names.push(name);
        }
        if let (Some(first), Some(last)) = (names.first(), names.last()) {
            println!(
                "wrote {} {extension} slides to {}, {first} to {last}",
                names.len(),
                out_dir.display()
            );
        }
    }
    Ok(())
}

fn sample_for_this_year() -> devwrapped_core::Stats {
    let today = Timestamp::now().to_zoned(TimeZone::system()).date();
    devwrapped_core::Stats {
        since: today.first_of_year(),
        until: today.last_of_year(),
        ..devwrapped_core::sample()
    }
}

fn read_sources(args: &Args) -> anyhow::Result<Report> {
    let time_zone = TimeZone::system();
    let now = Timestamp::now();
    let today = now.to_zoned(time_zone.clone()).date();
    let since = args.since.unwrap_or_else(|| today.first_of_year());
    let range = Range::from_days(since, args.until, now, &time_zone)
        .context("--since must be on or before --until and not in the future")?;
    if let Some(missing) = args.root.iter().find(|root| !root.is_dir()) {
        bail!("--root folder does not exist: {}", missing.display());
    }
    let home = std::env::home_dir().context("could not find the home folder")?;

    let wanted = |source| !args.skip.contains(&source);
    let git = wanted(Source::Git)
        .then(|| collect_git(&home, &args.root, &args.email, range))
        .flatten();
    let top_repositories = match &git {
        Some((scan, repositories)) if args.show_repo_names => {
            top_repositories(repositories, &scan.commits_per_repository)
        }
        _ => Vec::new(),
    };
    let git = git.map(|(scan, _)| scan);
    let (shell, unreadable_history_files) = match wanted(Source::Shell)
        .then(|| collect_shell(&home))
        .flatten()
    {
        Some((scans, unreadable)) => (Some(scans), unreadable),
        None => (None, 0),
    };
    let mut statuses = vec![
        (Source::Git, status(wanted(Source::Git), git.is_some())),
        (
            Source::Shell,
            status(wanted(Source::Shell), shell.is_some()),
        ),
    ];
    let mut agents = Vec::new();
    for (agent, source) in [
        (Agent::ClaudeCode, Source::Claude),
        (Agent::CodexCli, Source::Codex),
    ] {
        let collection = wanted(source)
            .then(|| agents::collect(agent, &home, |name| std::env::var_os(name)))
            .filter(|collection| collection.found);
        statuses.push((source, status(wanted(source), collection.is_some())));
        agents.extend(collection);
    }
    let sources = Sources {
        git,
        shell,
        agents: (!agents.is_empty()).then_some(agents),
    };

    let mut stats = combine(&sources, range, &time_zone);
    if let Some(git) = &mut stats.git {
        git.top_repositories = top_repositories;
    }
    Ok(Report {
        stats,
        run: Some(Run {
            sources: statuses,
            skipped_records: skipped_records(&sources) + unreadable_history_files,
        }),
    })
}

fn status(wanted: bool, found: bool) -> Status {
    match (wanted, found) {
        (false, _) => Status::Skipped,
        (true, true) => Status::Found,
        (true, false) => Status::NotFound,
    }
}

fn skipped_records(sources: &Sources) -> usize {
    let git = sources
        .git
        .as_ref()
        .map_or(0, |scan| scan.skipped_commits + scan.skipped_repositories);
    let shell: usize = sources
        .shell
        .iter()
        .flatten()
        .map(|scan| scan.skipped_lines)
        .sum();
    let agents: usize = sources
        .agents
        .iter()
        .flatten()
        .map(|collection| collection.skipped_files + collection.skipped_lines)
        .sum();
    git + shell + agents
}

fn collect_git(
    home: &Path,
    roots: &[PathBuf],
    emails: &[String],
    range: Range,
) -> Option<(CommitScan, Vec<PathBuf>)> {
    let roots = if roots.is_empty() {
        vec![home.to_path_buf()]
    } else {
        roots.to_vec()
    };
    // overlapping roots find the same repository twice
    let mut repositories: Vec<PathBuf> = roots
        .iter()
        .flat_map(|root| git::find_repositories(root))
        .map(|path| fs::canonicalize(&path).unwrap_or(path))
        .collect();
    repositories.sort();
    repositories.dedup();
    if repositories.is_empty() {
        return None;
    }
    let emails = git::user_emails(&repositories, emails);
    let scan = git::read_commits(&repositories, &emails, range);
    Some((scan, repositories))
}

// only the folder name is shown, never the path above it
fn top_repositories(repositories: &[PathBuf], commits: &[u64]) -> Vec<Tally> {
    let mut tallies: Vec<Tally> = repositories
        .iter()
        .zip(commits)
        .filter(|&(_, &count)| count > 0)
        .map(|(path, &count)| Tally {
            name: path.file_name().map_or_else(
                || "repository".to_string(),
                |name| name.to_string_lossy().into_owned(),
            ),
            count,
        })
        .collect();
    tallies.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    tallies.truncate(5);
    tallies
}

/// also returns the number of history files that could not be opened
fn collect_shell(home: &Path) -> Option<(Vec<HistoryScan>, usize)> {
    let files = find_history_files(home, |name| std::env::var_os(name));
    if files.is_empty() {
        return None;
    }
    let installed = Installed::from_path(std::env::var_os("PATH").as_deref());
    let mut scans = Vec::new();
    let mut unreadable = 0;
    for file in &files {
        match File::open(&file.path) {
            Ok(reader) => scans.push(file.shell.parse(reader, &installed)),
            Err(_) => unreadable += 1,
        }
    }
    Some((scans, unreadable))
}

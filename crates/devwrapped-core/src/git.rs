use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use gix::bstr::BStr;
use gix::hash::ObjectId;
use gix::revision::walk::Sorting;
use gix::traverse::commit::simple::CommitTimeOrder;
use jiff::Timestamp;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use walkdir::{DirEntry, WalkDir};

use crate::stats::{busiest, busiest_hour, local_time, top_tallies};
use crate::{GitStats, Range, Tally, language};

const MAX_DEPTH: usize = 4;
const SKIPPED_FOLDERS: [&str; 2] = ["node_modules", "target"];
const MAX_EXTENSION_LEN: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub time: Timestamp,
    pub changed_files: usize,
    pub extensions: Vec<String>,
}

#[derive(Debug, Default)]
pub struct CommitScan {
    pub commits: Vec<Commit>,
    /// repositories that hold at least 1 of the returned commits
    pub active_repositories: u32,
    /// commits in the range for each repository, in the order the repositories were given
    pub commits_per_repository: Vec<u64>,
    pub skipped_commits: usize,
    pub skipped_repositories: usize,
}

/// returns every folder up to 4 levels below `root` that holds a `.git` entry, sorted
pub fn find_repositories(root: &Path) -> Vec<PathBuf> {
    let mut repositories: Vec<PathBuf> = WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .into_iter()
        .filter_entry(|entry| entry.depth() == 0 || !is_skipped(entry))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir() && entry.path().join(".git").exists())
        .map(DirEntry::into_path)
        .collect();
    repositories.sort();
    repositories
}

/// returns the lowercased author emails from `extra` and from the git config of each repository
pub fn user_emails(repositories: &[PathBuf], extra: &[String]) -> BTreeSet<String> {
    let mut emails: BTreeSet<String> = extra.iter().filter_map(|email| normalize(email)).collect();
    for path in repositories {
        let Ok(repository) = gix::open(path) else {
            continue;
        };
        let config = repository.config_snapshot();
        for key in ["user.email", "author.email"] {
            if let Some(email) = config.string(key) {
                emails.extend(normalize(&String::from_utf8_lossy(&email)));
            }
        }
    }
    emails
}

/// returns the non-merge commits in `range` whose author email is in `emails`, once per commit id
pub fn read_commits(
    repositories: &[PathBuf],
    emails: &BTreeSet<String>,
    range: Range,
) -> CommitScan {
    let mut scan = CommitScan::default();
    let mut seen = HashSet::new();
    for path in repositories {
        let before = scan.commits.len();
        if read_repository(path, emails, range, &mut seen, &mut scan).is_err() {
            scan.skipped_repositories += 1;
        }
        let found = scan.commits.len() - before;
        if found > 0 {
            scan.active_repositories += 1;
        }
        scan.commits_per_repository.push(found as u64);
    }
    scan
}

fn read_repository(
    path: &Path,
    emails: &BTreeSet<String>,
    range: Range,
    seen: &mut HashSet<ObjectId>,
    scan: &mut CommitScan,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let repository = gix::open(path)?;
    let mut tips: Vec<ObjectId> = repository
        .head_id()
        .map(|id| id.detach())
        .into_iter()
        .collect();
    let references = repository.references()?;
    for reference in references
        .local_branches()?
        .chain(references.remote_branches()?)
    {
        if let Ok(id) = reference.and_then(|mut reference| reference.peel_to_id()) {
            tips.push(id.detach());
        }
    }
    if tips.is_empty() {
        return Ok(());
    }

    let shallow = repository.shallow_commits()?;
    // the cutoff uses committer time, which is never earlier than author time in practice
    let walk = repository
        .rev_walk(tips)
        .sorting(Sorting::ByCommitTimeCutoff {
            order: CommitTimeOrder::NewestFirst,
            seconds: range.start.as_second(),
        })
        .all()?;
    for info in walk {
        let Ok(info) = info else {
            scan.skipped_commits += 1;
            break;
        };
        if info.parent_ids.len() > 1 || !seen.insert(info.id) {
            continue;
        }
        let boundary = shallow
            .as_ref()
            .is_some_and(|commits| commits.binary_search(&info.id).is_ok());
        match read_commit(&repository, info.id, boundary, emails, range) {
            Ok(Some(commit)) => scan.commits.push(commit),
            Ok(None) => {}
            Err(_) => scan.skipped_commits += 1,
        }
    }
    Ok(())
}

fn read_commit(
    repository: &gix::Repository,
    id: ObjectId,
    shallow_boundary: bool,
    emails: &BTreeSet<String>,
    range: Range,
) -> Result<Option<Commit>, Box<dyn Error + Send + Sync>> {
    let commit = repository.find_commit(id)?;
    let author = commit.author()?;
    if !emails.contains(&String::from_utf8_lossy(author.email).to_lowercase()) {
        return Ok(None);
    }
    let time = Timestamp::from_second(author.seconds())?;
    if !range.contains(time) {
        return Ok(None);
    }

    // the parent of a shallow boundary was never fetched, so its changes are unknown
    if shallow_boundary {
        return Ok(Some(Commit {
            time,
            changed_files: 0,
            extensions: Vec::new(),
        }));
    }

    let tree = commit.tree()?;
    let parent_tree = match commit.parent_ids().next() {
        Some(parent) => parent.object()?.into_commit().tree()?,
        None => repository.empty_tree(),
    };
    let mut changed_files = 0;
    let mut extensions = Vec::new();
    parent_tree
        .changes()?
        .options(|options| {
            options.track_path().track_rewrites(None);
        })
        .for_each_to_obtain_tree(&tree, |change| {
            if !change.entry_mode().is_tree() {
                changed_files += 1;
                extensions.extend(extension(change.location()));
            }
            Ok(ControlFlow::Continue(()))
        })?;
    Ok(Some(Commit {
        time,
        changed_files,
        extensions,
    }))
}

/// groups the commits by local day, hour, and weekday in `time_zone`
pub fn summarize(commits: &[Commit], repositories: u32, time_zone: &TimeZone) -> GitStats {
    let mut days = BTreeSet::new();
    let mut hours = [0u64; 24];
    let mut weekdays = [0u64; 7];
    let mut christmas_commits = 0;
    let mut new_year_commits = 0;
    for commit in commits {
        let local = local_time(commit.time, time_zone);
        days.insert(local.date);
        hours[local.hour] += 1;
        weekdays[local.weekday] += 1;
        match (local.date.month(), local.date.day()) {
            (12, 25) => christmas_commits += 1,
            (1, 1) => new_year_commits += 1,
            _ => {}
        }
    }

    GitStats {
        commits: commits.len() as u64,
        active_days: days.len() as u32,
        longest_streak: longest_streak(&days),
        busiest_hour: busiest_hour(&hours),
        busiest_weekday: busiest(&weekdays)
            .and_then(|day| i8::try_from(day).ok())
            .and_then(|day| Weekday::from_monday_zero_offset(day).ok()),
        top_languages: top_languages(commits),
        changed_files: commits
            .iter()
            .map(|commit| commit.changed_files as u64)
            .sum(),
        christmas_commits,
        new_year_commits,
        repositories,
        top_repositories: Vec::new(),
    }
}

fn top_languages(commits: &[Commit]) -> Vec<Tally> {
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for extension in commits.iter().flat_map(|commit| &commit.extensions) {
        if let Some(language) = language(extension) {
            *counts.entry(language).or_default() += 1;
        }
    }
    top_tallies(counts, 1)
}

fn longest_streak(days: &BTreeSet<Date>) -> u32 {
    let mut longest = 0;
    let mut current = 0;
    let mut previous: Option<Date> = None;
    for &day in days {
        let follows = previous.and_then(|previous| previous.tomorrow().ok()) == Some(day);
        current = if follows { current + 1 } else { 1 };
        longest = longest.max(current);
        previous = Some(day);
    }
    longest
}

// odd extensions can hold private text, so keep only short alphanumeric ones
fn extension(location: &BStr) -> Option<String> {
    let name = location.rsplit(|&byte| byte == b'/').next()?;
    let dot = name.iter().rposition(|&byte| byte == b'.')?;
    let extension = &name[dot + 1..];
    let valid = dot > 0
        && (1..=MAX_EXTENSION_LEN).contains(&extension.len())
        && extension.iter().all(u8::is_ascii_alphanumeric);
    valid.then(|| String::from_utf8_lossy(extension).to_lowercase())
}

fn normalize(email: &str) -> Option<String> {
    let email = email.trim();
    (!email.is_empty()).then(|| email.to_lowercase())
}

fn is_skipped(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.') || SKIPPED_FOLDERS.contains(&name.as_ref()) || is_hidden(entry)
}

#[cfg(windows)]
fn is_hidden(entry: &DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    entry
        .metadata()
        .is_ok_and(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
}

#[cfg(not(windows))]
fn is_hidden(_entry: &DirEntry) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_repo::{ME, TestRepo};

    fn make_repo(root: &Path, relative: &str) {
        fs::create_dir_all(root.join(relative).join(".git")).unwrap();
    }

    fn found(root: &Path) -> Vec<PathBuf> {
        find_repositories(root)
            .into_iter()
            .map(|path| path.strip_prefix(root).unwrap().to_path_buf())
            .collect()
    }

    #[test]
    fn finds_repositories_up_to_depth_4() {
        let dir = tempfile::tempdir().unwrap();
        make_repo(dir.path(), "a");
        make_repo(dir.path(), "x/y/z/w");
        make_repo(dir.path(), "x/y/z/w2/too-deep");

        assert_eq!(
            found(dir.path()),
            [
                PathBuf::from("a"),
                Path::new("x").join("y").join("z").join("w")
            ]
        );
    }

    #[test]
    fn skips_build_and_hidden_folders() {
        let dir = tempfile::tempdir().unwrap();
        make_repo(dir.path(), "project/node_modules/dep");
        make_repo(dir.path(), "project/target/build");
        make_repo(dir.path(), ".cache/repo");

        assert!(found(dir.path()).is_empty());
    }

    #[test]
    fn finds_nested_repositories_and_the_root() {
        let dir = tempfile::tempdir().unwrap();
        make_repo(dir.path(), "");
        make_repo(dir.path(), "outer/inner");
        make_repo(dir.path(), "outer");

        assert_eq!(
            found(dir.path()),
            [
                PathBuf::new(),
                PathBuf::from("outer"),
                Path::new("outer").join("inner")
            ]
        );
    }

    #[test]
    fn finds_linked_worktrees() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(worktree.join(".git"), "gitdir: ../main/.git/worktrees/w\n").unwrap();

        assert_eq!(found(dir.path()), [PathBuf::from("worktree")]);
    }

    #[test]
    fn reads_user_and_author_emails() {
        let first = TestRepo::new();
        let second = TestRepo::new();
        first.append_config("[user]\n\temail = Dev@Example.com\n");
        second.append_config("[author]\n\temail = author@example.com\n");

        let emails = user_emails(&[first.path_buf(), second.path_buf()], &[]);

        assert!(emails.contains("dev@example.com"));
        assert!(emails.contains("author@example.com"));
    }

    #[test]
    fn adds_extra_emails_and_skips_broken_repositories() {
        let dir = tempfile::tempdir().unwrap();
        let extra = [" Old@Example.com ".to_string(), "  ".to_string()];

        let emails = user_emails(&[dir.path().join("missing")], &extra);

        assert!(emails.contains("old@example.com"));
        assert!(!emails.contains(""));
    }

    fn range_2026() -> Range {
        Range {
            start: "2026-01-01T00:00:00Z".parse().unwrap(),
            end: "2027-01-01T00:00:00Z".parse().unwrap(),
        }
    }

    fn me() -> BTreeSet<String> {
        BTreeSet::from([ME.to_string()])
    }

    #[test]
    fn reads_own_commits_in_range() {
        let repo = TestRepo::new();
        repo.commit(ME, "2025-12-31T23:00:00Z", &["a.rs"]);
        repo.commit("Dev@Example.com", "2026-03-02T10:00:00Z", &["a.rs", "b.md"]);
        repo.commit("other@example.com", "2026-03-03T10:00:00Z", &["a.rs"]);
        repo.commit(ME, "2027-01-01T00:00:00Z", &["a.rs"]);

        let scan = read_commits(&[repo.path_buf()], &me(), range_2026());

        assert_eq!(
            scan.commits,
            [Commit {
                time: "2026-03-02T10:00:00Z".parse().unwrap(),
                changed_files: 2,
                extensions: vec!["rs".to_string(), "md".to_string()],
            }]
        );
        assert_eq!((scan.skipped_commits, scan.skipped_repositories), (0, 0));
    }

    #[test]
    fn skips_merges_and_counts_each_commit_once() {
        let repo = TestRepo::new();
        let base = repo.commit(ME, "2026-03-02T10:00:00Z", &["a.rs"]);
        let side = repo.commit_on(
            "refs/heads/side",
            ME,
            "2026-03-03T10:00:00Z",
            &["a.rs", "b.py"],
            &[base],
        );
        repo.commit_on(
            "HEAD",
            ME,
            "2026-03-04T10:00:00Z",
            &["a.rs", "b.py"],
            &[base, side],
        );

        let scan = read_commits(&[repo.path_buf(), repo.path_buf()], &me(), range_2026());

        assert_eq!(scan.commits.len(), 2);
    }

    #[test]
    fn reads_commits_only_on_remote_branches() {
        let repo = TestRepo::new();
        let base = repo.commit(ME, "2026-03-02T10:00:00Z", &["a.rs"]);
        repo.commit_on(
            "refs/remotes/origin/work",
            ME,
            "2026-03-03T10:00:00Z",
            &["a.rs"],
            &[base],
        );

        let scan = read_commits(&[repo.path_buf()], &me(), range_2026());

        assert_eq!(scan.commits.len(), 2);
    }

    #[test]
    fn reads_commits_on_a_detached_head() {
        let repo = TestRepo::new();
        let base = repo.commit(ME, "2026-03-02T10:00:00Z", &["a.rs"]);
        let detached = repo.commit_on(
            "refs/heads/temp",
            ME,
            "2026-03-03T10:00:00Z",
            &["a.rs", "b.rs"],
            &[base],
        );
        repo.delete_ref("refs/heads/temp");
        repo.detach_head(detached);

        let scan = read_commits(&[repo.path_buf()], &me(), range_2026());

        assert_eq!(scan.commits.len(), 2);
        assert_eq!(scan.skipped_commits, 0);
    }

    #[test]
    fn reads_shallow_clones_up_to_the_boundary() {
        let repo = TestRepo::new();
        let dropped = repo.commit(ME, "2026-03-02T10:00:00Z", &["a.rs"]);
        let boundary = repo.commit(ME, "2026-03-03T10:00:00Z", &["a.rs", "b.go"]);
        repo.commit(ME, "2026-03-04T10:00:00Z", &["a.rs", "b.go"]);
        repo.make_shallow(boundary, dropped);

        let scan = read_commits(&[repo.path_buf()], &me(), range_2026());

        assert_eq!(scan.commits.len(), 2);
        assert_eq!(scan.commits[1].changed_files, 0);
        assert_eq!((scan.skipped_commits, scan.skipped_repositories), (0, 0));
    }

    #[test]
    fn counts_linked_worktree_commits_once() {
        let repo = TestRepo::new();
        let base = repo.commit(ME, "2026-03-02T10:00:00Z", &["a.rs"]);
        repo.commit_on(
            "refs/heads/feature",
            ME,
            "2026-03-03T10:00:00Z",
            &["a.rs", "b.rs"],
            &[base],
        );
        let worktree = repo.add_worktree("wt", "feature");

        let repositories = find_repositories(repo.path());
        assert_eq!(repositories, [repo.path_buf(), worktree]);
        let scan = read_commits(&repositories, &me(), range_2026());

        assert_eq!(scan.commits.len(), 2);
        assert_eq!((scan.skipped_commits, scan.skipped_repositories), (0, 0));
    }

    #[test]
    fn empty_and_broken_repositories_do_not_fail() {
        let empty = TestRepo::new();
        let missing = empty.path().join("missing");

        let scan = read_commits(&[empty.path_buf(), missing], &me(), range_2026());

        assert!(scan.commits.is_empty());
        assert_eq!(scan.skipped_repositories, 1);
    }

    fn at(time_zone: &TimeZone, day: Date, hour: i8) -> Commit {
        Commit {
            time: day
                .at(hour, 30, 0, 0)
                .to_zoned(time_zone.clone())
                .unwrap()
                .timestamp(),
            changed_files: 1,
            extensions: Vec::new(),
        }
    }

    #[test]
    fn summarizes_days_streaks_and_busiest_slots() {
        let utc = TimeZone::UTC;
        let commits = [
            at(&utc, jiff::civil::date(2026, 3, 2), 9),
            at(&utc, jiff::civil::date(2026, 3, 3), 22),
            at(&utc, jiff::civil::date(2026, 3, 4), 22),
            at(&utc, jiff::civil::date(2026, 3, 10), 22),
            at(&utc, jiff::civil::date(2026, 3, 10), 8),
            at(&utc, jiff::civil::date(2026, 3, 17), 10),
        ];

        let stats = summarize(&commits, 3, &utc);

        assert_eq!(stats.commits, 6);
        assert_eq!(stats.active_days, 5);
        assert_eq!(stats.longest_streak, 3);
        assert_eq!(stats.busiest_hour, Some(22));
        assert_eq!(stats.busiest_weekday, Some(Weekday::Tuesday));
        assert_eq!(stats.repositories, 3);
    }

    #[test]
    fn groups_by_local_time() {
        let utc = TimeZone::UTC;
        let plus_two = TimeZone::fixed(jiff::tz::offset(2));
        let commits = [at(&utc, jiff::civil::date(2026, 3, 1), 23)];

        let stats = summarize(&commits, 1, &plus_two);

        assert_eq!(stats.busiest_hour, Some(1));
        assert_eq!(stats.busiest_weekday, Some(Weekday::Monday));
    }

    #[test]
    fn counts_holiday_commits_by_local_date() {
        let utc = TimeZone::UTC;
        let minus_five = TimeZone::fixed(jiff::tz::offset(-5));
        let commits = [
            at(&utc, jiff::civil::date(2026, 12, 25), 3),
            at(&utc, jiff::civil::date(2026, 12, 25), 12),
            at(&utc, jiff::civil::date(2026, 1, 1), 12),
        ];

        let stats = summarize(&commits, 1, &minus_five);

        assert_eq!((stats.christmas_commits, stats.new_year_commits), (1, 1));
    }

    #[test]
    fn ties_go_to_the_earliest_slot() {
        let utc = TimeZone::UTC;
        let commits = [
            at(&utc, jiff::civil::date(2026, 3, 6), 20),
            at(&utc, jiff::civil::date(2026, 3, 2), 7),
        ];

        let stats = summarize(&commits, 1, &utc);

        assert_eq!(stats.busiest_hour, Some(7));
        assert_eq!(stats.busiest_weekday, Some(Weekday::Monday));
    }

    fn touching(extensions: &[&str]) -> Commit {
        Commit {
            time: Timestamp::constant(1_700_000_000, 0),
            changed_files: extensions.len(),
            extensions: extensions
                .iter()
                .map(|extension| extension.to_string())
                .collect(),
        }
    }

    fn names(tallies: &[Tally]) -> Vec<(&str, u64)> {
        tallies
            .iter()
            .map(|tally| (tally.name.as_str(), tally.count))
            .collect()
    }

    #[test]
    fn ranks_languages_by_changed_files() {
        let commits = [
            touching(&["rs", "rs", "md", "toml"]),
            touching(&["py", "rs", "json"]),
            touching(&["py", "tsx", "ts"]),
        ];

        let stats = summarize(&commits, 1, &TimeZone::UTC);

        assert_eq!(
            names(&stats.top_languages),
            [("Rust", 3), ("Python", 2), ("TypeScript", 2)]
        );
    }

    #[test]
    fn keeps_the_top_5_languages_and_breaks_ties_by_name() {
        let commits = [touching(&[
            "go", "go", "rs", "py", "c", "rb", "java", "lua",
        ])];

        let stats = summarize(&commits, 1, &TimeZone::UTC);

        assert_eq!(
            names(&stats.top_languages),
            [("Go", 2), ("C", 1), ("Java", 1), ("Lua", 1), ("Python", 1)]
        );
    }

    #[test]
    fn summarizes_no_commits() {
        let stats = summarize(&[], 0, &TimeZone::UTC);

        assert_eq!(stats.commits, 0);
        assert_eq!(stats.longest_streak, 0);
        assert_eq!(stats.busiest_hour, None);
        assert_eq!(stats.busiest_weekday, None);
        assert!(stats.top_languages.is_empty());
    }

    #[test]
    fn keeps_only_plain_extensions() {
        let cases = [
            ("src/main.RS", Some("rs")),
            ("archive.tar.gz", Some("gz")),
            (".gitignore", None),
            ("Makefile", None),
            ("notes.my_secret_value", None),
            ("data.averyveryverylongextension", None),
            ("trailing.", None),
        ];
        for (path, expected) in cases {
            assert_eq!(extension(path.into()).as_deref(), expected, "{path}");
        }
    }

    #[test]
    fn missing_root_finds_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_repositories(&dir.path().join("missing")).is_empty());
    }
}

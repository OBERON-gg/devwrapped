use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("devwrapped")
        .unwrap()
        .current_dir(dir)
        .arg("--sample")
        .args(args)
        .output()
        .unwrap()
}

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    files
}

fn year() -> i16 {
    jiff::Zoned::now().year()
}

#[test]
fn prints_only_the_summary_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(dir.path(), &[]);
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("awards"));
    assert!(files(dir.path()).is_empty());
}

fn names(dir: &Path) -> Vec<String> {
    files(dir)
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn writes_every_format_into_a_new_folder() {
    let dir = tempfile::tempdir().unwrap();
    let nested = Path::new("cards").join("new");
    let output = run(
        dir.path(),
        &["--format", "all", "--out-dir", nested.to_str().unwrap()],
    );
    assert!(output.status.success());
    let out = dir.path().join(&nested);
    let year = year();
    let names = names(&out);
    assert_eq!(names.len(), 1 + 2 * 11, "{names:?}");
    for name in [
        format!("devwrapped-{year}.html"),
        format!("devwrapped-{year}-01-intro.png"),
        format!("devwrapped-{year}-11-summary.png"),
        format!("devwrapped-{year}-01-intro.svg"),
        format!("devwrapped-{year}-11-summary.svg"),
    ] {
        assert!(names.contains(&name), "{name} in {names:?}");
    }
    let read = |name: String| fs::read(out.join(name)).unwrap();
    assert!(read(format!("devwrapped-{year}.html")).starts_with(b"<!doctype html>"));
    assert!(read(format!("devwrapped-{year}-05-peak-hour.png")).starts_with(b"\x89PNG"));
    assert!(read(format!("devwrapped-{year}-05-peak-hour.svg")).starts_with(b"<svg "));

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("awards"));
    for extension in ["svg", "png"] {
        let line = format!(
            "wrote 11 {extension} slides to {}, devwrapped-{year}-01-intro.{extension} to \
             devwrapped-{year}-11-summary.{extension}\n",
            nested.display()
        );
        assert!(stdout.contains(&line), "{stdout}");
    }
    let html = nested.join(format!("devwrapped-{year}.html"));
    assert!(
        stdout.contains(&format!("wrote {}\n", html.display())),
        "{stdout}"
    );
}

#[test]
fn writes_only_the_chosen_formats() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(dir.path(), &["--format", "svg,html"]);
    assert!(output.status.success());
    assert!(!String::from_utf8(output.stdout).unwrap().contains("awards"));
    let mut extensions: Vec<String> = files(dir.path())
        .iter()
        .map(|path| path.extension().unwrap().to_string_lossy().into_owned())
        .collect();
    extensions.sort();
    extensions.dedup();
    assert_eq!(extensions, ["html", "svg"]);
}

#[test]
fn adds_a_repository_slide_only_when_asked() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(dir.path(), &["--format", "svg", "--show-repo-names"]);
    assert!(output.status.success());
    let names = names(dir.path());
    assert_eq!(names.len(), 12);
    let repositories = format!("devwrapped-{}-11-repositories.svg", year());
    assert!(names.contains(&repositories), "{names:?}");
    let slide = fs::read_to_string(dir.path().join(repositories)).unwrap();
    assert!(slide.contains(">dotfiles</text>"));
}

#[test]
fn uses_the_chosen_theme() {
    let dir = tempfile::tempdir().unwrap();
    let svg = |theme: &str| {
        let output = run(dir.path(), &["--format", "svg", "--theme", theme]);
        assert!(output.status.success());
        let name = format!("devwrapped-{}-01-intro.svg", year());
        fs::read_to_string(dir.path().join(name)).unwrap()
    };
    assert!(svg("night").contains("#0c0a1d"));
    assert!(svg("poster").contains("#e3120b"));
    assert!(svg("paper").contains("#f3ede1"));
}

#[test]
fn overwrites_an_earlier_card() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir
        .path()
        .join(format!("devwrapped-{}-01-intro.svg", year()));
    fs::write(&path, "old").unwrap();
    assert!(run(dir.path(), &["--format", "svg"]).status.success());
    assert!(fs::read_to_string(&path).unwrap().starts_with("<svg "));
}

#[test]
fn rejects_an_unknown_format_or_theme() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!run(dir.path(), &["--format", "svg,gif"]).status.success());
    assert!(!run(dir.path(), &["--theme", "neon"]).status.success());
    assert!(files(dir.path()).is_empty());
}

#[test]
fn explains_an_out_dir_that_is_a_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("taken"), "").unwrap();
    let output = run(dir.path(), &["--format", "svg", "--out-dir", "taken"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("could not create taken"), "{stderr}");
}

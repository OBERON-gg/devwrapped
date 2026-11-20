use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;

const SECRETS: [&str; 9] = [
    "AKIAFAKEFAKEFAKE0000",
    "hunter2",
    "sk-fake-0000",
    "fake-private-path",
    "fake-private-branch",
    "fake-private-server",
    "123456789012",
    "dev@example.com",
    "deadbeef",
];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("privacy")
        .join(name)
}

fn copy(from: &str, to: &Path) {
    fs::create_dir_all(to.parent().unwrap()).unwrap();
    fs::copy(fixture(from), to).unwrap();
}

fn home_with_secrets() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let share = home.path().join(".local").join("share");
    copy("bash_history", &home.path().join(".bash_history"));
    copy("zsh_history", &home.path().join(".zsh_history"));
    copy("fish_history", &share.join("fish").join("fish_history"));
    copy(
        "powershell_history",
        &share
            .join("powershell")
            .join("PSReadLine")
            .join("ConsoleHost_history.txt"),
    );
    home
}

fn repository_with_secrets(root: &Path) {
    let repository = gix::init(root.join("fake-private-path")).unwrap();
    let blob = repository
        .write_blob("const KEY: &str = \"AKIAFAKEFAKEFAKE0000\";")
        .unwrap()
        .detach();
    let tree = repository
        .write_object(gix::objs::Tree {
            entries: vec![gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Blob.into(),
                filename: "fake-private-path.rs".into(),
                oid: blob,
            }],
        })
        .unwrap();
    let signature = gix::actor::SignatureRef {
        name: "hunter2".into(),
        email: "dev@example.com".into(),
        time: "1772445600 +0000",
    };
    let parents: [gix::hash::ObjectId; 0] = [];
    repository
        .commit_as(
            signature,
            signature,
            "refs/heads/fake-private-branch",
            "add hunter2 and sk-fake-0000",
            tree,
            parents,
        )
        .unwrap();
}

// keeps the run away from the real home, history, agent logs, and installed tools
fn run(home: &Path, args: &[&str]) -> Output {
    let empty_path = home.join("empty-path");
    fs::create_dir_all(&empty_path).unwrap();
    let mut command = Command::cargo_bin("devwrapped").unwrap();
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PATH", &empty_path)
        .env("CLAUDE_CONFIG_DIR", fixture("claude"))
        .env("CODEX_HOME", fixture("codex"));
    for name in ["HISTFILE", "ZDOTDIR", "XDG_DATA_HOME", "APPDATA"] {
        command.env_remove(name);
    }
    command.current_dir(home).args(args).output().unwrap()
}

fn assert_no_secret(place: &str, bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    for secret in SECRETS {
        assert!(!text.contains(secret), "{place} shows a secret");
    }
}

#[test]
fn no_secret_reaches_any_output() {
    let home = home_with_secrets();
    let root = home.path().join("code");
    repository_with_secrets(&root);
    let out = home.path().join("cards");

    let output = run(
        home.path(),
        &[
            "--since",
            "2026-01-01",
            "--until",
            "2026-12-31",
            "--email",
            "dev@example.com",
            "--root",
            root.to_str().unwrap(),
            "--format",
            "all",
            "--out-dir",
            out.to_str().unwrap(),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    for found in ["git                found", "shell              found"] {
        assert!(stdout.contains(found), "{stdout}");
    }
    assert!(stdout.contains("claude             found"), "{stdout}");
    assert!(stdout.contains("codex              found"), "{stdout}");
    assert!(stdout.contains("  commits            1\n"), "{stdout}");
    assert!(
        stdout.contains("unreadable records were skipped"),
        "{stdout}"
    );
    assert_no_secret("the terminal", &output.stdout);
    assert_no_secret("stderr", &output.stderr);
    let cards: Vec<PathBuf> = fs::read_dir(&out)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    for extension in ["svg", "png", "html"] {
        assert!(
            cards
                .iter()
                .any(|card| card.extension().is_some_and(|found| found == extension)),
            "no {extension}"
        );
    }
    for card in &cards {
        assert_no_secret(&card.display().to_string(), &fs::read(card).unwrap());
    }
}

#[test]
fn errors_never_show_a_raw_record() {
    let home = tempfile::tempdir().unwrap();
    let mut history = b"hunter2 \xff\xfe AKIAFAKEFAKEFAKE0000\n".to_vec();
    history.extend(b"sk-fake-0000 ".repeat(10_000));
    history.extend(b"\n: not-a-time:0;hunter2\n");
    fs::write(home.path().join(".zsh_history"), &history).unwrap();
    fs::write(home.path().join(".bash_history"), &history).unwrap();
    fs::write(home.path().join("taken"), "").unwrap();

    let read = run(home.path(), &["--since", "2026-01-01"]);
    assert!(read.status.success());
    assert_no_secret("the terminal", &read.stdout);
    assert_no_secret("stderr", &read.stderr);

    let failed = run(
        home.path(),
        &[
            "--since",
            "2026-01-01",
            "--format",
            "svg",
            "--out-dir",
            "taken",
        ],
    );
    assert!(!failed.status.success());
    assert!(!failed.stderr.is_empty());
    assert_no_secret("the error", &failed.stderr);
    assert_no_secret("the terminal", &failed.stdout);
}

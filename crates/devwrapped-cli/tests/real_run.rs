use std::path::Path;

use assert_cmd::Command;

const ME: &str = "dev@example.com";

fn commit_as_me(repository: &gix::Repository, time: &str) {
    let blob = repository.write_blob("fn main() {}").unwrap().detach();
    let tree = repository
        .write_object(gix::objs::Tree {
            entries: vec![gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Blob.into(),
                filename: "main.rs".into(),
                oid: blob,
            }],
        })
        .unwrap();
    let signature = gix::actor::SignatureRef {
        name: "dev".into(),
        email: ME.into(),
        time,
    };
    let parents: [gix::hash::ObjectId; 0] = [];
    repository
        .commit_as(signature, signature, "HEAD", "change", tree, parents)
        .unwrap();
}

// keeps the run away from the real home, history, and agent logs of the machine
fn devwrapped(home: &Path) -> Command {
    let mut command = Command::cargo_bin("devwrapped").unwrap();
    command.env("HOME", home).env("USERPROFILE", home);
    for name in [
        "HISTFILE",
        "ZDOTDIR",
        "XDG_DATA_HOME",
        "APPDATA",
        "CLAUDE_CONFIG_DIR",
        "CODEX_HOME",
    ] {
        command.env_remove(name);
    }
    command
}

#[test]
fn scans_the_given_roots_in_place_of_home() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let repository = gix::init(root.path().join("project")).unwrap();
    commit_as_me(&repository, "1772445600 +0000");

    let output = devwrapped(home.path())
        .args([
            "--since",
            "2026-01-01",
            "--until",
            "2026-12-31",
            "--email",
            ME,
        ])
        .arg("--root")
        .arg(root.path())
        .arg("--root")
        .arg(root.path().join("project"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("  commits            1\n"), "{stdout}");
    assert!(stdout.contains("  repositories       1\n"), "{stdout}");
}

#[test]
fn rejects_a_root_that_does_not_exist() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("missing");

    let output = devwrapped(home.path())
        .arg("--root")
        .arg(&missing)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--root folder does not exist"));
}

fn home_with_history_and_agent_logs() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join(".bash_history"), "ls\nls\nls\n").unwrap();
    let project = home.path().join(".claude").join("projects").join("p");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("s1.jsonl"),
        r#"{"type":"user","sessionId":"s1","timestamp":"2026-03-02T09:00:00Z"}
"#,
    )
    .unwrap();
    home
}

#[test]
fn reads_every_source_by_default() {
    let home = home_with_history_and_agent_logs();

    let output = devwrapped(home.path())
        .args(["--since", "2026-01-01", "--until", "2026-12-31"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("shell\n  commands"), "{stdout}");
    assert!(stdout.contains("agents\n  sessions"), "{stdout}");
    assert!(!stdout.contains("skipped"), "{stdout}");
    assert!(stdout.contains("cleanupPeriodDays"), "{stdout}");
    assert!(
        stdout.contains("note: 3 shell commands have no date."),
        "{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn counts_a_history_file_it_cannot_open() {
    use std::os::unix::fs::PermissionsExt;

    let home = home_with_history_and_agent_logs();
    let locked = home.path().join(".zsh_history");
    std::fs::write(&locked, ": 1772445600:0;ls\n").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&locked).is_ok() {
        // root can open the file anyway, so the case cannot happen
        return;
    }

    let output = devwrapped(home.path())
        .args(["--since", "2026-01-01", "--until", "2026-12-31"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("shell\n  commands           3"), "{stdout}");
    assert!(
        stdout.contains("note: 1 unreadable record was skipped."),
        "{stdout}"
    );
}

#[test]
fn leaves_out_skipped_sources() {
    let home = home_with_history_and_agent_logs();

    let output = devwrapped(home.path())
        .args(["--since", "2026-01-01", "--until", "2026-12-31"])
        .args(["--skip", "claude", "--skip", "shell", "--skip", "claude"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(
            "sources\n  git                not found\n  shell              skipped\n  \
             claude             skipped\n  codex              not found\n"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("cleanupPeriodDays"), "{stdout}");
    assert!(!stdout.contains("shell\n  commands"), "{stdout}");
    assert!(!stdout.contains("agents\n"), "{stdout}");
}

#[test]
fn prints_no_color_when_output_is_not_a_terminal() {
    let home = home_with_history_and_agent_logs();

    let output = devwrapped(home.path())
        .env_remove("NO_COLOR")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
}

#[test]
fn rejects_an_unknown_source() {
    let home = tempfile::tempdir().unwrap();

    let output = devwrapped(home.path())
        .args(["--skip", "browser"])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn shows_repository_names_only_when_asked() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    for (name, time) in [("alpha", "1772445600 +0000"), ("beta", "1772449200 +0000")] {
        let repository = gix::init(root.path().join(name)).unwrap();
        commit_as_me(&repository, time);
    }
    let run = |extra: &[&str]| {
        let output = devwrapped(home.path())
            .args(["--since", "2026-01-01", "--until", "2026-12-31"])
            .args(["--email", ME, "--skip", "shell"])
            .args(["--skip", "claude", "--skip", "codex"])
            .arg("--root")
            .arg(root.path())
            .args(extra)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };

    let hidden = run(&[]);
    assert!(hidden.contains("  repositories       2\n"), "{hidden}");
    assert!(
        !hidden.contains("alpha") && !hidden.contains("beta"),
        "{hidden}"
    );

    let shown = run(&["--show-repo-names"]);
    assert!(
        shown.contains("  top repositories   alpha  1\n                     beta   1\n"),
        "{shown}"
    );
    let root_name = root.path().file_name().unwrap().to_string_lossy();
    assert!(!shown.contains(root_name.as_ref()), "{shown}");
}

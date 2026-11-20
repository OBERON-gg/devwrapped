use assert_cmd::Command;

#[test]
fn sample_prints_every_section() {
    let output = Command::cargo_bin("devwrapped")
        .unwrap()
        .arg("--sample")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for section in ["git\n", "shell\n", "agents\n", "awards\n  Agent Whisperer"] {
        assert!(stdout.contains(section), "missing {section:?}");
    }
}

#[test]
fn sample_refuses_source_flags() {
    for flag in [
        ["--since", "2026-01-01"],
        ["--root", "."],
        ["--skip", "git"],
        ["--email", "dev@example.com"],
    ] {
        let output = Command::cargo_bin("devwrapped")
            .unwrap()
            .arg("--sample")
            .args(flag)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{flag:?}");
    }
}

#[test]
fn rejects_a_range_that_ends_before_it_starts() {
    let output = Command::cargo_bin("devwrapped")
        .unwrap()
        .args(["--since", "2026-06-02", "--until", "2026-06-01"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--since must be on or before --until"));
}

#[test]
fn rejects_a_date_in_the_wrong_form() {
    let output = Command::cargo_bin("devwrapped")
        .unwrap()
        .args(["--since", "2026-13-01"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn sample_hides_repository_names_unless_asked() {
    let run = |extra: &[&str]| {
        let output = Command::cargo_bin("devwrapped")
            .unwrap()
            .arg("--sample")
            .args(extra)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    assert!(!run(&[]).contains("dotfiles"));
    assert!(run(&["--show-repo-names"]).contains("dotfiles"));
}

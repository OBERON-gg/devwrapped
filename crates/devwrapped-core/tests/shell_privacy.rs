use std::fs::File;
use std::path::Path;

use devwrapped_core::shell::{HistoryScan, Installed, bash, fish, powershell, typos, zsh};

const SECRETS: [&str; 6] = [
    "fake-private-server",
    "123456789012",
    "AKIAFAKEFAKEFAKE0000",
    "hunter2",
    "sk-fake-0000",
    "fake-private-path",
];

fn fixture(name: &str) -> File {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("privacy")
        .join(name);
    File::open(path).unwrap()
}

fn assert_no_secret(shell: &str, scan: &HistoryScan) {
    assert!(!scan.commands.is_empty(), "{shell} parsed nothing");
    let typos: Vec<String> = scan.commands.iter().filter_map(typos::find_typo).collect();
    let kept = format!("{:?} {typos:?} {}", scan.commands, scan.skipped_lines);
    for secret in SECRETS {
        assert!(!kept.contains(secret), "{shell} kept a secret");
    }
}

#[test]
fn zsh_keeps_no_secret() {
    assert_no_secret(
        "zsh",
        &zsh::parse(fixture("zsh_history"), &Installed::default()),
    );
}

#[test]
fn bash_keeps_no_secret() {
    assert_no_secret(
        "bash",
        &bash::parse(fixture("bash_history"), &Installed::default()),
    );
}

#[test]
fn fish_keeps_no_secret() {
    assert_no_secret(
        "fish",
        &fish::parse(fixture("fish_history"), &Installed::default()),
    );
}

#[test]
fn powershell_keeps_no_secret() {
    assert_no_secret(
        "powershell",
        &powershell::parse(fixture("powershell_history"), &Installed::default()),
    );
}

#[test]
fn oversized_lines_keep_no_secret() {
    let mut input = b"ls\n".to_vec();
    for _ in 0..10_000 {
        input.extend_from_slice(b"AKIAFAKEFAKEFAKE0000 hunter2 ");
    }
    input.push(b'\n');

    assert_no_secret("bash", &bash::parse(&input[..], &Installed::default()));
    assert_no_secret(
        "powershell",
        &powershell::parse(&input[..], &Installed::default()),
    );
}

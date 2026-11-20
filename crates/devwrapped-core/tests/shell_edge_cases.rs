use std::fs::File;
use std::path::Path;

use devwrapped_core::shell::{HistoryScan, Installed, bash, fish, powershell, zsh};

fn fixture(name: &str) -> File {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("shell")
        .join(name);
    File::open(path).unwrap()
}

fn summary(scan: &HistoryScan) -> Vec<(&str, Option<&str>, Option<i64>)> {
    scan.commands
        .iter()
        .map(|command| {
            (
                command.name.as_str(),
                command.subcommand.as_deref(),
                command.time.map(|time| time.as_second()),
            )
        })
        .collect()
}

#[test]
fn zsh_handles_bom_crlf_invalid_utf8_and_corrupt_lines() {
    let scan = zsh::parse(fixture("zsh_edge_cases"), &Installed::default());

    assert_eq!(
        summary(&scan),
        [
            ("git", Some("status"), Some(1_767_225_600)),
            ("ls", None, Some(1_767_225_601)),
            ("other", None, Some(1_767_225_602)),
            ("echo", None, Some(1_767_225_604)),
            ("cargo", Some("test"), Some(1_767_225_605)),
            ("echo", None, Some(1_767_225_606)),
        ]
    );
    assert_eq!(scan.skipped_lines, 2);
}

#[test]
fn bash_handles_bom_crlf_invalid_utf8_and_corrupt_lines() {
    let scan = bash::parse(fixture("bash_edge_cases"), &Installed::default());

    assert_eq!(
        summary(&scan),
        [
            ("ls", None, None),
            ("git", Some("status"), Some(1_767_225_600)),
            ("other", None, None),
            ("echo", None, Some(1_767_225_601)),
            ("cargo", Some("test"), Some(1_767_225_602)),
        ]
    );
    assert_eq!(scan.skipped_lines, 1);
}

#[test]
fn fish_handles_bom_crlf_invalid_utf8_and_corrupt_lines() {
    let scan = fish::parse(fixture("fish_edge_cases"), &Installed::default());

    assert_eq!(
        summary(&scan),
        [
            ("git", Some("status"), Some(1_767_225_600)),
            ("ls", None, Some(1_767_225_601)),
            ("other", None, Some(1_767_225_602)),
            ("docker", Some("ps"), None),
            ("echo", None, Some(1_767_225_603)),
        ]
    );
    assert_eq!(scan.skipped_lines, 2);
}

#[test]
fn powershell_handles_bom_crlf_and_invalid_utf8() {
    let scan = powershell::parse(fixture("powershell_edge_cases"), &Installed::default());

    assert_eq!(
        summary(&scan),
        [
            ("git", Some("status"), None),
            ("ls", None, None),
            ("other", None, None),
            ("echo", None, None),
            ("Write-Host", None, None),
            ("cargo", Some("test"), None),
        ]
    );
    assert_eq!(scan.skipped_lines, 0);
}

#[test]
fn every_parser_reads_an_empty_file() {
    for scan in [
        zsh::parse(fixture("empty"), &Installed::default()),
        bash::parse(fixture("empty"), &Installed::default()),
        fish::parse(fixture("empty"), &Installed::default()),
        powershell::parse(fixture("empty"), &Installed::default()),
    ] {
        assert!(scan.commands.is_empty());
        assert_eq!(scan.skipped_lines, 0);
    }
}

#[test]
fn line_parsers_skip_an_oversized_line() {
    for scan in [
        zsh::parse(fixture("oversized_line"), &Installed::default()),
        bash::parse(fixture("oversized_line"), &Installed::default()),
        powershell::parse(fixture("oversized_line"), &Installed::default()),
    ] {
        assert_eq!(
            summary(&scan),
            [("ls", None, None), ("git", Some("status"), None)]
        );
        assert_eq!(scan.skipped_lines, 1);
    }
}

#[test]
fn fish_skips_an_oversized_line() {
    let mut input = b"- cmd: ls\n  when: 1767225600\n- cmd: ".to_vec();
    input.extend(std::iter::repeat_n(b'x', 70_000));
    input.extend_from_slice(b"\n  when: 1767225601\n- cmd: git status\n");

    let scan = fish::parse(&input[..], &Installed::default());

    assert_eq!(
        summary(&scan),
        [
            ("ls", None, Some(1_767_225_600)),
            ("git", Some("status"), None),
        ]
    );
    assert_eq!(scan.skipped_lines, 1);
}

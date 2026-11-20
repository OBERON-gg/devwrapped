use std::io::{self, Read};

use jiff::Timestamp;

use crate::lines;

pub mod bash;
pub mod files;
pub mod fish;
pub mod names;
pub mod powershell;
pub mod typos;
pub mod zsh;

pub use names::Installed;

const MAX_LINE_LEN: usize = 64 * 1024;

/// one shell command reduced to the parts devwrapped keeps
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    pub name: String,
    pub subcommand: Option<String>,
    pub time: Option<Timestamp>,
}

#[derive(Debug, Default)]
pub struct HistoryScan {
    pub commands: Vec<ParsedCommand>,
    pub skipped_lines: usize,
}

fn for_each_line(reader: impl Read, each: impl FnMut(Option<&[u8]>)) -> io::Result<()> {
    lines::for_each_line(reader, MAX_LINE_LEN, each)
}

#[cfg(test)]
fn fixture(name: &str) -> std::fs::File {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("shell")
        .join(name);
    std::fs::File::open(path).unwrap()
}

#[cfg(test)]
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

const WRAPPERS: [&str; 4] = ["sudo", "time", "nohup", "env"];
const OTHER: &str = "other";
const MAX_NAME_LEN: usize = 32;

/// turns a name that is not known, installed, or a typo into `other`, and keeps only known subcommands
pub fn parse_line(
    line: &str,
    time: Option<Timestamp>,
    installed: &Installed,
) -> Option<ParsedCommand> {
    let (name, next) = command_words(line)?;
    if !is_valid_name(name) || !is_shown(name, installed) {
        return Some(ParsedCommand {
            name: OTHER.to_string(),
            subcommand: None,
            time,
        });
    }
    let subcommand = next
        .filter(|next| is_valid_name(next) && is_known_subcommand(name, next))
        .map(str::to_string);
    Some(ParsedCommand {
        name: name.to_string(),
        subcommand,
        time,
    })
}

fn is_shown(name: &str, installed: &Installed) -> bool {
    names::is_known_command(name)
        || installed.contains(name)
        || names::is_cmdlet(name)
        || typos::is_command_typo(name)
}

fn is_known_subcommand(tool: &str, word: &str) -> bool {
    let word = word.to_ascii_lowercase();
    names::subcommands(tool).is_some_and(|known| {
        known.contains(&word.as_str()) || known.iter().any(|known| typos::is_one_edit(&word, known))
    })
}

// paths, variables, and pasted text fail this check, so they never reach the output
fn is_valid_name(name: &str) -> bool {
    name.len() <= MAX_NAME_LEN
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

/// returns the command name and the word after it, after leading assignments and wrappers
pub fn command_words(line: &str) -> Option<(&str, Option<&str>)> {
    let mut words = line.split_whitespace().peekable();
    if words.peek()?.starts_with('#') {
        return None;
    }

    let mut wrapper = None;
    while let Some(&word) = words.peek() {
        if is_assignment(word) {
            words.next();
        } else if WRAPPERS.contains(&word) {
            words.next();
            wrapper = Some(word);
            while let Some(option) = words.next_if(|word| word.starts_with('-')) {
                if option == "--" {
                    break;
                }
                if takes_value(word, option) {
                    words.next();
                }
            }
        } else {
            break;
        }
    }

    let name = words.next().or(wrapper)?;
    let next = words.next().filter(|word| !word.starts_with('-'));
    Some((name, next))
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
}

// skipping the value keeps a user name or path from being read as the command
fn takes_value(wrapper: &str, option: &str) -> bool {
    let (short, long): (&[u8], &[&str]) = match wrapper {
        "sudo" => (
            b"CDghpRrTtUu",
            &[
                "--chdir",
                "--chroot",
                "--close-from",
                "--command-timeout",
                "--group",
                "--host",
                "--other-user",
                "--prompt",
                "--role",
                "--type",
                "--user",
            ],
        ),
        "env" => (b"CSu", &["--chdir", "--split-string", "--unset"]),
        "time" => (b"fo", &["--format", "--output"]),
        _ => return false,
    };
    if option.starts_with("--") {
        return long.contains(&option);
    }
    // in -iu root the value follows, in -uroot it is attached
    let flags = option.trim_start_matches('-').as_bytes();
    flags
        .iter()
        .position(|flag| short.contains(flag))
        .is_some_and(|index| index + 1 == flags.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_name_and_next_word() {
        let cases = [
            ("git commit -m hunter2", Some(("git", Some("commit")))),
            ("  git\tstatus  ", Some(("git", Some("status")))),
            ("ls -la /home/dev/secret", Some(("ls", None))),
            ("cargo", Some(("cargo", None))),
            ("cd x && cargo build", Some(("cd", Some("x")))),
        ];
        for (line, expected) in cases {
            assert_eq!(command_words(line), expected, "{line}");
        }
    }

    #[test]
    fn removes_assignments_and_wrappers() {
        let cases = [
            (
                "TOKEN=abc AWS_KEY=AKIAFAKEFAKEFAKE0000 cargo build",
                ("cargo", Some("build")),
            ),
            ("A=b=c ls", ("ls", None)),
            (
                "sudo -u root env FOO=1 time nohup docker ps",
                ("docker", Some("ps")),
            ),
            ("sudo -E -- npm install", ("npm", Some("install"))),
            ("time -f %e -o out.txt make all", ("make", Some("all"))),
            ("env -u HOME -i PATH=/bin git log", ("git", Some("log"))),
            ("sudo -iu fakeuser psql", ("psql", None)),
            ("sudo -ufakeuser -i psql", ("psql", None)),
            ("sudo --user fakeuser git status", ("git", Some("status"))),
            ("sudo --user=fakeuser git status", ("git", Some("status"))),
            ("sudo --login --preserve-env=PATH make", ("make", None)),
            ("time --output fake-report.txt make", ("make", None)),
            ("time -vo fake-report.txt make", ("make", None)),
            (
                "env --unset=HOME --chdir /tmp/fake cargo test",
                ("cargo", Some("test")),
            ),
        ];
        for (line, expected) in cases {
            assert_eq!(command_words(line), Some(expected), "{line}");
        }
    }

    #[test]
    fn keeps_a_wrapper_with_no_command() {
        assert_eq!(command_words("sudo -i"), Some(("sudo", None)));
        assert_eq!(command_words("env"), Some(("env", None)));
        assert_eq!(command_words("FOO=1 env"), Some(("env", None)));
    }

    #[test]
    fn finds_no_command() {
        for line in ["", "   ", "# a comment", "FOO=bar", "TOKEN=hunter2 OTHER=x"] {
            assert_eq!(command_words(line), None, "{line}");
        }
    }

    fn parsed(line: &str) -> Option<(String, Option<String>)> {
        parse_line(line, None, &Installed::default())
            .map(|command| (command.name, command.subcommand))
    }

    #[test]
    fn keeps_subcommands_of_known_tools() {
        assert_eq!(
            parsed("git commit -m hunter2"),
            Some(("git".to_string(), Some("commit".to_string())))
        );
        assert_eq!(
            parsed("sudo docker compose up"),
            Some(("docker".to_string(), Some("compose".to_string())))
        );
    }

    #[test]
    fn drops_the_next_word_of_other_commands() {
        for (line, name) in [
            ("cd /home/dev/secret", "cd"),
            ("cat notes.txt", "cat"),
            ("claude fix the hunter2 bug", "claude"),
            ("make deploy-prod", "make"),
        ] {
            assert_eq!(parsed(line), Some((name.to_string(), None)), "{line}");
        }
    }

    #[test]
    fn counts_invalid_names_as_other() {
        for line in [
            "./deploy.sh --prod",
            "/usr/bin/git status",
            "$env:TOKEN = hunter2",
            "& tool.exe",
            "caf\u{e9} menu",
            "1FOO=x ls",
            "abcdefghijklmnopqrstuvwxyz0123456 arg",
        ] {
            assert_eq!(parsed(line), Some(("other".to_string(), None)), "{line}");
        }
    }

    #[test]
    fn accepts_installed_names_up_to_32_characters() {
        let names = ["python3.12", "zola", "abcdefghijklmnopqrstuvwxyz012345"];
        let installed = Installed::of(&names);
        for name in names {
            let command = parse_line(name, None, &installed).unwrap();
            assert_eq!(command.name, name);
        }
        let too_long = "abcdefghijklmnopqrstuvwxyz0123456";
        let installed = Installed::of(&[too_long]);
        assert_eq!(
            parse_line(too_long, None, &installed).unwrap().name,
            "other"
        );
    }

    #[test]
    fn hides_names_that_are_not_known_or_installed() {
        for line in ["hunter2", "AKIAFAKEFAKEFAKE0000", "deploy-acme-fake", "EOF"] {
            assert_eq!(parsed(line), Some(("other".to_string(), None)), "{line}");
        }
        for line in ["g++", "docker-compose", "for", "Get-ChildItem", "gti"] {
            assert_eq!(parsed(line), Some((line.to_string(), None)), "{line}");
        }
    }

    #[test]
    fn keeps_only_known_subcommands_and_their_typos() {
        let cases = [
            ("git ../private-dir", "git", None),
            ("cargo +nightly build", "cargo", None),
            ("yarn deploy-acme-fake", "yarn", None),
            ("dotnet FakeApp.dll", "dotnet", None),
            ("deno fake-script.ts", "deno", None),
            ("yarn build", "yarn", Some("build")),
            ("Git Status", "Git", Some("Status")),
            ("git stauts", "git", Some("stauts")),
        ];
        for (line, name, subcommand) in cases {
            assert_eq!(
                parsed(line),
                Some((name.to_string(), subcommand.map(str::to_string))),
                "{line}"
            );
        }
    }

    #[test]
    fn keeps_the_time() {
        let time = Some(Timestamp::constant(1_700_000_000, 0));
        assert_eq!(
            parse_line("ls", time, &Installed::default()).unwrap().time,
            time
        );
    }

    #[test]
    fn leaves_invalid_assignments_as_the_name() {
        assert_eq!(command_words("1FOO=x ls"), Some(("1FOO=x", Some("ls"))));
    }
}

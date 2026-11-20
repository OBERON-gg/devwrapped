use std::io::Read;

use jiff::Timestamp;

use super::{HistoryScan, Installed, ParsedCommand, for_each_line, parse_line};

pub fn parse(reader: impl Read, installed: &Installed) -> HistoryScan {
    let mut scan = HistoryScan::default();
    // the "when" line comes after the command, so hold the parsed command until then
    let mut pending: Option<ParsedCommand> = None;
    let result = for_each_line(reader, |line| {
        // a skipped line ends the pending entry, so a later "when" cannot attach to it
        let Some(line) = line else {
            scan.skipped_lines += 1;
            scan.commands.extend(pending.take());
            return;
        };
        let text = String::from_utf8_lossy(line);
        if let Some(command) = text.strip_prefix("- cmd: ") {
            scan.commands.extend(pending.take());
            pending = parse_line(&unescape(command), None, installed);
        } else if let Some(seconds) = text.strip_prefix("  when: ") {
            match seconds
                .trim()
                .parse()
                .ok()
                .and_then(|seconds| Timestamp::from_second(seconds).ok())
            {
                Some(time) => {
                    if let Some(command) = pending.as_mut() {
                        command.time = Some(time);
                    }
                }
                None => scan.skipped_lines += 1,
            }
        } else if !text.starts_with(' ') && !text.is_empty() {
            scan.skipped_lines += 1;
            scan.commands.extend(pending.take());
        }
    });
    scan.commands.extend(pending);
    if result.is_err() {
        scan.skipped_lines += 1;
    }
    scan
}

// fish stores a newline as "\n" and a backslash as "\\"
fn unescape(command: &str) -> String {
    let mut out = String::with_capacity(command.len());
    let mut chars = command.chars();
    while let Some(current) = chars.next() {
        if current != '\\' {
            out.push(current);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{fixture, summary};

    #[test]
    fn parses_history() {
        let scan = parse(fixture("fish_history"), &Installed::default());

        assert_eq!(
            summary(&scan),
            [
                ("git", Some("status"), Some(1_767_225_600)),
                ("cargo", Some("build"), Some(1_767_225_660)),
                ("echo", None, Some(1_767_225_720)),
                ("ls", None, Some(1_767_225_780)),
                ("docker", Some("ps"), None),
                ("npm", Some("install"), None),
            ]
        );
        assert_eq!(scan.skipped_lines, 2);
    }

    #[test]
    fn keeps_no_arguments_or_paths() {
        let scan = parse(fixture("fish_history"), &Installed::default());
        let kept = format!("{:?}", scan.commands);

        for secret in ["hunter2", "private", "secret-arg", "second", "push"] {
            assert!(!kept.contains(secret), "{secret}");
        }
    }

    #[test]
    fn unescapes_newlines_and_backslashes() {
        assert_eq!(unescape(r"ls\ngit"), "ls\ngit");
        assert_eq!(unescape(r"echo \\n"), r"echo \n");
        assert_eq!(unescape(r"end\"), r"end\");
    }
}

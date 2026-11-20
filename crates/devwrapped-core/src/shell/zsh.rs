use std::io::Read;

use jiff::Timestamp;

use super::{HistoryScan, Installed, ParsedCommand, for_each_line, parse_line};

pub fn parse(reader: impl Read, installed: &Installed) -> HistoryScan {
    let mut scan = HistoryScan::default();
    let mut continued = false;
    let result = for_each_line(reader, |line| {
        let Some(line) = line else {
            scan.skipped_lines += 1;
            continued = false;
            return;
        };
        // zsh ends each line of a multi-line command except the last with a backslash
        let was_continued = continued;
        continued = line.last() == Some(&b'\\');
        if was_continued {
            return;
        }
        match parse_entry(line, installed) {
            Some(Some(command)) => scan.commands.push(command),
            Some(None) => {}
            None => scan.skipped_lines += 1,
        }
    });
    if result.is_err() {
        scan.skipped_lines += 1;
    }
    scan
}

/// returns `None` for a corrupt line and `Some(None)` for a line with no command
fn parse_entry(line: &[u8], installed: &Installed) -> Option<Option<ParsedCommand>> {
    let text = String::from_utf8_lossy(line);
    let Some(rest) = text.strip_prefix(": ") else {
        return Some(parse_line(&text, None, installed));
    };
    let (meta, command) = rest.split_once(';')?;
    let (seconds, _duration) = meta.split_once(':')?;
    let time = Timestamp::from_second(seconds.parse().ok()?).ok()?;
    Some(parse_line(command, Some(time), installed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{fixture, summary};

    #[test]
    fn parses_extended_history() {
        let scan = parse(fixture("zsh_history"), &Installed::default());

        assert_eq!(
            summary(&scan),
            [
                ("git", Some("status"), Some(1_767_225_600)),
                ("cargo", Some("build"), Some(1_767_225_660)),
                ("npm", Some("install"), Some(1_767_225_720)),
                ("echo", None, Some(1_767_225_780)),
                ("ls", None, Some(1_767_225_840)),
                ("cd", None, None),
                ("docker", Some("ps"), Some(1_767_225_900)),
            ]
        );
        assert_eq!(scan.skipped_lines, 1);
    }

    #[test]
    fn keeps_no_arguments() {
        let scan = parse(fixture("zsh_history"), &Installed::default());
        let kept = format!("{:?}", scan.commands);

        for secret in ["hunter2", "left-pad", "private", "second line"] {
            assert!(!kept.contains(secret), "{secret}");
        }
    }

    #[test]
    fn skips_corrupt_entries() {
        let scan = parse(
            &b": 12:0\n: :0;ls\n: 99999999999999999999:0;ls\n"[..],
            &Installed::default(),
        );

        assert!(scan.commands.is_empty());
        assert_eq!(scan.skipped_lines, 3);
    }
}

use std::io::Read;

use jiff::Timestamp;

use super::{HistoryScan, Installed, for_each_line, parse_line};

pub fn parse(reader: impl Read, installed: &Installed) -> HistoryScan {
    let mut scan = HistoryScan::default();
    let mut pending_time = None;
    let mut in_timed_entry = false;
    let result = for_each_line(reader, |line| {
        let Some(line) = line else {
            scan.skipped_lines += 1;
            return;
        };
        let text = String::from_utf8_lossy(line);
        // bash writes "#<epoch>" before each command when HISTTIMEFORMAT is set
        if let Some(digits) = timestamp_digits(&text) {
            pending_time = digits
                .parse()
                .ok()
                .and_then(|seconds| Timestamp::from_second(seconds).ok());
            if pending_time.is_none() {
                scan.skipped_lines += 1;
            }
            in_timed_entry = false;
            return;
        }
        // the lines after the first in a timed entry belong to the same multi-line command
        if pending_time.is_none() && in_timed_entry {
            return;
        }
        in_timed_entry = pending_time.is_some();
        scan.commands
            .extend(parse_line(&text, pending_time.take(), installed));
    });
    if result.is_err() {
        scan.skipped_lines += 1;
    }
    scan
}

fn timestamp_digits(line: &str) -> Option<&str> {
    let digits = line.strip_prefix('#')?;
    (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())).then_some(digits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{fixture, summary};

    #[test]
    fn parses_history_with_and_without_timestamps() {
        let scan = parse(fixture("bash_history"), &Installed::default());

        assert_eq!(
            summary(&scan),
            [
                ("ls", None, None),
                ("git", Some("pull"), None),
                ("git", Some("status"), Some(1_767_225_600)),
                ("cargo", Some("test"), Some(1_767_225_660)),
                ("for", None, Some(1_767_225_720)),
                ("docker", Some("ps"), None),
                ("systemctl", Some("restart"), Some(1_767_225_780)),
            ]
        );
        assert_eq!(scan.skipped_lines, 1);
    }

    #[test]
    fn keeps_no_arguments() {
        let scan = parse(fixture("bash_history"), &Installed::default());
        let kept = format!("{:?}", scan.commands);

        for secret in ["hunter2", "private", "secret-arg", "nginx", "done"] {
            assert!(!kept.contains(secret), "{secret}");
        }
    }

    #[test]
    fn treats_comments_as_no_command() {
        let scan = parse(&b"# a note\n#\nls\n"[..], &Installed::default());

        assert_eq!(summary(&scan), [("ls", None, None)]);
        assert_eq!(scan.skipped_lines, 0);
    }
}

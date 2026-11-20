use std::io::Read;

use super::{HistoryScan, Installed, for_each_line, parse_line};

/// the history has no timestamps, so every command is undated
pub fn parse(reader: impl Read, installed: &Installed) -> HistoryScan {
    let mut scan = HistoryScan::default();
    let mut continued = false;
    let result = for_each_line(reader, |line| {
        let Some(line) = line else {
            scan.skipped_lines += 1;
            continued = false;
            return;
        };
        // each line of a multi-line command except the last ends with a backtick
        let was_continued = continued;
        continued = line.last() == Some(&b'`');
        if was_continued {
            return;
        }
        scan.commands
            .extend(parse_line(&String::from_utf8_lossy(line), None, installed));
    });
    if result.is_err() {
        scan.skipped_lines += 1;
    }
    scan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{fixture, summary};

    #[test]
    fn parses_history() {
        let scan = parse(fixture("powershell_history"), &Installed::default());

        assert_eq!(
            summary(&scan),
            [
                ("git", Some("status"), None),
                ("Get-ChildItem", None, None),
                ("other", None, None),
                ("cargo", Some("build"), None),
                ("Write-Host", None, None),
                ("docker", Some("ps"), None),
                ("other", None, None),
                ("winget", Some("install"), None),
            ]
        );
        assert_eq!(scan.skipped_lines, 0);
    }

    #[test]
    fn keeps_no_arguments() {
        let scan = parse(fixture("powershell_history"), &Installed::default());
        let kept = format!("{:?}", scan.commands);

        for secret in ["hunter2", "private", "secret-arg", "Tools", "Some.App"] {
            assert!(!kept.contains(secret), "{secret}");
        }
    }
}

use super::names::{KNOWN_COMMANDS, SUBCOMMANDS, is_known_command, subcommands};
use super::{OTHER, ParsedCommand};

/// returns the typo in `command`, such as `gti` or `git stauts`, when it has one
pub fn find_typo(command: &ParsedCommand) -> Option<String> {
    if command.name == OTHER {
        return None;
    }
    let name = command.name.to_ascii_lowercase();
    if !is_known_command(&name) {
        return is_command_typo(&name).then(|| command.name.clone());
    }

    let subcommand = command.subcommand.as_ref()?;
    let known_subcommands = subcommands(&name)?;
    let typed = subcommand.to_ascii_lowercase();
    if known_subcommands.contains(&typed.as_str()) {
        return None;
    }
    known_subcommands
        .iter()
        .any(|known| is_one_edit(&typed, known))
        .then(|| format!("{} {subcommand}", command.name))
}

/// returns the name a typo from `find_typo` most likely meant, such as `git` for `gti`
pub fn correction(typo: &str) -> Option<String> {
    let typo = typo.to_ascii_lowercase();
    if let Some((tool, subcommand)) = typo.split_once(' ') {
        let fix = subcommands(tool)?
            .iter()
            .find(|known| is_one_edit(subcommand, known))?;
        return Some(format!("{tool} {fix}"));
    }
    KNOWN_COMMANDS
        .iter()
        .chain(SUBCOMMANDS.iter().map(|(tool, _)| tool))
        .find(|known| is_one_edit(&typo, known))
        .map(|known| known.to_string())
}

pub(crate) fn is_command_typo(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    !is_known_command(&name)
        && KNOWN_COMMANDS
            .iter()
            .chain(SUBCOMMANDS.iter().map(|(tool, _)| tool))
            .any(|known| is_one_edit(&name, known))
}

// a swap of neighbors is 1 edit, and a typed name of 1 or 2 letters only counts through a swap
pub(crate) fn is_one_edit(typed: &str, known: &str) -> bool {
    let (a, b) = (typed.as_bytes(), known.as_bytes());
    if a.len() == b.len() {
        let Some(i) = a.iter().zip(b).position(|(x, y)| x != y) else {
            return false;
        };
        let swapped =
            i + 1 < a.len() && a[i] == b[i + 1] && a[i + 1] == b[i] && a[i + 2..] == b[i + 2..];
        let changed = a.len() > 2 && a[i + 1..] == b[i + 1..];
        swapped || changed
    } else if a.len() > 2 && a.len().abs_diff(b.len()) == 1 {
        let (short, long) = if a.len() < b.len() { (a, b) } else { (b, a) };
        let i = short.iter().zip(long).take_while(|(x, y)| x == y).count();
        short[i..] == long[i + 1..]
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typo(name: &str, subcommand: Option<&str>) -> Option<String> {
        find_typo(&ParsedCommand {
            name: name.to_string(),
            subcommand: subcommand.map(str::to_string),
            time: None,
        })
    }

    #[test]
    fn finds_command_typos() {
        for name in ["gti", "sl", "lss", "carg", "grpe", "dokcer", "Gti"] {
            assert_eq!(typo(name, None).as_deref(), Some(name), "{name}");
        }
    }

    #[test]
    fn ignores_known_names_aliases_and_unrelated_names() {
        for name in [
            "git",
            "Git",
            "ls",
            "gs",
            "ll",
            "la",
            "x",
            "mycooltool",
            "other",
        ] {
            assert_eq!(typo(name, None), None, "{name}");
        }
    }

    #[test]
    fn finds_subcommand_typos_for_known_tools() {
        assert_eq!(typo("git", Some("stauts")).as_deref(), Some("git stauts"));
        assert_eq!(typo("cargo", Some("biuld")).as_deref(), Some("cargo biuld"));
        assert_eq!(typo("git", Some("status")), None);
        assert_eq!(typo("git", Some("st")), None);
        assert_eq!(typo("brew", Some("instal")).as_deref(), Some("brew instal"));
    }

    #[test]
    fn corrects_typos() {
        let cases = [
            ("gti", Some("git")),
            ("Gti", Some("git")),
            ("dokcer", Some("docker")),
            ("git stauts", Some("git status")),
            ("cargo biuld", Some("cargo build")),
            ("mycooltool", None),
            ("git xyzzy", None),
        ];
        for (typo, fix) in cases {
            assert_eq!(correction(typo).as_deref(), fix, "{typo}");
        }
    }

    #[test]
    fn measures_one_edit() {
        let cases = [
            ("gti", "git", true),
            ("sl", "ls", true),
            ("gut", "git", true),
            ("gitt", "git", true),
            ("gi", "git", false),
            ("gs", "go", false),
            ("tig", "git", false),
            ("git", "git", false),
        ];
        for (typed, known, expected) in cases {
            assert_eq!(is_one_edit(typed, known), expected, "{typed} {known}");
        }
    }
}

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::{HistoryScan, Installed, bash, fish, powershell, zsh};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl Shell {
    pub fn parse(self, reader: impl Read, installed: &Installed) -> HistoryScan {
        match self {
            Shell::Bash => bash::parse(reader, installed),
            Shell::Zsh => zsh::parse(reader, installed),
            Shell::Fish => fish::parse(reader, installed),
            Shell::PowerShell => powershell::parse(reader, installed),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryFile {
    pub shell: Shell,
    pub path: PathBuf,
}

/// lists the history files that exist, reading environment variables through `var`
pub fn find_history_files(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> Vec<HistoryFile> {
    let data_home = var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"));

    let mut candidates = vec![(Shell::Bash, home.join(".bash_history"))];
    if let Some(zdotdir) = var("ZDOTDIR") {
        candidates.push((Shell::Zsh, PathBuf::from(zdotdir).join(".zsh_history")));
    }
    candidates.push((Shell::Zsh, home.join(".zsh_history")));
    candidates.push((Shell::Zsh, home.join(".histfile")));
    candidates.push((Shell::Fish, data_home.join("fish").join("fish_history")));

    let mut psreadline = vec![data_home.join("powershell").join("PSReadLine")];
    if let Some(appdata) = var("APPDATA") {
        psreadline.push(
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("PowerShell")
                .join("PSReadLine"),
        );
    }
    for folder in psreadline {
        candidates.extend(host_history_files(&folder).map(|path| (Shell::PowerShell, path)));
    }

    // shells rarely export HISTFILE, and it can belong to bash or zsh
    if let Some(histfile) = var("HISTFILE").map(PathBuf::from) {
        candidates.push((bash_or_zsh(&histfile), histfile));
    }

    let mut files: Vec<HistoryFile> = Vec::new();
    for (shell, path) in candidates {
        if path.is_file() && !files.iter().any(|file| same_file(&file.path, &path)) {
            files.push(HistoryFile { shell, path });
        }
    }
    files
}

// every PowerShell host, such as the console or an editor, keeps its own file
fn host_history_files(folder: &Path) -> impl Iterator<Item = PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with("_history.txt"))
        })
        .collect();
    paths.sort();
    paths.into_iter()
}

// both parsers read plain lines the same way, so only zsh extended entries need zsh
fn bash_or_zsh(path: &Path) -> Shell {
    let mut start = [0u8; 3];
    let read = File::open(path)
        .and_then(|mut file| file.read(&mut start))
        .unwrap_or(0);
    if read == 3 && start.starts_with(b": ") && start[2].is_ascii_digit() {
        Shell::Zsh
    } else {
        Shell::Bash
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    a == b
        || matches!(
            (fs::canonicalize(a), fs::canonicalize(b)),
            (Ok(a), Ok(b)) if a == b
        )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn touch(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn find(home: &Path, env: &[(&str, &Path)]) -> Vec<(Shell, PathBuf)> {
        let env: HashMap<&str, OsString> = env
            .iter()
            .map(|(name, value)| (*name, value.as_os_str().to_os_string()))
            .collect();
        find_history_files(home, |name| env.get(name).cloned())
            .into_iter()
            .map(|file| (file.shell, file.path))
            .collect()
    }

    #[test]
    fn finds_default_files_that_exist() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        let bash = home.join(".bash_history");
        let zsh = home.join(".zsh_history");
        let fish = home
            .join(".local")
            .join("share")
            .join("fish")
            .join("fish_history");
        let powershell = home
            .join(".local")
            .join("share")
            .join("powershell")
            .join("PSReadLine")
            .join("ConsoleHost_history.txt");
        for path in [&bash, &zsh, &fish, &powershell] {
            touch(path, "ls\n");
        }

        assert_eq!(
            find(home, &[]),
            [
                (Shell::Bash, bash),
                (Shell::Zsh, zsh),
                (Shell::Fish, fish),
                (Shell::PowerShell, powershell),
            ]
        );
    }

    #[test]
    fn finds_nothing_in_an_empty_home() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find(dir.path(), &[]).is_empty());
    }

    #[test]
    fn follows_environment_folders() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let data = dir.path().join("data");
        let zdotdir = dir.path().join("zsh");
        let appdata = dir.path().join("appdata");
        let fish = data.join("fish").join("fish_history");
        let zsh = zdotdir.join(".zsh_history");
        let psreadline = appdata
            .join("Microsoft")
            .join("Windows")
            .join("PowerShell")
            .join("PSReadLine");
        let console = psreadline.join("ConsoleHost_history.txt");
        let editor = psreadline.join("Visual Studio Code Host_history.txt");
        for path in [&fish, &zsh, &console, &editor] {
            touch(path, "ls\n");
        }
        touch(&psreadline.join("notes.txt"), "not history\n");

        let found = find(
            &home,
            &[
                ("XDG_DATA_HOME", &data),
                ("ZDOTDIR", &zdotdir),
                ("APPDATA", &appdata),
            ],
        );

        assert_eq!(
            found,
            [
                (Shell::Zsh, zsh),
                (Shell::Fish, fish),
                (Shell::PowerShell, console),
                (Shell::PowerShell, editor),
            ]
        );
    }

    #[test]
    fn reads_histfile_as_bash_or_zsh() {
        let dir = tempfile::tempdir().unwrap();
        let zsh = dir.path().join("zsh_custom");
        let bash = dir.path().join("bash_custom");
        touch(&zsh, ": 1767225600:0;ls\n");
        touch(&bash, "#1767225600\nls\n");

        assert_eq!(find(dir.path(), &[("HISTFILE", &zsh)]), [(Shell::Zsh, zsh)]);
        assert_eq!(
            find(dir.path(), &[("HISTFILE", &bash)]),
            [(Shell::Bash, bash)]
        );
    }

    #[test]
    fn lists_a_histfile_at_a_default_path_once() {
        let dir = tempfile::tempdir().unwrap();
        let zsh = dir.path().join(".zsh_history");
        touch(&zsh, "ls\n");

        assert_eq!(find(dir.path(), &[("HISTFILE", &zsh)]), [(Shell::Zsh, zsh)]);
    }
}

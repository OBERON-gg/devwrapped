use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;

// other names show only when installed, so a typed password stays hidden
pub(crate) const KNOWN_COMMANDS: &[&str] = &[
    "abbr",
    "alias",
    "and",
    "atop",
    "awk",
    "basename",
    "bash",
    "bat",
    "begin",
    "bg",
    "bind",
    "break",
    "btop",
    "builtin",
    "bunx",
    "case",
    "cat",
    "cd",
    "chmod",
    "chown",
    "clang",
    "clang++",
    "claude",
    "clear",
    "cls",
    "cmake",
    "code",
    "codex",
    "command",
    "complete",
    "continue",
    "copy",
    "cp",
    "crontab",
    "curl",
    "cut",
    "date",
    "dd",
    "declare",
    "del",
    "df",
    "diff",
    "dig",
    "dir",
    "dirname",
    "disown",
    "dmesg",
    "do",
    "done",
    "du",
    "duf",
    "dust",
    "echo",
    "egrep",
    "emacs",
    "end",
    "env",
    "eval",
    "exec",
    "exit",
    "explorer",
    "export",
    "eza",
    "false",
    "fastfetch",
    "fd",
    "fg",
    "fgrep",
    "file",
    "find",
    "fish",
    "for",
    "free",
    "function",
    "functions",
    "fzf",
    "g++",
    "gcc",
    "gitk",
    "grep",
    "gzip",
    "hash",
    "head",
    "help",
    "history",
    "host",
    "hostname",
    "htop",
    "id",
    "if",
    "ip",
    "ipconfig",
    "java",
    "javac",
    "jobs",
    "journalctl",
    "jq",
    "kill",
    "killall",
    "kubectx",
    "kubens",
    "l",
    "la",
    "less",
    "let",
    "ll",
    "ln",
    "local",
    "locate",
    "logout",
    "ls",
    "lsblk",
    "lsd",
    "lua",
    "luac",
    "make",
    "man",
    "mkdir",
    "more",
    "mount",
    "move",
    "mv",
    "mvn",
    "nano",
    "neofetch",
    "node",
    "nohup",
    "not",
    "notepad",
    "npx",
    "nu",
    "nvim",
    "nvm",
    "open",
    "or",
    "pacman",
    "paru",
    "passwd",
    "patch",
    "perf",
    "perl",
    "pgrep",
    "php",
    "ping",
    "pipx",
    "pkill",
    "pnpx",
    "popd",
    "printenv",
    "printf",
    "ps",
    "pushd",
    "pwd",
    "pwsh",
    "python",
    "python3",
    "read",
    "readlink",
    "realpath",
    "reboot",
    "ren",
    "return",
    "rg",
    "rm",
    "rmdir",
    "rsync",
    "ruby",
    "rustc",
    "scp",
    "screen",
    "sed",
    "set",
    "set_color",
    "sh",
    "shift",
    "shopt",
    "shutdown",
    "sleep",
    "sort",
    "source",
    "ss",
    "ssh",
    "stat",
    "string",
    "su",
    "sudo",
    "switch",
    "tail",
    "tar",
    "tee",
    "test",
    "then",
    "tig",
    "time",
    "tmux",
    "top",
    "touch",
    "trap",
    "tree",
    "true",
    "type",
    "typeset",
    "ulimit",
    "umask",
    "umount",
    "unalias",
    "uname",
    "uniq",
    "unset",
    "until",
    "unzip",
    "uptime",
    "uvx",
    "vi",
    "vim",
    "wait",
    "watch",
    "wc",
    "wget",
    "where",
    "which",
    "while",
    "whoami",
    "xargs",
    "xdg-open",
    "yay",
    "yq",
    "z",
    "zed",
    "zip",
    "zoxide",
    "zsh",
];

// the next word can be a script or file name, so only these count as subcommands
pub(crate) const SUBCOMMANDS: &[(&str, &[&str])] = &[
    (
        "apt",
        &[
            "autoremove",
            "full-upgrade",
            "install",
            "list",
            "policy",
            "purge",
            "remove",
            "search",
            "show",
            "update",
            "upgrade",
        ],
    ),
    (
        "apt-get",
        &[
            "autoremove",
            "clean",
            "dist-upgrade",
            "install",
            "purge",
            "remove",
            "source",
            "update",
            "upgrade",
        ],
    ),
    (
        "aws",
        &[
            "cloudformation",
            "configure",
            "dynamodb",
            "ec2",
            "ecr",
            "ecs",
            "eks",
            "iam",
            "kms",
            "lambda",
            "logs",
            "rds",
            "route53",
            "s3",
            "secretsmanager",
            "sns",
            "sqs",
            "ssm",
            "sso",
            "sts",
        ],
    ),
    (
        "az",
        &[
            "account",
            "acr",
            "ad",
            "aks",
            "deployment",
            "functionapp",
            "group",
            "keyvault",
            "login",
            "monitor",
            "network",
            "role",
            "storage",
            "vm",
            "webapp",
        ],
    ),
    (
        "brew",
        &[
            "cleanup",
            "doctor",
            "info",
            "install",
            "link",
            "list",
            "outdated",
            "reinstall",
            "search",
            "services",
            "tap",
            "uninstall",
            "unlink",
            "untap",
            "update",
            "upgrade",
        ],
    ),
    (
        "bun",
        &[
            "add", "build", "create", "dev", "init", "install", "pm", "remove", "run", "test",
            "update", "upgrade", "x",
        ],
    ),
    (
        "cargo",
        &[
            "add", "bench", "build", "check", "clean", "clippy", "doc", "fmt", "init", "install",
            "new", "publish", "remove", "run", "search", "test", "tree", "update",
        ],
    ),
    (
        "choco",
        &[
            "info",
            "install",
            "list",
            "outdated",
            "search",
            "uninstall",
            "upgrade",
        ],
    ),
    (
        "conda",
        &[
            "activate",
            "config",
            "create",
            "deactivate",
            "env",
            "info",
            "install",
            "list",
            "remove",
            "search",
            "update",
        ],
    ),
    (
        "deno",
        &[
            "add", "bench", "cache", "check", "compile", "doc", "fmt", "info", "init", "install",
            "lint", "repl", "run", "serve", "task", "test", "upgrade",
        ],
    ),
    (
        "dnf",
        &[
            "autoremove",
            "group",
            "history",
            "info",
            "install",
            "list",
            "makecache",
            "remove",
            "repolist",
            "search",
            "update",
            "upgrade",
        ],
    ),
    (
        "docker",
        &[
            "build", "compose", "exec", "images", "inspect", "kill", "login", "logs", "network",
            "ps", "pull", "push", "restart", "rm", "rmi", "run", "start", "stop", "system",
            "volume",
        ],
    ),
    (
        "docker-compose",
        &[
            "build", "config", "down", "exec", "logs", "ps", "pull", "restart", "run", "start",
            "stop", "up",
        ],
    ),
    (
        "dotnet",
        &[
            "add", "build", "clean", "ef", "format", "new", "nuget", "pack", "publish", "remove",
            "restore", "run", "sln", "test", "tool", "watch",
        ],
    ),
    (
        "flatpak",
        &[
            "info",
            "install",
            "list",
            "remote-add",
            "remotes",
            "run",
            "search",
            "uninstall",
            "update",
        ],
    ),
    (
        "gcloud",
        &[
            "app",
            "auth",
            "builds",
            "components",
            "compute",
            "config",
            "container",
            "functions",
            "iam",
            "init",
            "projects",
            "run",
            "services",
            "sql",
            "storage",
        ],
    ),
    (
        "gh",
        &[
            "api",
            "auth",
            "browse",
            "cache",
            "codespace",
            "gist",
            "issue",
            "label",
            "pr",
            "release",
            "repo",
            "run",
            "search",
            "secret",
            "status",
            "workflow",
        ],
    ),
    (
        "git",
        &[
            "add",
            "bisect",
            "blame",
            "branch",
            "checkout",
            "cherry-pick",
            "clean",
            "clone",
            "commit",
            "config",
            "diff",
            "fetch",
            "init",
            "log",
            "merge",
            "mv",
            "pull",
            "push",
            "rebase",
            "reflog",
            "remote",
            "reset",
            "restore",
            "revert",
            "rm",
            "show",
            "stash",
            "status",
            "switch",
            "tag",
            "worktree",
        ],
    ),
    (
        "glab",
        &[
            "api", "auth", "ci", "issue", "mr", "pipeline", "release", "repo",
        ],
    ),
    (
        "go",
        &[
            "build", "env", "fmt", "generate", "get", "install", "mod", "run", "test", "vet",
            "work",
        ],
    ),
    (
        "helm",
        &[
            "create",
            "dependency",
            "get",
            "history",
            "install",
            "lint",
            "list",
            "package",
            "pull",
            "repo",
            "rollback",
            "search",
            "show",
            "status",
            "template",
            "uninstall",
            "upgrade",
        ],
    ),
    (
        "kubectl",
        &[
            "apply",
            "create",
            "delete",
            "describe",
            "edit",
            "exec",
            "get",
            "logs",
            "port-forward",
            "rollout",
            "scale",
        ],
    ),
    (
        "npm",
        &[
            "audit",
            "ci",
            "exec",
            "init",
            "install",
            "link",
            "list",
            "outdated",
            "publish",
            "run",
            "start",
            "test",
            "uninstall",
            "update",
        ],
    ),
    (
        "pip",
        &[
            "cache",
            "check",
            "config",
            "download",
            "freeze",
            "install",
            "list",
            "search",
            "show",
            "uninstall",
            "wheel",
        ],
    ),
    (
        "pip3",
        &[
            "cache",
            "check",
            "config",
            "download",
            "freeze",
            "install",
            "list",
            "search",
            "show",
            "uninstall",
            "wheel",
        ],
    ),
    (
        "pnpm",
        &[
            "add", "build", "create", "dev", "dlx", "exec", "init", "install", "list", "outdated",
            "publish", "remove", "run", "start", "store", "test", "update", "why",
        ],
    ),
    (
        "podman",
        &[
            "build", "compose", "exec", "images", "inspect", "kill", "login", "logs", "network",
            "ps", "pull", "push", "restart", "rm", "rmi", "run", "start", "stop", "system",
            "volume",
        ],
    ),
    (
        "poetry",
        &[
            "add", "build", "check", "config", "env", "init", "install", "lock", "new", "publish",
            "remove", "run", "shell", "show", "update",
        ],
    ),
    (
        "rustup",
        &[
            "component",
            "default",
            "doc",
            "install",
            "override",
            "run",
            "self",
            "show",
            "target",
            "toolchain",
            "uninstall",
            "update",
        ],
    ),
    (
        "scoop",
        &[
            "bucket",
            "cleanup",
            "info",
            "install",
            "list",
            "reset",
            "search",
            "status",
            "uninstall",
            "update",
        ],
    ),
    (
        "snap",
        &[
            "find", "info", "install", "list", "refresh", "remove", "run", "services",
        ],
    ),
    (
        "systemctl",
        &[
            "cat",
            "daemon-reload",
            "disable",
            "edit",
            "enable",
            "is-active",
            "is-enabled",
            "list-units",
            "mask",
            "reload",
            "restart",
            "start",
            "status",
            "stop",
            "unmask",
        ],
    ),
    (
        "terraform",
        &[
            "apply",
            "console",
            "destroy",
            "fmt",
            "import",
            "init",
            "login",
            "output",
            "plan",
            "providers",
            "refresh",
            "show",
            "state",
            "taint",
            "validate",
            "workspace",
        ],
    ),
    (
        "uv",
        &[
            "add", "build", "export", "init", "lock", "pip", "publish", "python", "remove", "run",
            "sync", "tool", "tree", "venv",
        ],
    ),
    (
        "winget",
        &[
            "install",
            "list",
            "search",
            "show",
            "source",
            "uninstall",
            "upgrade",
        ],
    ),
    (
        "yarn",
        &[
            "add",
            "build",
            "create",
            "dev",
            "dlx",
            "global",
            "info",
            "init",
            "install",
            "remove",
            "run",
            "start",
            "test",
            "up",
            "upgrade",
            "why",
            "workspace",
            "workspaces",
        ],
    ),
];

// powershell cmdlets live inside the shell, so they are never on the path
const CMDLET_VERBS: &[&str] = &[
    "add",
    "clear",
    "compare",
    "connect",
    "convertfrom",
    "convertto",
    "copy",
    "disable",
    "disconnect",
    "enable",
    "enter",
    "exit",
    "expand",
    "export",
    "find",
    "foreach",
    "format",
    "get",
    "group",
    "import",
    "install",
    "invoke",
    "join",
    "measure",
    "move",
    "new",
    "out",
    "pop",
    "push",
    "read",
    "receive",
    "register",
    "remove",
    "rename",
    "resolve",
    "restart",
    "save",
    "select",
    "send",
    "set",
    "show",
    "sort",
    "split",
    "start",
    "stop",
    "test",
    "uninstall",
    "unregister",
    "update",
    "wait",
    "where",
    "write",
];

const EXECUTABLE_EXTENSIONS: [&str; 5] = [".exe", ".cmd", ".bat", ".com", ".ps1"];

/// lowercase names of the files in the folders on the path, without windows executable extensions
#[derive(Debug, Default)]
pub struct Installed(HashSet<String>);

impl Installed {
    pub fn from_path(path: Option<&OsStr>) -> Installed {
        let mut names = HashSet::new();
        for folder in path.map(env::split_paths).into_iter().flatten() {
            let Ok(entries) = fs::read_dir(folder) else {
                continue;
            };
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    names.insert(command_name(name));
                }
            }
        }
        Installed(names)
    }

    #[cfg(test)]
    pub(crate) fn of(names: &[&str]) -> Installed {
        Installed(names.iter().map(|name| name.to_ascii_lowercase()).collect())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.contains(&name.to_ascii_lowercase())
    }
}

fn command_name(file_name: &str) -> String {
    let lower = file_name.to_ascii_lowercase();
    EXECUTABLE_EXTENSIONS
        .iter()
        .find_map(|extension| lower.strip_suffix(extension))
        .map_or_else(|| lower.clone(), str::to_string)
}

pub(crate) fn is_known_command(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    KNOWN_COMMANDS.contains(&name.as_str()) || subcommands(&name).is_some()
}

pub(crate) fn subcommands(tool: &str) -> Option<&'static [&'static str]> {
    let tool = tool.to_ascii_lowercase();
    SUBCOMMANDS
        .iter()
        .find(|(known, _)| *known == tool)
        .map(|(_, subcommands)| *subcommands)
}

pub(crate) fn is_cmdlet(name: &str) -> bool {
    let Some((verb, noun)) = name.split_once('-') else {
        return false;
    };
    CMDLET_VERBS.contains(&verb.to_ascii_lowercase().as_str())
        && !noun.is_empty()
        && noun.bytes().all(|byte| byte.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_subcommand_tool_once() {
        for (index, (tool, words)) in SUBCOMMANDS.iter().enumerate() {
            assert!(!words.is_empty(), "{tool}");
            assert!(!KNOWN_COMMANDS.contains(tool), "{tool}");
            assert!(
                SUBCOMMANDS[..index].iter().all(|(other, _)| other != tool),
                "{tool}"
            );
        }
    }

    #[test]
    fn reads_installed_names_from_the_path() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        fs::write(first.path().join("zola"), "").unwrap();
        fs::write(second.path().join("Deploy.EXE"), "").unwrap();
        fs::write(second.path().join("build.ps1"), "").unwrap();
        let missing = first.path().join("missing");
        let path = env::join_paths([first.path(), &missing, second.path()]).unwrap();
        let installed = Installed::from_path(Some(&path));
        for name in ["zola", "ZOLA", "deploy", "build"] {
            assert!(installed.contains(name), "{name}");
        }
        assert!(!installed.contains("hunter2"));
        assert!(!Installed::from_path(None).contains("zola"));
    }

    #[test]
    fn knows_cmdlets_by_verb() {
        for name in ["Get-ChildItem", "set-location", "ConvertTo-Json"] {
            assert!(is_cmdlet(name), "{name}");
        }
        for name in [
            "hunter2",
            "Get-",
            "Fake-Thing",
            "Get-Item2",
            "get-child-item",
        ] {
            assert!(!is_cmdlet(name), "{name}");
        }
    }
}

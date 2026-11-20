# Privacy

devwrapped reads data that can be sensitive. Shell history and agent logs can hold passwords, tokens, and private text. This page lists every file the tool opens, what it takes from each one, and what it keeps.

"Keeps" means the value stays in memory after a record is parsed. Everything else is dropped while the record is parsed. Nothing is kept after the run ends, apart from the files you ask for.

## Summary

- devwrapped runs on your machine and makes no network calls. The binary has no network code, and `cargo deny` bans HTTP, socket, TLS, SSH, and DNS crates in CI.
- It prints a summary in the terminal. It writes files only when you ask for them with `--format`: one SVG or PNG file per slide, or one HTML file. It writes no cache, config, or log files.
- It never prints repository paths or branch names. Repository names stay hidden too. `--show-repo-names` shows the folder names of your 5 most active repositories, and nothing above them in the path.
- `--skip` leaves a source out. A skipped source is never opened.
- `--sample` uses built-in synthetic data and reads nothing from your machine.

## Environment

devwrapped reads these environment variables:

| Variable | Use |
| --- | --- |
| `HOME`, or `USERPROFILE` on Windows | The home folder, where the scan and the default paths start |
| `PATH` | The folders whose file names tell which commands are installed |
| `HISTFILE`, `ZDOTDIR`, `XDG_DATA_HOME`, `APPDATA` | Shell history locations |
| `CLAUDE_CONFIG_DIR`, `CODEX_HOME` | Agent log locations |
| `NO_COLOR` | Turns off color in the terminal |
| `LC_ALL`, `LC_TIME`, `LANG` | The region of your locale, to choose a 12-hour or 24-hour clock |

The values are used only to find files and to choose the clock. They are never printed.

Unless you pass `--clock`, devwrapped also reads 1 system setting for the clock. On Windows it runs `reg query` for the `iTime` value in `HKCU\Control Panel\International`. On macOS it runs `defaults read -g` for `AppleICUForce24HourTime` and `AppleICUForce12HourTime`.

For `PATH`, devwrapped lists the file names in each folder and keeps them in lowercase, with the Windows extensions such as `.exe` removed. It uses the list only to decide whether a command name from your shell history may be shown. It never prints the list.

## Git

devwrapped looks for repositories up to 4 folders below your home folder, or below each `--root`. It reads folder names to find them and skips hidden folders, `node_modules`, and `target`.

From each repository it reads:

- `user.email` and `author.email` from the git config, including your global config
- HEAD, the local branches, and the remote-tracking branches, to find where to start the walk
- the shallow file, if the clone is shallow
- each commit back to the start of the range: its parents, its author email, its author time, and its tree
- the committer time, only to stop the walk at the start of the range
- the paths that changed between each commit and its first parent

It keeps:

- the author time of each of your commits in the range
- the number of changed files in each commit
- the extension of each changed file, when it is 16 characters or less and only letters and digits
- the number of your commits in each repository
- the commit ids, only to count a commit once when it is on several branches

It uses the emails from the git config and `--email` only to decide which commits are yours. Your emails are never printed. A commit by anyone else is skipped. Merge commits are skipped.

It never uses commit messages, author or committer names, the committer email, file contents, or diffs. It never keeps file names, folder names, branch names, or emails. The path of a repository is used only to open it, and with `--show-repo-names` its last folder name goes on the list of your 5 most active repositories.

## Shell history

devwrapped reads these history files when they exist:

| Shell | Files |
| --- | --- |
| bash | `~/.bash_history`, or `$HISTFILE` |
| zsh | `~/.zsh_history`, `~/.histfile`, `$ZDOTDIR/.zsh_history`, or `$HISTFILE` |
| fish | `~/.local/share/fish/fish_history`, or the same file under `$XDG_DATA_HOME` |
| PowerShell | every `*_history.txt` file in `%APPDATA%\Microsoft\Windows\PowerShell\PSReadLine` and `~/.local/share/powershell/PSReadLine`, or the same folder under `$XDG_DATA_HOME` |

When `$HISTFILE` is set, devwrapped reads its first 3 bytes to tell whether it uses the zsh format.

It reads each file one line at a time. A line longer than 64 KB is skipped without being kept. From each line it takes:

- the command text
- the timestamp, when the format has one: the `: <time>:<duration>;` prefix in zsh, a `#<time>` line before the command in bash, and the `when:` line in fish. The zsh duration is not kept. PowerShell history has no timestamps.

From the command text, it keeps only:

- the command name
- the first subcommand, when the command is a known tool
- the timestamp, if there is one

All arguments are dropped while the line is parsed. Before it reads the command name, it removes leading environment assignments such as `TOKEN=abc`. It also removes the wrappers `sudo`, `time`, `nohup`, and `env`, along with the values of their options, such as the user name in `sudo -u`.

A command name is kept only when it is on the built-in list of tools, a program in a `PATH` folder, a PowerShell cmdlet, or 1 edit away from a known name. Every other name is kept as `other`, so a password typed alone on a line never shows. A command name longer than 32 characters, or with a character outside `A-Za-z0-9._+-`, is also kept as `other`.

A subcommand is kept only when it is on the built-in list for that tool, or 1 edit away from one. A script or file name after the command is never kept.

The top commands, the top subcommands, and the typo of the year show only names you used 3 or more times. A name used once or twice still counts in the totals, but never shows.

## Agent logs

devwrapped reads the `.jsonl` files up to 4 folders below `~/.claude/projects` for Claude Code, and below `~/.codex/sessions` for Codex CLI. `CLAUDE_CONFIG_DIR` and `CODEX_HOME` move these folders. It reads no other file in them, such as settings, `auth.json`, or `session_index.jsonl`.

It reads each file one line at a time and never loads a whole file. A line longer than 16 MB is skipped without being kept. Each line is a JSON object. devwrapped parses only the fields below. Every other field is skipped by the parser and never stored.

From Claude Code logs it reads:

- `type`, `timestamp`, and `sessionId` of every record
- in assistant records: `message.id`, `message.model`, the 4 token counts in `message.usage`, and `type` and `name` of each block in `message.content`

From Codex CLI logs it reads:

- `type`, `timestamp`, and `payload.type` of every record
- `payload.id` and `payload.timestamp` of the session record
- `payload.model` of each turn record
- `payload.name` of each tool call
- `input_tokens`, `cached_input_tokens`, and `output_tokens` from `payload.info.total_token_usage` of each token record

It keeps, for each session:

- the start time
- the number of uses of each tool
- the number of turns or messages for each model
- the input, output, cache read, and cache write token counts
- the session id and the message ids, only to join records of the same session and to count each message once

Record timestamps are used only to find when each session started. Active days and the busiest hour come from the session start times.

MCP tool names hold the name of the server, so every MCP tool is kept as `mcp`. A tool or model name longer than 64 characters, or with a character outside `A-Za-z0-9._+-`, is kept as `other`. That keeps out model ids that hold an account number, such as a Bedrock ARN.

It never stores prompts, responses, reasoning, tool input, tool output, file contents, file paths, working folders, branch names, or account ids. [docs/agent-formats.md](docs/agent-formats.md) describes both log formats and lists the fields that hold private text.

## Output

The terminal summary and the slides show:

- the range
- which sources were found, not found, or skipped
- counts, days, streaks, hours, and weekdays
- the names of up to 5 languages, commands, subcommands, agents, tools, and models, and the typo of the year
- the awards
- with `--show-repo-names`, the folder names of up to 5 repositories

They never show raw history lines, log text, commit data, paths, or emails.

The terminal summary removes control characters from names, so a crafted name cannot send escape codes to your terminal. The SVG and HTML files escape every name. PNG files hold only the image and no metadata.

The HTML file holds every slide, the fonts, the styles, and the script. It has no external URLs and works offline.

Error messages never include a raw record. When a record or file cannot be read, devwrapped skips it and adds 1 to the count of unreadable records in the terminal summary.

## Tests

The test fixtures hold only synthetic data. The tests feed fake secrets into every source, including commit messages, file names, branch names, command arguments, bare lines, prompts, tool names, and model names. They then check that no secret appears in the terminal output, error messages, SVG, PNG, or HTML. Another test checks that the HTML file has no external URL.

## Reporting a privacy bug

If devwrapped shows or writes something it should not, please report it privately through GitHub: open the Security tab of the repository and choose "Report a vulnerability". Describe what showed up and where. Please do not paste real history, logs, or secrets into the report.

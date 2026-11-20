use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use walkdir::WalkDir;

pub mod claude;
pub mod codex;

// covers the subagent folders of Claude Code and the year/month/day folders of Codex CLI
const MAX_DEPTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Agent {
    ClaudeCode,
    CodexCli,
}

impl Agent {
    pub fn name(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "Claude Code",
            Agent::CodexCli => "Codex CLI",
        }
    }
}

/// `input` leaves out cached input, which `cache_read` counts
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl Tokens {
    pub fn total(&self) -> u64 {
        [self.input, self.output, self.cache_read, self.cache_write]
            .into_iter()
            .fold(0, u64::saturating_add)
    }
}

impl std::ops::AddAssign for Tokens {
    fn add_assign(&mut self, other: Tokens) {
        self.input = self.input.saturating_add(other.input);
        self.output = self.output.saturating_add(other.output);
        self.cache_read = self.cache_read.saturating_add(other.cache_read);
        self.cache_write = self.cache_write.saturating_add(other.cache_write);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    pub agent: Agent,
    pub start: Timestamp,
    pub tool_uses: BTreeMap<String, u64>,
    pub models: BTreeMap<String, u64>,
    pub tokens: Tokens,
}

#[derive(Debug, Default)]
pub struct AgentScan {
    /// sessions by session id, which can span several files
    pub sessions: BTreeMap<String, AgentSession>,
    pub skipped_lines: usize,
}

#[derive(Default)]
struct SessionBuilder {
    start: Option<Timestamp>,
    tool_uses: BTreeMap<String, u64>,
    models: BTreeMap<String, u64>,
    tokens: Tokens,
}

impl SessionBuilder {
    fn saw(&mut self, time: Timestamp) {
        self.start = Some(self.start.map_or(time, |start| start.min(time)));
    }

    // a session with no readable timestamp cannot be placed in the range, so it is dropped
    fn finish(self, agent: Agent) -> Option<AgentSession> {
        Some(AgentSession {
            agent,
            start: self.start?,
            tool_uses: self.tool_uses,
            models: self.models,
            tokens: self.tokens,
        })
    }
}

#[derive(Debug, Default)]
pub struct AgentCollection {
    pub found: bool,
    pub sessions: Vec<AgentSession>,
    pub skipped_files: usize,
    pub skipped_lines: usize,
}

/// reads every log file of `agent`, reading environment variables through `var`
pub fn collect(
    agent: Agent,
    home: &Path,
    var: impl Fn(&str) -> Option<OsString>,
) -> AgentCollection {
    let folder = log_folder(agent, home, var);
    let mut collection = AgentCollection {
        found: folder.is_dir(),
        ..AgentCollection::default()
    };
    let mut sessions: BTreeMap<String, AgentSession> = BTreeMap::new();
    for (path, len) in log_files(&folder) {
        let Ok(file) = File::open(&path) else {
            collection.skipped_files += 1;
            continue;
        };
        let scan = match agent {
            Agent::ClaudeCode => claude::parse(file),
            Agent::CodexCli => codex::parse(file),
        };
        // a file with content but no session is in a format this version does not know
        if scan.sessions.is_empty() && len > 0 {
            collection.skipped_files += 1;
        }
        collection.skipped_lines += scan.skipped_lines;
        for (id, session) in scan.sessions {
            match sessions.entry(id) {
                Entry::Vacant(entry) => {
                    entry.insert(session);
                }
                Entry::Occupied(mut entry) => entry.get_mut().merge(session),
            }
        }
    }
    collection.sessions = sessions.into_values().collect();
    collection
}

fn log_folder(agent: Agent, home: &Path, var: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    let (variable, default, logs) = match agent {
        Agent::ClaudeCode => ("CLAUDE_CONFIG_DIR", ".claude", "projects"),
        Agent::CodexCli => ("CODEX_HOME", ".codex", "sessions"),
    };
    var(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(default))
        .join(logs)
}

fn log_files(folder: &Path) -> impl Iterator<Item = (PathBuf, u64)> {
    WalkDir::new(folder)
        .max_depth(MAX_DEPTH)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "jsonl")
        })
        .map(|entry| {
            let len = entry.metadata().map_or(0, |metadata| metadata.len());
            (entry.into_path(), len)
        })
}

impl AgentSession {
    fn merge(&mut self, other: AgentSession) {
        self.start = self.start.min(other.start);
        for (name, count) in other.tool_uses {
            *self.tool_uses.entry(name).or_default() += count;
        }
        for (name, count) in other.models {
            *self.models.entry(name).or_default() += count;
        }
        self.tokens += other.tokens;
    }
}

const MAX_NAME_LEN: usize = 64;
const OTHER: &str = "other";

// mcp tool names hold the server name, which can be private
fn tool_name(name: &str) -> String {
    if name.contains("__") {
        return "mcp".to_string();
    }
    checked_name(name)
}

// arns and profile paths hold account ids, so they fail the check
fn model_name(name: &str) -> String {
    let name = match name.rsplit_once(':') {
        Some((base, version))
            if !version.is_empty() && version.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => name,
    };
    checked_name(name)
}

fn checked_name(name: &str) -> String {
    let valid = !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte));
    if valid { name } else { OTHER }.to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;

    use super::*;

    #[test]
    fn collapses_mcp_tools() {
        assert_eq!(tool_name("mcp__fake-private-server__query"), "mcp");
        assert_eq!(tool_name("fake_server__lookup"), "mcp");
        assert_eq!(tool_name("Bash"), "Bash");
        assert_eq!(tool_name("apply_patch"), "apply_patch");
        assert_eq!(tool_name("/home/dev/fake-private-path"), "other");
    }

    #[test]
    fn keeps_plain_model_names_only() {
        let cases = [
            ("claude-opus-5-5", "claude-opus-5-5"),
            ("gpt-5.5-codex", "gpt-5.5-codex"),
            (
                "us.anthropic.claude-opus-5-5-v1:0",
                "us.anthropic.claude-opus-5-5-v1",
            ),
            (
                "arn:aws:bedrock:us-east-1:123456789012:application-inference-profile/x",
                "other",
            ),
            ("model with spaces", "other"),
            ("", "other"),
        ];
        for (raw, kept) in cases {
            assert_eq!(model_name(raw), kept, "{raw}");
        }
        assert_eq!(model_name(&"m".repeat(65)), "other");
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn collect_with(agent: Agent, home: &Path, env: &[(&str, &Path)]) -> AgentCollection {
        let env: HashMap<&str, OsString> = env
            .iter()
            .map(|(name, value)| (*name, value.as_os_str().to_os_string()))
            .collect();
        collect(agent, home, |name| env.get(name).cloned())
    }

    const CLAUDE_MAIN: &str = r#"{"type":"user","sessionId":"s1","timestamp":"2026-03-02T09:00:00Z"}
{"type":"assistant","sessionId":"s1","timestamp":"2026-03-02T09:00:01Z","message":{"id":"m1","usage":{"input_tokens":5},"content":[{"type":"tool_use","name":"Read"}]}}
"#;
    const CLAUDE_SUBAGENT: &str = r#"{"type":"assistant","sessionId":"s1","timestamp":"2026-03-02T08:30:00Z","message":{"id":"m2","usage":{"input_tokens":7},"content":[{"type":"tool_use","name":"Read"}]}}
"#;

    #[test]
    fn merges_claude_sessions_across_files() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join(".claude").join("projects").join("p");
        write(&project.join("s1.jsonl"), CLAUDE_MAIN);
        write(
            &project.join("s1").join("subagents").join("agent-a.jsonl"),
            CLAUDE_SUBAGENT,
        );
        write(&project.join("notes.md"), "not a log");

        let collection = collect_with(Agent::ClaudeCode, dir.path(), &[]);

        assert!(collection.found);
        assert_eq!(collection.sessions.len(), 1);
        let session = &collection.sessions[0];
        assert_eq!(session.start, "2026-03-02T08:30:00Z".parse().unwrap());
        assert_eq!(session.tool_uses["Read"], 2);
        assert_eq!(session.tokens.input, 12);
        assert_eq!((collection.skipped_files, collection.skipped_lines), (0, 0));
    }

    #[test]
    fn skips_unknown_files_and_truncated_lines() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join(".claude").join("projects").join("p");
        write(&project.join("a.jsonl"), r#"{"format":"something else"}"#);
        write(&project.join("b.jsonl"), "");
        let truncated = format!("{CLAUDE_MAIN}{{\"type\":\"assistant\",\"sessi");
        write(&project.join("c.jsonl"), &truncated);

        let collection = collect_with(Agent::ClaudeCode, dir.path(), &[]);

        assert_eq!(collection.sessions.len(), 1);
        assert_eq!(collection.skipped_files, 1);
        assert_eq!(collection.skipped_lines, 1);
    }

    #[test]
    fn follows_the_folder_variables() {
        let dir = tempfile::tempdir().unwrap();
        let claude_config = dir.path().join("claude-config");
        let codex_home = dir.path().join("codex-home");
        write(
            &claude_config.join("projects").join("p").join("s1.jsonl"),
            CLAUDE_MAIN,
        );
        write(
            &codex_home
                .join("sessions")
                .join("2026")
                .join("03")
                .join("02")
                .join("rollout-2026-03-02T10-00-00-c1.jsonl"),
            r#"{"timestamp":"2026-03-02T10:00:00Z","type":"session_meta","payload":{"id":"c1"}}
"#,
        );
        let env = [
            ("CLAUDE_CONFIG_DIR", claude_config.as_path()),
            ("CODEX_HOME", codex_home.as_path()),
        ];

        let claude = collect_with(Agent::ClaudeCode, &dir.path().join("home"), &env);
        let codex = collect_with(Agent::CodexCli, &dir.path().join("home"), &env);

        assert_eq!(claude.sessions.len(), 1);
        assert_eq!(codex.sessions.len(), 1);
        assert_eq!(codex.sessions[0].agent, Agent::CodexCli);
    }

    #[test]
    fn reports_a_missing_folder_as_not_found() {
        let dir = tempfile::tempdir().unwrap();

        let collection = collect_with(Agent::CodexCli, dir.path(), &[]);

        assert!(!collection.found);
        assert!(collection.sessions.is_empty());
        assert_eq!(collection.skipped_files, 0);
    }
}

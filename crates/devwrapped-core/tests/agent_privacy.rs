use std::ffi::OsString;
use std::path::{Path, PathBuf};

use devwrapped_core::agents::{Agent, AgentCollection, collect};

const SECRETS: [&str; 9] = [
    "fake-private-server",
    "123456789012",
    "AKIAFAKEFAKEFAKE0000",
    "hunter2",
    "sk-fake-0000",
    "fake-private-path",
    "fake-private-branch",
    "dev@example.com",
    "deadbeef",
];

fn privacy_folder(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("privacy")
        .join(name)
}

fn collect_fixture(agent: Agent) -> AgentCollection {
    let claude = privacy_folder("claude").into_os_string();
    let codex = privacy_folder("codex").into_os_string();
    let var = |name: &str| -> Option<OsString> {
        match name {
            "CLAUDE_CONFIG_DIR" => Some(claude.clone()),
            "CODEX_HOME" => Some(codex.clone()),
            _ => None,
        }
    };
    collect(agent, Path::new("no-home"), var)
}

fn assert_no_secret(agent: Agent) {
    let collection = collect_fixture(agent);
    assert!(
        !collection.sessions.is_empty(),
        "{} found no session",
        agent.name()
    );
    let tool_uses: u64 = collection
        .sessions
        .iter()
        .flat_map(|session| session.tool_uses.values())
        .sum();
    assert!(tool_uses > 0, "{} found no tool use", agent.name());

    let kept = format!("{collection:?}");
    for secret in SECRETS {
        assert!(!kept.contains(secret), "{} kept a secret", agent.name());
    }
}

#[test]
fn claude_code_keeps_no_secret() {
    assert_no_secret(Agent::ClaudeCode);
}

#[test]
fn codex_cli_keeps_no_secret() {
    assert_no_secret(Agent::CodexCli);
}

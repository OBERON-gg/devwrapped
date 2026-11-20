use std::io::Read;

use jiff::Timestamp;
use serde::Deserialize;

use super::{Agent, AgentScan, SessionBuilder, Tokens, model_name, tool_name};
use crate::lines::for_each_line;

// tool output can hold whole files, so lines get far more room than shell history lines
const MAX_LINE_LEN: usize = 16 * 1024 * 1024;

#[derive(Deserialize)]
struct Envelope {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    payload: Option<PayloadKind>,
}

#[derive(Deserialize)]
struct PayloadKind {
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Deserialize)]
struct Record<T> {
    payload: T,
}

#[derive(Deserialize)]
struct SessionMeta {
    id: Option<String>,
    timestamp: Option<String>,
}

#[derive(Deserialize)]
struct TurnContext {
    model: Option<String>,
}

#[derive(Deserialize)]
struct ToolCall {
    name: Option<String>,
}

#[derive(Deserialize)]
struct TokenCount {
    info: Option<TokenInfo>,
}

#[derive(Deserialize)]
struct TokenInfo {
    total_token_usage: Usage,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

/// each file holds a single session
pub fn parse(reader: impl Read) -> AgentScan {
    let mut builder = SessionBuilder::default();
    let mut session_id = None;
    let mut last_usage = None;
    let mut skipped_lines = 0;
    let result = for_each_line(reader, MAX_LINE_LEN, |line| {
        let Some(line) = line else {
            skipped_lines += 1;
            return;
        };
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        let Ok(envelope) = serde_json::from_slice::<Envelope>(line) else {
            skipped_lines += 1;
            return;
        };
        if let Some(time) = parse_time(envelope.timestamp) {
            builder.saw(time);
        }
        let payload_kind = envelope.payload.and_then(|payload| payload.kind);
        let read = match (envelope.kind.as_deref(), payload_kind.as_deref()) {
            (Some("session_meta"), _) => read::<SessionMeta>(line).map(|meta| {
                session_id = meta.id;
                if let Some(time) = parse_time(meta.timestamp) {
                    builder.saw(time);
                }
            }),
            (Some("turn_context"), _) => read::<TurnContext>(line).map(|turn| {
                if let Some(model) = turn.model {
                    *builder.models.entry(model_name(&model)).or_default() += 1;
                }
            }),
            (Some("response_item"), Some("function_call" | "custom_tool_call")) => {
                read::<ToolCall>(line).map(|call| {
                    if let Some(name) = call.name {
                        *builder.tool_uses.entry(tool_name(&name)).or_default() += 1;
                    }
                })
            }
            // the usage is a running total, so the last one covers the whole session
            (Some("event_msg"), Some("token_count")) => read::<TokenCount>(line).map(|count| {
                if let Some(info) = count.info {
                    last_usage = Some(info.total_token_usage);
                }
            }),
            _ => Some(()),
        };
        if read.is_none() {
            skipped_lines += 1;
        }
    });
    if result.is_err() {
        skipped_lines += 1;
    }

    if let Some(usage) = last_usage {
        builder.tokens = tokens(&usage);
    }
    let sessions = session_id
        .zip(builder.finish(Agent::CodexCli))
        .into_iter()
        .collect();
    AgentScan {
        sessions,
        skipped_lines,
    }
}

fn read<T: for<'de> Deserialize<'de>>(line: &[u8]) -> Option<T> {
    serde_json::from_slice::<Record<T>>(line)
        .ok()
        .map(|record| record.payload)
}

fn parse_time(time: Option<String>) -> Option<Timestamp> {
    time?.parse().ok()
}

// codex counts cached input inside input, while `Tokens` keeps the two apart
fn tokens(usage: &Usage) -> Tokens {
    let cached = usage.cached_input_tokens.unwrap_or(0);
    Tokens {
        input: usage.input_tokens.unwrap_or(0).saturating_sub(cached),
        output: usage.output_tokens.unwrap_or(0),
        cache_read: cached,
        cache_write: 0,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::path::Path;

    use super::*;

    fn fixture(name: &str) -> File {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("agents")
            .join(name);
        File::open(path).unwrap()
    }

    fn counts(pairs: &[(&str, u64)]) -> BTreeMap<String, u64> {
        pairs
            .iter()
            .map(|(name, count)| (name.to_string(), *count))
            .collect()
    }

    #[test]
    fn parses_a_session() {
        let scan = parse(fixture("codex_session.jsonl"));

        let session = &scan.sessions["c1"];
        assert_eq!(scan.sessions.len(), 1);
        assert_eq!(session.agent, Agent::CodexCli);
        assert_eq!(session.start, "2026-03-02T10:00:00Z".parse().unwrap());
        assert_eq!(
            session.tool_uses,
            counts(&[("apply_patch", 1), ("exec_command", 2)])
        );
        assert_eq!(
            session.models,
            counts(&[("gpt-5.5-codex", 1), ("gpt-5.5-codex-mini", 1)])
        );
        assert_eq!(
            session.tokens,
            Tokens {
                input: 1000,
                output: 120,
                cache_read: 2000,
                cache_write: 0,
            }
        );
        assert_eq!(scan.skipped_lines, 1);
    }

    #[test]
    fn keeps_no_text_paths_or_git_details() {
        let kept = format!("{:?}", parse(fixture("codex_session.jsonl")));

        for secret in [
            "hunter2",
            "AKIAFAKEFAKEFAKE0000",
            "fake-private-path",
            "fake-private-branch",
            "secret instructions",
        ] {
            assert!(!kept.contains(secret), "{secret}");
        }
    }

    #[test]
    fn drops_a_file_without_session_meta() {
        let input = br#"{"timestamp":"2026-03-02T10:00:00Z","type":"turn_context","payload":{"model":"gpt-5.5-codex"}}
"#;
        let scan = parse(&input[..]);

        assert!(scan.sessions.is_empty());
        assert_eq!(scan.skipped_lines, 0);
    }

    #[test]
    fn skips_records_of_the_wrong_shape() {
        let input =
            br#"{"timestamp":"2026-03-02T10:00:00Z","type":"session_meta","payload":{"id":"c1"}}
{"timestamp":"2026-03-02T10:00:01Z","type":"turn_context","payload":{"model":42}}
{"timestamp":"2026-03-02T10:00:02Z","type":"event_msg","payload":"not an object"}
"#;
        let scan = parse(&input[..]);

        assert_eq!(scan.sessions.len(), 1);
        assert!(scan.sessions["c1"].models.is_empty());
        assert_eq!(scan.skipped_lines, 2);
    }
}

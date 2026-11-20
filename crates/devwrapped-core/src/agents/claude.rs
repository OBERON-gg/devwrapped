use std::collections::{BTreeMap, HashSet};
use std::io::Read;

use jiff::Timestamp;
use serde::Deserialize;

use super::{Agent, AgentScan, SessionBuilder, Tokens, model_name, tool_name};
use crate::lines::for_each_line;

// tool results can hold whole files, so lines get far more room than shell history lines
const MAX_LINE_LEN: usize = 16 * 1024 * 1024;

#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Deserialize)]
struct AssistantRecord {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
    #[serde(default)]
    content: Vec<Block>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    name: Option<String>,
}

#[derive(Default)]
struct Session {
    builder: SessionBuilder,
    message_ids: HashSet<String>,
}

pub fn parse(reader: impl Read) -> AgentScan {
    let mut sessions: BTreeMap<String, Session> = BTreeMap::new();
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
        let Some(session_id) = envelope.session_id else {
            return;
        };
        let session = sessions.entry(session_id).or_default();
        if let Some(time) = envelope
            .timestamp
            .and_then(|time| time.parse::<Timestamp>().ok())
        {
            session.builder.saw(time);
        }
        if envelope.kind.as_deref() == Some("assistant") {
            match serde_json::from_slice::<AssistantRecord>(line) {
                Ok(record) => session.add(record.message),
                Err(_) => skipped_lines += 1,
            }
        }
    });
    if result.is_err() {
        skipped_lines += 1;
    }

    AgentScan {
        sessions: sessions
            .into_iter()
            .filter_map(|(id, session)| {
                let session = session.builder.finish(Agent::ClaudeCode)?;
                Some((id, session))
            })
            .collect(),
        skipped_lines,
    }
}

impl Session {
    fn add(&mut self, message: Message) {
        let builder = &mut self.builder;
        for block in message.content {
            if block.kind == "tool_use"
                && let Some(name) = block.name
            {
                *builder.tool_uses.entry(tool_name(&name)).or_default() += 1;
            }
        }

        // one message spans several lines that all repeat its usage, so count it once
        let first_time = message.id.is_none_or(|id| self.message_ids.insert(id));
        if !first_time {
            return;
        }
        if let Some(model) = message.model.filter(|model| !model.starts_with('<')) {
            *builder.models.entry(model_name(&model)).or_default() += 1;
        }
        if let Some(usage) = message.usage {
            builder.tokens += Tokens {
                input: usage.input_tokens.unwrap_or(0),
                output: usage.output_tokens.unwrap_or(0),
                cache_read: usage.cache_read_input_tokens.unwrap_or(0),
                cache_write: usage.cache_creation_input_tokens.unwrap_or(0),
            };
        }
    }
}

#[cfg(test)]
mod tests {
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
        let scan = parse(fixture("claude_session.jsonl"));

        let session = &scan.sessions["s1"];
        assert_eq!(scan.sessions.len(), 1);
        assert_eq!(session.agent, Agent::ClaudeCode);
        assert_eq!(session.start, "2026-03-02T08:59:59Z".parse().unwrap());
        assert_eq!(session.tool_uses, counts(&[("Bash", 2), ("Read", 1)]));
        assert_eq!(
            session.models,
            counts(&[("claude-opus-5-5", 1), ("claude-sonnet-5-5", 1)])
        );
        assert_eq!(
            session.tokens,
            Tokens {
                input: 11,
                output: 22,
                cache_read: 1050,
                cache_write: 100,
            }
        );
        assert_eq!(scan.skipped_lines, 1);
    }

    #[test]
    fn keeps_no_text_or_paths() {
        let kept = format!("{:?}", parse(fixture("claude_session.jsonl")));

        for secret in [
            "hunter2",
            "AKIAFAKEFAKEFAKE0000",
            "fake-private-path",
            "API error",
        ] {
            assert!(!kept.contains(secret), "{secret}");
        }
    }

    #[test]
    fn groups_records_by_session_id() {
        let input = br#"{"type":"user","sessionId":"a","timestamp":"2026-03-02T09:00:00Z"}
{"type":"user","sessionId":"b","timestamp":"2026-03-03T09:00:00Z"}
{"type":"assistant","sessionId":"b","timestamp":"2026-03-03T09:00:01Z","message":{"content":[{"type":"tool_use","name":"Edit"}]}}
"#;
        let scan = parse(&input[..]);

        assert_eq!(scan.sessions.len(), 2);
        assert_eq!(scan.sessions["b"].tool_uses, counts(&[("Edit", 1)]));
        assert!(scan.sessions["a"].tool_uses.is_empty());
    }

    #[test]
    fn drops_sessions_without_a_timestamp() {
        let input = br#"{"type":"user","sessionId":"a","timestamp":"not a time"}
{"type":"user","sessionId":"b"}
"#;
        let scan = parse(&input[..]);

        assert!(scan.sessions.is_empty());
        assert_eq!(scan.skipped_lines, 0);
    }

    #[test]
    fn caps_token_counts_that_would_overflow() {
        let input = br#"{"type":"assistant","sessionId":"a","timestamp":"2026-03-02T09:00:00Z","message":{"id":"m1","usage":{"input_tokens":18446744073709551615,"output_tokens":5}}}
{"type":"assistant","sessionId":"a","timestamp":"2026-03-02T09:00:01Z","message":{"id":"m2","usage":{"input_tokens":18446744073709551615,"output_tokens":5}}}
"#;
        let scan = parse(&input[..]);

        assert_eq!(scan.sessions["a"].tokens.input, u64::MAX);
        assert_eq!(scan.sessions["a"].tokens.output, 10);
    }

    #[test]
    fn skips_assistant_records_of_the_wrong_shape() {
        let input = br#"{"type":"assistant","sessionId":"a","timestamp":"2026-03-02T09:00:00Z","message":{"content":"plain text"}}
"#;
        let scan = parse(&input[..]);

        assert_eq!(scan.sessions.len(), 1);
        assert_eq!(scan.skipped_lines, 1);
    }
}

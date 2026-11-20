# Agent log formats

devwrapped reads the local logs of Claude Code and Codex CLI. This page lists the fields it reads. It keeps nothing else.

These notes describe logs written in November 2026. Both tools change their formats without notice. The parsers ignore unknown fields and skip records they cannot read.

## Claude Code

### Location

Claude Code writes one file per session:

```
~/.claude/projects/<project>/<session-id>.jsonl
```

`<project>` is the working folder with each path separator replaced by a dash. `<session-id>` is a UUID. On Windows the folder is `%USERPROFILE%\.claude\projects`. The `CLAUDE_CONFIG_DIR` environment variable moves the `.claude` folder.

The project folders also hold other files, such as Markdown notes and tool output. devwrapped reads only `.jsonl` files.

### Records

Each line is one JSON object. The `type` field names the kind of record. Common kinds are `assistant`, `user`, `attachment`, `system`, `api-request`, `mode`, `last-prompt`, `ai-title`, and `file-history-snapshot`.

devwrapped reads `assistant` records. From every other record it reads only `timestamp` and `sessionId`.

| Field | Type | Use |
| --- | --- | --- |
| `type` | string | Kind of record |
| `timestamp` | string, UTC in ISO 8601, such as `2026-11-14T18:30:00.000Z` | Session start, active days, busiest hour |
| `sessionId` | string | Session count |
| `message.id` | string | Counts each message once |
| `message.model` | string | Model name |
| `message.usage.input_tokens` | number | Token count |
| `message.usage.output_tokens` | number | Token count |
| `message.usage.cache_creation_input_tokens` | number | Token count |
| `message.usage.cache_read_input_tokens` | number | Token count |
| `message.content[].type` | string | `tool_use` marks a tool call |
| `message.content[].name` | string | Tool name, read only when the type is `tool_use` |

### Counting rules

- Claude Code splits one assistant message over several lines, with one content block per line. Every line repeats the same `message.usage`. Count tokens once per `message.id`.
- Each `tool_use` block has its own `id` and appears on one line only. Count tool uses per block.
- A session is one `sessionId`. Each file holds one session. The session starts at its earliest `timestamp`.

### Fields devwrapped never reads

These fields hold private text, file contents, or paths:

- `message.content[].text`, `message.content[].thinking`, `message.content[].input`, and `message.content[].content`: prompts, responses, tool input, and tool results
- `message.content` in `user` records: the prompt text
- `toolUseResult`: tool output, which can include whole files
- `cwd`, `gitBranch`, and `attachment`: paths, branch names, and context
- `snapshot.trackedFileBackups`: its keys are file paths
- `lastPrompt`, `aiTitle`, `accountUuid`, and `organizationUuid`

## Codex CLI

### Location

Codex CLI writes one file per session:

```
~/.codex/sessions/<year>/<month>/<day>/rollout-<date>T<time>-<session-id>.jsonl
```

`<session-id>` is a UUID. On Windows the folder is `%USERPROFILE%\.codex\sessions`. The `CODEX_HOME` environment variable moves the `.codex` folder.

The `.codex` folder also holds settings, SQLite databases, plugins, `session_index.jsonl`, and `auth.json`. devwrapped reads only the `.jsonl` files under `sessions`.

### Records

Each line is one JSON object with 3 fields: `timestamp`, `type`, and `payload`. The `type` field is `session_meta`, `turn_context`, `response_item`, or `event_msg`. Inside `response_item` and `event_msg` records, `payload.type` names the kind of item, such as `message`, `reasoning`, `function_call`, `custom_tool_call`, `token_count`, `user_message`, or `task_complete`.

| Record | Field | Type | Use |
| --- | --- | --- | --- |
| every record | `timestamp` | string, UTC in ISO 8601, such as `2026-11-14T18:30:00.000Z` | Active days, busiest hour |
| every record | `type` | string | Kind of record |
| `session_meta` | `payload.id` | string | Session count |
| `session_meta` | `payload.timestamp` | string, same format | Session start |
| `turn_context` | `payload.model` | string | Model name, once per turn |
| `response_item` | `payload.type` | string | `function_call` and `custom_tool_call` mark a tool call |
| `response_item` | `payload.name` | string | Tool name, read only for tool calls |
| `event_msg` | `payload.type` | string | `token_count` marks a token update |
| `event_msg` | `payload.info.total_token_usage.input_tokens` | number | Token count |
| `event_msg` | `payload.info.total_token_usage.cached_input_tokens` | number | Token count |
| `event_msg` | `payload.info.total_token_usage.output_tokens` | number | Token count |

### Counting rules

- `total_token_usage` is a running total for the session and only grows. Take the last one in each session. Do not add up `last_token_usage`, because Codex sometimes repeats a token event.
- `payload.info` can be `null` in a `token_count` event. Skip that event.
- `input_tokens` includes `cached_input_tokens`, and `output_tokens` includes `reasoning_output_tokens`. `total_tokens` is `input_tokens` plus `output_tokens`.
- Each tool call is one `response_item` with its own `call_id`. Count tool uses per record.
- A session is one file with one `session_meta` record. The session starts at `payload.timestamp` of that record.
- The model can change between turns. Count models per `turn_context` record.

### Fields devwrapped never reads

These fields hold private text, file contents, commands, or paths:

- `payload.content`, `payload.text`, `payload.message`, `payload.summary`, `payload.encrypted_content`, and `payload.last_agent_message`: prompts, responses, and reasoning
- `payload.arguments`, `payload.input`, `payload.output`, `payload.command`, `payload.parsed_cmd`, `payload.stdout`, `payload.stderr`, `payload.aggregated_output`, and `payload.formatted_output`: tool input and output
- `payload.changes`: diffs, keyed by file path
- `payload.cwd` and `payload.git`: paths, branch names, commit hashes, and repository URLs
- `payload.instructions` and `payload.collaboration_mode`: instructions
- `session_index.jsonl`, which holds thread names, and `auth.json`, which holds credentials

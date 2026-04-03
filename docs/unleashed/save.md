# Saving transcripts (`/save`)

`/save` exports the current session transcript to disk so you can archive, review, or build
datasets from Codex runs.

## Quick start

In the TUI, type:

```text
/save
```

This writes a compact Markdown transcript to `codex-<timestamp>.md` in your current working
directory.

## Compact vs full

By default, `/save` writes a **compact** export that focuses on the conversation narrative:

- user messages
- assistant messages
- plan updates
- reasoning summaries

To include tool calls/results, patches, and other non-chat events, use:

```text
/save --full
```

## Filenames

- `/save` → `codex-<timestamp>.md`
- `/save notes` → `notes.md`

Extensions are added automatically (`.md` / `.jsonl`) if missing.

## JSONL for training datasets

### SFT (supervised fine-tuning)

```text
/save --sft-jsonl
```

Emits a single JSONL record with an OpenAI/TRL-style `messages[]` schema:

```jsonl
{"id":"<thread-id>","createdAt":1712170000,"messages":[{"role":"user","content":"..."},{"role":"assistant","content":"..."}],"metadata":{"savedAt":"2026-04-03T22:13:43+02:00","codexVersion":"0.87.0","cwd":"/path","threadId":"...","model":"gpt-5.2","reasoningEffort":"xhigh"}}
```

### CPT (continued pretraining)

```text
/save --cpt-jsonl
```

Emits a single JSONL record with a plain `text` field:

```jsonl
{"id":"<thread-id>","createdAt":1712170000,"text":"User:\\n...\\n\\nAssistant:\\n...","metadata":{...}}
```

## Safety

Exports may contain sensitive data (file contents, commands, environment variables).
On Unix, transcript files are written with `0600` permissions.

## Implementation notes (this fork)

- Transcript export logic: `codex-rs/tui/src/save_transcript.rs`.
- Slash command wiring: `codex-rs/tui/src/app.rs`, `codex-rs/tui/src/chatwidget.rs`.


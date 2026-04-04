# Context: What Codex Sends (and Why)

This document explains **exactly what gets included in Codex’s next model request**, what gets filtered out, and where truncation/compaction happens.

It is written against this fork’s current baseline (upstream Codex CLI `v0.87.x` lineage). Nothing in this document assumes prior knowledge of the codebase.

> **Key idea:** Codex maintains an in-memory “history” (`Vec<ResponseItem>`) that is used as the **input** to the next model request. The UI shows many things, but only some of them are stored as `ResponseItem`s and therefore eligible to be sent back to the model.

---

## Terminology (mapped to the wire request)

Codex sends two different “channels” of information to the model:

1. **`instructions`** (a single string)
   - This is the *base/system prompt* for the selected model.
   - It is **not** stored in the conversation history.

2. **`input`** (a list of items)
   - This is a `Vec<ResponseItem>` built from the session history.
   - This is the **conversation context** in the strict sense: messages, tool calls, tool outputs, reasoning items, etc.

Additionally, Codex sends:

3. **`tools`** (a list of tool schemas)
4. **Reasoning configuration** (effort/summary settings) for reasoning-capable models

You can see the shaping of the request in:
- `codex-rs/core/src/client.rs:609` (`build_api_prompt`: `instructions` + `input` + `tools`)
- `codex-rs/core/src/codex.rs:2624` (how `input` is currently sourced from history)

---

## The “History” that becomes `input`

### Where history lives

History is stored in `ContextManager`:
- `codex-rs/core/src/context_manager/history.rs:21`

It is an ordered list:
- Oldest items at the beginning
- Newest items at the end

### What gets recorded into history (the hard filter)

When Codex records items, it **does not store everything**. It filters `ResponseItem`s through `is_api_message` and only then appends them:
- `codex-rs/core/src/context_manager/history.rs:53` (`record_items`)
- `codex-rs/core/src/context_manager/history.rs:300` (`is_api_message`)

**Included (`is_api_message == true`)**

These item types are recorded and can be sent back in the next prompt:

- `ResponseItem::Message` for any role except `"system"`  
  (this includes `"user"`, `"assistant"`, and `"developer"`)
- `ResponseItem::FunctionCall`
- `ResponseItem::FunctionCallOutput`
- `ResponseItem::CustomToolCall`
- `ResponseItem::CustomToolCallOutput`
- `ResponseItem::LocalShellCall`
- `ResponseItem::WebSearchCall`
- `ResponseItem::Reasoning` ✅ **Reasoning is included**
- `ResponseItem::Compaction` ✅ **Compaction metadata is included**

**Excluded (`is_api_message == false`)**

These item types are *not* recorded in the in-memory history:

- `ResponseItem::Other`
- Any message with `role == "system"` (system messages are ignored for history)

**Special case: Ghost snapshots**

`ResponseItem::GhostSnapshot` is recorded even though it is not an API message, but it is explicitly removed before sending:
- Recorded because `record_items` whitelists it (`history.rs:60`)
- Dropped from the model prompt in `for_prompt` (`history.rs:72`)

---

## What is sent in the next request

The next model request uses:

1. A **clone** of the current in-memory history
2. A call to `for_prompt()` which:
   - Normalizes call/output pairing invariants
   - Removes ghost snapshots

Code path:
- Build `input` right before sampling:
  - `codex-rs/core/src/codex.rs:2624` (`sess.clone_history().await.for_prompt()`)
- Normalize + drop ghosts:
  - `codex-rs/core/src/context_manager/history.rs:72` (`for_prompt`)

### Normalization (call/output invariants)

Before sending, history is normalized:
- `codex-rs/core/src/context_manager/history.rs:249` (`normalize_history`)

Normalization does two things:
1. Ensures every call has an output (inserts synthetic `"aborted"` outputs if missing)
   - `codex-rs/core/src/context_manager/normalize.rs:7`
2. Removes orphan outputs that have no corresponding call
   - `codex-rs/core/src/context_manager/normalize.rs:78`

This matters because the model expects tool call/output pairs to be consistent.

---

## Truncation: what gets shortened before it reaches the model

Truncation happens when items are recorded into history (not at send-time).

In `process_item`, Codex truncates:
- `ResponseItem::FunctionCallOutput` (text content and content-items)
- `ResponseItem::CustomToolCallOutput`

Code:
- `codex-rs/core/src/context_manager/history.rs:257`

Everything else is stored as-is:
- Messages (`user`/`assistant`/`developer`)
- Tool calls (the call itself)
- Reasoning items
- Compaction items

> Practical impact: the UI or rollout file may contain **larger raw outputs**, but the prompt history uses a **truncated** version for some tool outputs.

---

## Compaction: how the history can be rewritten

Compaction is an explicit operation (`/compact`) and can also be triggered automatically when nearing a model-specific token threshold.

When compaction runs, Codex replaces the in-memory history with a compacted version:
- `codex-rs/core/src/compact.rs:168` → `sess.replace_history(new_history).await;`

Remote compaction (when enabled) similarly replaces history:
- `codex-rs/core/src/compact_remote.rs:45`

Important: compaction changes what will be sent in future prompts because it **replaces the history** (it does not merely “summarize a view”).

---

## Reasoning: is it part of the next prompt?

Yes.

`ResponseItem::Reasoning` is:
- recorded (`is_api_message` returns true)  
  `codex-rs/core/src/context_manager/history.rs:300`
- preserved by `for_prompt()` (it is not filtered out)

In addition, for reasoning-capable models, Codex requests encrypted reasoning payloads from the Responses API:
- `codex-rs/core/src/client.rs:293` sets `include = ["reasoning.encrypted_content"]` when reasoning is enabled.

That means the model’s prior reasoning items can appear in history and can be re-sent as part of `input` in subsequent requests.

> Note: the UI setting “show/hide raw reasoning” affects how some providers stream/aggregate output, but it does not retroactively remove `ResponseItem::Reasoning` items from the history once they are recorded.

---

## What you see in the TUI vs. what is in `input`

The TUI renders a mixture of:
- turn items derived from `ResponseItem`s (user messages, assistant messages, reasoning)
- lifecycle and tool execution events (`EventMsg::*`) that are **not** stored in history

Only `ResponseItem`s recorded into `ContextManager` become `Prompt.input`.

The mapping from `ResponseItem` → UI “turn items” is intentionally selective:
- `"developer"` messages are not shown as user turns
- environment context XML is not shown as a user turn
- AGENTS.md/skill injections are hidden from the visible “chat turns”

See:
- `codex-rs/core/src/event_mapping.rs:95` (`parse_turn_item`)

This is why the “chat window” is not a reliable representation of the raw prompt context by itself.

---

## Diagram: from conversation to next prompt

```text
┌────────────────────────────────────────────────────────────────┐
│ 1) User types / UI actions / model stream / tool execution      │
└───────────────┬────────────────────────────────────────────────┘
                │ produces ResponseItem(s) and EventMsg(s)
                │
                ▼
┌────────────────────────────────────────────────────────────────┐
│ 2) record_conversation_items()                                  │
│    - persists rollout (raw ResponseItem)                        │
│    - emits RawResponseItem event                                │
│    - records into ContextManager (FILTER + TRUNCATION)          │
└───────────────┬────────────────────────────────────────────────┘
                │
                ▼
┌────────────────────────────────────────────────────────────────┐
│ 3) ContextManager history (processed ResponseItem list)         │
│    - only is_api_message items (plus GhostSnapshot)             │
│    - tool outputs may be truncated                              │
└───────────────┬────────────────────────────────────────────────┘
                │
                ▼
┌────────────────────────────────────────────────────────────────┐
│ 4) Before each sampling request                                 │
│    input = clone_history().for_prompt()                         │
│    - normalize call/output invariants                           │
│    - drop GhostSnapshot                                          │
└───────────────┬────────────────────────────────────────────────┘
                │
                ▼
┌────────────────────────────────────────────────────────────────┐
│ 5) API request payload                                           │
│    instructions: (base instructions string)                      │
│    input: Vec<ResponseItem>  ← THIS IS "CONTEXT"                 │
│    tools: tool schemas                                           │
│    reasoning: effort/summary + include reasoning.encrypted_content│
└────────────────────────────────────────────────────────────────┘
```

---

## Unleashed feature: “Context Blocks” (goal + constraints)

This fork adds a `/context` dashboard so you can see (and later selectively include/exclude) *logical blocks* of the next prompt.

**Design constraints**
- **Baseline behavior must remain unchanged**: when all blocks are enabled, the next request must match upstream behavior.
- **No deletion**: blocks should be toggleable on/off, not deleted.
- **Safe invariants**: blocks should be defined so that tool call/output pairs remain consistent.

### Using `/context`

`/context` opens a full-screen dashboard (similar to `/agents`) that shows:
- An estimated total token count for the **next** request (`instructions + tools + input`)
- A list of **context blocks** with ids, token estimates, and one-line descriptions
- A color-coded bar per block to visualize relative size

Commands:

- `/context`  
  Show the overview dashboard.
- `/context <block-id>`  
  Show full details for a single block (the exact `ResponseItem`s).
- `/context enable <block-id>`  
  Re-include a previously disabled block in future requests.
- `/context disable <block-id>`  
  Exclude a block from future requests (does not delete anything).

Notes:
- Disabling blocks only affects **future** model requests. It never deletes history or rollouts.
- Some blocks are **required** and cannot be disabled.
- If you compact (`/compact`), the in-memory history is still rewritten as in upstream Codex.

### Block ids and meanings

`/context` shows synthetic blocks first, then history-derived blocks:

- `instructions` (required)  
  The model’s base/system prompt string for this turn.
- `tools` (required)  
  The JSON schema for available tools (including MCP tools when configured).

History-derived blocks (from `ContextManager` history):

- `setup` (required)  
  Session bootstrap context injected by Codex (developer permissions/instructions, environment context, etc.).
- `update:<n>` (required)  
  A “context update” inserted between turns (e.g., changed working directory, environment context refresh, permission updates).
- `turn:<n>` (toggleable)  
  A single user turn plus all following items until the next turn boundary.
- `misc:<idx>` (required)  
  Context items that are not part of a normal user turn (rare, but preserved to avoid breaking invariants).

### What “details” show

`/context <block-id>` renders the raw `ResponseItem`s in that block, including:
- `message` items (user/assistant/developer)
- tool call/output items
- reasoning items (`ResponseItem::Reasoning`, encrypted)
- compaction metadata

This is the closest you can get to “what is actually sent to the model” without capturing the raw HTTP request.

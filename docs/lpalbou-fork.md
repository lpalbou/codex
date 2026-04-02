# `lpalbou/codex` fork notes

## Philosophy: depth over speed

This fork is for a specific preference:

- One deep, careful agentic run that can take **1 hour or more** is better than 10 fast runs that
  require supervision, debugging, and repeated fixes.
- Longer runs make the system easier to observe while it’s working, and reduce the total volume of
  trial-and-error text produced across multiple shallow attempts.

## Goal: predictable model routing + effort

The main goal is to make it easy to run Codex with a single model + reasoning effort (for example
`gpt-5.2` + `model_reasoning_effort = "xhigh"`) across:

- the main thread
- spawned sub-agents
- compaction (`/compact` and auto-compaction)

## Notes on model/effort consistency (v0.87)

- In upstream `rust-v0.87.0`, `agent_type=worker` sub-agents override the model to `gpt-5.2-codex`.
  This fork adds a feature flag `worker_model_override` (default: `false`) so workers can inherit
  the parent model selection.
- `/compact` and auto-compaction may use the remote compact endpoint when `remote_compaction` is
  enabled. In `v0.87`, that endpoint does **not** accept `reasoning.effort`, so the only clean way
  to ensure effort is honored for compaction is to disable that feature:
  `codex --disable remote_compaction`.

## Sub-agent observability (v0.87)

Codex v0.87 does not provide a dedicated “agent dashboard” in the TUI, but you can still observe
sub-agents in a few practical ways:

- **In the TUI chat history:** collab events (`spawn_agent`, `send_input`, `wait`, `close_agent`)
  include agent IDs and status snapshots.
- **Inspect persisted sessions:** every spawned agent has its own rollout file under
  `$CODEX_HOME/sessions/.../rollout-...-<agent_id>.jsonl`.
  - Open the rollout directly (for example with `jq -C . <file>`), or
  - Resume the session by ID: `codex resume <agent_id>`.
- **Logs:** the TUI writes logs to `$CODEX_HOME/log/codex-tui.log`. You can follow it with:
  `tail -F $CODEX_HOME/log/codex-tui.log` and increase verbosity via `RUST_LOG`.

## Max sub-agents (v0.87)

There is no built-in hard limit on the number of spawned agents in `v0.87`. The main control is
prompt discipline (re-use agents; close them when done). A hard cap would require a source patch
to enforce a maximum in `spawn_agent`.

## Context window / token usage for sub-agents (v0.87)

- The TUI shows token usage for the currently viewed session only.
- Rollouts include `token_count` events; the payload includes both current usage and (when known)
  the model context window.

## Compaction strategy for sub-agents (v0.87)

Each agent thread is a full session and follows the same compaction rules as the main thread:

- Auto-compaction triggers when token usage reaches the model’s auto-compaction threshold.
- With `remote_compaction` enabled (and an OpenAI provider), compaction uses the Compact endpoint.
- Otherwise, compaction is performed “locally” by running a dedicated compaction turn via the
  normal model request path (and will honor `model_reasoning_effort`).


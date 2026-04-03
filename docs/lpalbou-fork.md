# `lpalbou/codex` fork notes

## Philosophy: depth over speed

This fork is for a specific preference:

- One deep, careful agentic run that can take **1 hour or more** is better than 10 fast runs that
  require supervision, debugging, and repeated fixes.
- Longer runs make the system easier to observe while it's working, and reduce the total volume of
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
- Spawned agents start with **no** conversation history. The only way to include prior discussion
  is to explicitly include it in the `spawn_agent` prompt message.
- `/compact` and auto-compaction may use the remote compact endpoint when `remote_compaction` is
  enabled. In `v0.87`, that endpoint does **not** accept `reasoning.effort`, so the only clean way
  to ensure effort is honored for compaction is to disable that feature:
  `codex --disable remote_compaction`.

## Sub-agent observability (v0.87)

This fork adds a dedicated live "agent dashboard" in the TUI:

- Run `/agents` to open a real-time dashboard of spawned sub-agents (press Esc to close).
- The dashboard summarizes each agent's status, last action (tools + collab ops), approvals, model
  selection, and context-window usage (current/max when available).
- Note: this dashboard shows **real** spawned sub-agents (threads). To enable spawning, you must
  enable the `collab` feature (it's disabled by default in upstream v0.87): `codex --enable collab`.

You can also observe sub-agents in a few other practical ways:

- **In the TUI chat history:** collab events (`spawn_agent`, `send_input`, `wait`, `close_agent`)
  include agent IDs and status snapshots.
- **Inspect persisted sessions:** every spawned agent has its own rollout file under
  `$CODEX_HOME/sessions/.../rollout-...-<agent_id>.jsonl`.
  - Open the rollout directly (for example with `jq -C . <file>`), or
  - Resume the session by ID: `codex resume <agent_id>`.
- **Logs:** the TUI writes logs to `$CODEX_HOME/log/codex-tui.log`. You can follow it with:
  `tail -F $CODEX_HOME/log/codex-tui.log` and increase verbosity via `RUST_LOG`.

## Local install (side-by-side)

To keep your current `codex` and also have a second binary (for example `codex-best`) pointing at
this fork:

```bash
# From the repo root:
cd codex-rs

# Build a release binary.
cargo build -p codex-cli --release

# Install a side-by-side alias.
mkdir -p ~/.local/bin
ln -sf "$(pwd)/target/release/codex" ~/.local/bin/codex-best

# Ensure ~/.local/bin is on PATH (example for zsh: add this line to ~/.zshrc, then restart shell):
# export PATH="$HOME/.local/bin:$PATH"

# Verify.
codex-best --version
```

## Quick smoke test: `/agents`

1. Run `codex-best --enable collab` to open the TUI with sub-agent tools enabled.
2. In the composer, send a prompt that forces sub-agent usage, for example:
   "Use the `spawn_agent` tool (do not simulate) to create 2 sub-agents. Agent 1 runs `ls` and reports back. Agent 2 runs `grep -n \"Collab\" codex-rs/core/src/tools/handlers/collab.rs` and reports back. Then wait for them and close both agents."
3. While Codex is working, type `/agents` and press Enter.
4. Confirm you see agents appear, their status changes, last actions update, and context usage is
   shown when available.
5. Press Esc to close the dashboard.

## Max sub-agents (v0.87)

There is no built-in hard limit on the number of spawned agents in `v0.87`. The main control is
prompt discipline (re-use agents; close them when done). A hard cap would require a source patch
to enforce a maximum in `spawn_agent`.

## Context window / token usage for sub-agents (v0.87)

- The TUI shows token usage for the currently viewed session only.
- Rollouts include `token_count` events; the payload includes both current usage and (when known)
  the model context window.
- `gpt-5.2` advertises a 400K total context window in the API docs, but Codex reports an **effective
  input window** (what it can safely keep in history) after reserving headroom for system/tool
  overhead and model output.
  - In `v0.87`, local/remote model metadata uses `context_window = 272_000` with
    `effective_context_window_percent = 95`, which yields `272_000 * 0.95 = 258_400` usable input
    tokens (what `/status` shows as the context window).
  - You can override the input window and compaction thresholds in `~/.codex/config.toml` via
    `model_context_window` and `model_auto_compact_token_limit`, but setting values above what the
    backend supports will eventually trigger context-window errors.

## Compaction strategy for sub-agents (v0.87)

Each agent thread is a full session and follows the same compaction rules as the main thread:

- Auto-compaction triggers when token usage reaches the model's auto-compaction threshold.
- With `remote_compaction` enabled (and an OpenAI provider), compaction uses the Compact endpoint.
- Otherwise, compaction is performed "locally" by running a dedicated compaction turn via the
  normal model request path (and will honor `model_reasoning_effort`).

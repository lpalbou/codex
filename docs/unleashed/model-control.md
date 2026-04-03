# Model & reasoning control

This fork aims for **predictable model selection** and **consistent reasoning effort** across:

- the main thread,
- spawned sub-agents, and
- compaction (`/compact` and auto-compaction).

## Defaults (fresh install)

With no config overrides:

- Default model: `gpt-5.2`
- Default reasoning effort: `xhigh` (for Responses-based providers and `gpt-5.2`)

You can always override the model and/or provider explicitly (see below).

## Change the model

### One run (CLI)

```sh
codex-unleashed --model gpt-5.2
```

### Persist it (config)

Edit `~/.codex-unleashed/config.toml`:

```toml
model = "gpt-5.2"
model_reasoning_effort = "xhigh"
```

### During a session (TUI)

Use `/model` to switch models interactively.

## Ensure sub-agents inherit the same model

Sub-agents inherit the parent session configuration (provider/model/effort, sandbox, CWD, and user
instructions). The only thing they do **not** automatically inherit is the full conversation
history.

### Worker model overrides

Codex supports a special “worker” role that can optionally override the model to `gpt-5.2-codex`.
In `rust-v0.87.0` this override is gated behind an experimental feature flag:
`worker_model_override` (default: **disabled**).

Recommendation: keep `worker_model_override` disabled if you want **one model across all tasks**.

Inspect feature state:

```sh
codex-unleashed features list
```

## Compaction and reasoning effort

Some providers/routes may not support `reasoning.effort` for compaction requests.
If you care about compaction quality, prefer local compaction turns (or disable remote compaction
when it is not honoring effort).

## Implementation notes (upstream + fork)

- Default effort selection: `codex-rs/core/src/codex.rs` (sets `xhigh` for `gpt-5.2` on Responses).
- Worker override gate: `codex-rs/core/src/agent/role.rs` (`worker_model_override`).
- Provider/model CLI overrides: `codex-rs/tui/src/cli.rs`, `codex-rs/tui/src/lib.rs`.


# Model & reasoning control

This fork aims for **predictable model selection** and **consistent reasoning effort** across:

- the main thread,
- spawned sub-agents, and
- compaction (`/compact` and auto-compaction).

It also adds explicit **agent limit controls** and a launch-time **OpenAI base URL override** so
you can keep the same reasoning policy while changing routing or concurrency.

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

You can combine model selection with agent controls:

```sh
codex-unleashed --model gpt-5.2 --max-threads -1 --max-depth -1
```

- `--max-threads -1`: unlimited spawned-agent threads
- `--max-threads 0`: disable spawned agents entirely
- `--max-depth -1`: unlimited spawn depth
- `--max-depth 0`: root agent cannot spawn children

### Persist it (config)

Edit `~/.codex-unleashed/config.toml`:

```toml
model = "gpt-5.2"
model_reasoning_effort = "xhigh"

[agents]
max_threads = 24
max_depth = 4
```

Omit the `[agents]` keys entirely if you want the original `0.87` “unlimited unless otherwise
constrained” behavior.

### During a session (TUI)

Use `/model` to switch models interactively.

Use `/max-threads` and `/max-depth` to inspect or change the current session limits:

```text
/max-threads
/max-threads 12
/max-threads -1
/max-depth
/max-depth 3
/max-depth -1
```

These commands restart the underlying agent session so the new limits are applied cleanly to the
active thread and all future spawned children.

## Launch-time base URL override

Use `--base-url` when you want to keep the built-in `openai` provider but route it to a gateway,
proxy, or compatible endpoint for that run:

```sh
codex-unleashed --model gpt-5.2 --base-url http://127.0.0.1:8099/v1
```

This is **launch-only** by design. There is no in-session `/base-url` command.

- With the built-in `openai` provider, `--base-url` overrides the provider base URL for that run.
- With OSS providers (`lmstudio`, `ollama`, `ollama-chat`), `--base-url` is redirected to the OSS
  bootstrap path instead.

## Ensure sub-agents inherit the same model

Sub-agents inherit the parent session configuration (provider/model/effort, sandbox, CWD, and user
instructions). The only thing they do **not** automatically inherit is the full conversation
history.

In this fork, spawned thread children now also inherit:

- the parent shell snapshot (when shell snapshots are enabled)
- shared execpolicy state when the same config roots apply

So the practical baseline is:

- same provider
- same model
- same reasoning effort
- same shell/approval/sandbox runtime policy

The remaining responsibility is on the spawn prompt: it must carry the task-specific context the
child needs, because the whole parent conversation is still not copied by default.

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

## Turn-state sticky routing

This fork also backports **turn-state sticky routing** for Responses traffic.

In practice, when the backend returns an `x-codex-turn-state` header, Codex stores it and replays
it on the follow-up requests that belong to the same turn. This is transport-level continuity: it
helps keep a long tool-using turn pinned to the same backend route, but it does **not** change the
prompt, the model, or the orchestration policy.

There is no user-facing command for this feature. It is automatic.

## Implementation notes (upstream + fork)

- Default effort selection: `codex-rs/core/src/codex.rs` (sets `xhigh` for `gpt-5.2` on Responses).
- Worker override gate: `codex-rs/core/src/agent/role.rs` (`worker_model_override`).
- Provider/model CLI overrides: `codex-rs/tui/src/cli.rs`, `codex-rs/tui/src/lib.rs`.
- Agent limit config and persistence: `codex-rs/core/src/config/mod.rs`,
  `codex-rs/core/src/config/edit.rs`.
- Spawn guard enforcement: `codex-rs/core/src/agent/control.rs`,
  `codex-rs/core/src/agent/guards.rs`.
- Child-depth tracking and runtime override inheritance:
  `codex-rs/core/src/tools/handlers/collab.rs`.
- Turn-state sticky routing: `codex-rs/core/src/client.rs`,
  `codex-rs/codex-api/src/endpoint/responses.rs`,
  `codex-rs/codex-api/src/sse/responses.rs`,
  `codex-rs/codex-api/src/endpoint/responses_websocket.rs`.

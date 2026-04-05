# Sub-agent observability (`/agents`)

This fork ships with a dedicated **real-time dashboard** for spawned sub-agents.

## Open the dashboard

In the TUI, type:

```text
/agents
```

Press **Esc** to close the overlay.

## Important: “real” sub-agents require `collab`

`/agents` shows **actual spawned threads**. For Codex to be able to spawn them, the experimental
feature flag `collab` must be enabled.

Enable it for a run:

```sh
codex-unleashed --enable collab
```

Or toggle it inside the running TUI (session-scoped):

```text
/collab on
```

```text
/collab off
```

```text
/collab status
```

Note: `/collab` restarts the underlying agent session so the toolset is rebuilt with the updated
feature state. It cannot be used while a task is running. To persist collab across restarts, use
`/experimental`.

Then ask Codex to use the `spawn_agent` tool explicitly (not to simulate).

## Control how many agents can spawn

This fork adds explicit thread and depth limits.

### Launch-time flags

```sh
codex-unleashed --enable collab --max-threads -1 --max-depth -1
```

- `--max-threads -1`: unlimited concurrent spawned-agent threads
- `--max-threads 0`: disable spawned agents
- `--max-depth -1`: unlimited spawn depth
- `--max-depth 0`: root agent cannot spawn children

### In-session controls

```text
/max-threads
/max-threads 8
/max-threads -1
/max-depth
/max-depth 2
/max-depth -1
```

These commands are session-scoped and restart the active agent so the new limits take effect
cleanly.

### Persistent config

```toml
[agents]
max_threads = 8
max_depth = 2
```

## What you can see

For each spawned agent, the dashboard shows:

- agent id + parent id
- status (pending/running/done)
- model + reasoning effort (when available)
- context usage (current/max when the backend provides it)
- task summary (“doing”)
- last user message / last assistant output (helpful for debugging agent drift)
- rollout file path (persisted history on disk)

## What the model can now see

This fork also adds a model-facing `list_agents` tool.

The main agent can call it to recover the live spawned-agent inventory for the current shared
agent tree. The tool returns:

- `current_thread_id`
- `agents[]`
  - `agent_id`
  - `parent_id`
  - `depth`
  - `agent_type`
  - `agent_status`
  - `last_task_message`

This matters because the orchestrator no longer has to rely only on remembered agent ids. If it
loses track of a child after a long turn, it can call `list_agents`, then decide whether to
`wait`, `send_input`, or `close_agent`.

Important: `list_agents` is a **registry view**, not a live self-written progress report from each
child.

- `agent_id`, `parent_id`, `depth`, `agent_type`, and `agent_status` come from the runtime agent
  registry.
- `last_task_message` is the last task prompt the parent sent to that child.
- It does **not** automatically contain a fresh child-authored summary of intermediate progress.

So the flow is:

1. `list_agents` tells the orchestrator **who exists and what task they were last assigned**.
2. The parent can then `wait` for a result, `send_input` to ask for an update, or `close_agent`.

This is intentional: it keeps the tool small, explicit, and robust for the `0.87` fork.

## What children now inherit

Thread-spawn sub-agents now inherit the same operational baseline as the parent:

- provider / model / reasoning effort
- current working directory
- sandbox and approval policy
- shell-environment policy
- parent shell snapshot (when shell snapshots are enabled)
- shared execpolicy state when the same config-layer roots apply

They do **not** automatically inherit the full conversation history. The spawn prompt still needs
to be self-contained for the specific subtask.

## Practical workflow tips

- Ask the main agent to **reuse** agents instead of spawning new ones for every subtask.
- Ask it to **close** agents when they’re done to reduce noise.
- Ask it to use **long waits** for substantive work. In this fork, `wait` now defaults to 5
  minutes and accepts up to 1 hour.
- If an agent seems stuck, you can follow its rollout file under `$CODEX_HOME/sessions/...`.
- If you want the original `0.87` behavior, leave the agent limits unset (or use `-1` in the CLI).

## Child completion notifications

This fork also hardens spawned-agent orchestration:

- spawned agents now carry explicit **parent-thread provenance**
- when a spawned agent reaches a final status, Codex injects a structured
  `<subagent_notification>...</subagent_notification>` message into the parent thread history
- this means the parent can still see the child result on later turns even if it forgot to call
  `wait` at exactly the right moment

This does **not** replace `wait` when the parent needs the result immediately during the same
workflow. It makes the overall system more robust and less brittle when the parent continues before
harvesting every child result perfectly.

## Shared approvals across sub-agents

Spawned thread-tree children now reuse the parent thread’s **execpolicy manager** when they were
spawned from the same config-layer roots.

In practice, this means:

- if you persist an allow-prefix rule from the parent thread,
- a spawned child using the same Codex config roots can reuse that same rule state,
- so you avoid unnecessary re-approval churn during long multi-agent runs.

This is intentionally conservative: the sharing only happens when the child is effectively using
the same execpolicy configuration base.

## Reused shell snapshots for thread-spawn children

When shell snapshots are enabled, thread-spawn children now reuse the **parent shell snapshot**
instead of creating a fresh one.

This improves spawned-agent fidelity for shell-driven tasks because the child sees the same captured
shell setup as the parent:

- exported environment variables
- aliases
- shell functions
- shell options

It also avoids unnecessary snapshot churn and keeps parent/child shell behavior aligned.

## Implementation notes (upstream + fork)

- Collab tool + spawn plumbing: `codex-rs/core/src/tools/handlers/collab.rs`.
- Thread reservation guard: `codex-rs/core/src/agent/guards.rs`.
- Runtime spawn control: `codex-rs/core/src/agent/control.rs`.
- Execpolicy sharing: `codex-rs/core/src/exec_policy.rs`.
- Session-prefix classification: `codex-rs/core/src/session_prefix.rs`.
- Shell snapshots: `codex-rs/core/src/shell_snapshot.rs`.
- Dashboard UI: `codex-rs/tui/src/agents_dashboard`.

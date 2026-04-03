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

## What you can see

For each spawned agent, the dashboard shows:

- agent id + parent id
- status (pending/running/done)
- model + reasoning effort (when available)
- context usage (current/max when the backend provides it)
- task summary (“doing”)
- last user message / last assistant output (helpful for debugging agent drift)
- rollout file path (persisted history on disk)

## Practical workflow tips

- Ask the main agent to **reuse** agents instead of spawning new ones for every subtask.
- Ask it to **close** agents when they’re done to reduce noise.
- If an agent seems stuck, you can follow its rollout file under `$CODEX_HOME/sessions/...`.

## Implementation notes (upstream + fork)

- Collab tool + spawn plumbing: `codex-rs/core/src/tools/handlers/collab.rs`.
- Dashboard UI: `codex-rs/tui/src/agents_dashboard`.

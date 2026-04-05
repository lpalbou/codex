# Planned: Resume spawned agents

## Why this exists

Later Codex versions add `resume_agent`, which lets the orchestrator reopen a previously closed
agent thread instead of spawning a brand-new replacement.

For this fork, the value is not speed. The value is continuity:

- keep the same rollout / evidence trail
- avoid re-priming a replacement worker from scratch
- preserve the worker’s local context for long-running, careful tasks

## Current fork baseline

Today, `0.87.x`-based collab can:

- `spawn_agent`
- `send_input`
- `wait`
- `close_agent`
- `list_agents`

Once an agent is closed, the orchestrator cannot reopen it. The only option is to spawn a new
agent and restate the task.

## Why it is deferred

`resume_agent` is useful, but it is not as foundational as:

- explicit child provenance
- child completion notifications
- shared execpolicy
- shell snapshot reuse
- `list_agents`

Those pieces improve reliability immediately without changing the lifecycle model very much.

## Proposed design for this fork

Keep it simple and `0.87`-native:

1. Add a `resume_agent` collab tool that accepts an existing thread id.
2. Restrict it to threads that already belong to the same shared agent tree.
3. Reuse the same child provenance metadata if the thread was originally a thread-spawn child.
4. Rehydrate the same runtime overrides we already preserve for fresh children:
   - cwd
   - sandbox policy
   - approval policy
   - shell environment policy
   - inherited shell snapshot
   - shared execpolicy when applicable

## Acceptance criteria

- Resuming a previously closed child does not create a duplicate live-agent entry.
- `list_agents` / `/agents` show the resumed worker clearly.
- The resumed child keeps the same rollout path and previous evidence trail.
- The orchestrator can resume a child without changing the default depth-first behavior of the
  fork.

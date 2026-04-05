# Planned: Agent graph + mailbox-style waiting

## Why this exists

Later Codex versions evolve collab into a richer network:

- agent graph / parent-child path tracking
- mailbox-based `wait_agent`
- `send_message`
- `assign_task`
- deeper lifecycle management

That stack improves orchestration polish, but it is materially larger than the current
`0.87`-style collab model.

## Current fork baseline

This fork intentionally stays on the simpler v1 model:

- spawn children as real threads
- track live children explicitly
- let the parent `wait`, `send_input`, `close_agent`
- inject completion notifications back into the parent history

This is already a substantial improvement over stock `0.87` while keeping the architecture small.

## Why it is deferred

The v2 stack is not an obvious reasoning win by itself. It primarily improves:

- coordination ergonomics
- lifecycle management
- recoverability
- path-based naming

Those are useful, but they are secondary to this fork’s main objective: better work quality from
the selected main model.

## Recommended evaluation path

If we revisit this later, do it incrementally:

1. first add a lightweight agent-graph model for subtree close/resume
2. then evaluate whether mailbox waiting adds enough value
3. only then consider importing broader v2 concepts such as path-based targets

## What to avoid

- pulling in the entire later MultiAgentV2 stack at once
- changing the default orchestration model just to match upstream
- making the collab layer more complex before we prove a quality benefit

## Acceptance criteria

- Any added graph/mailbox layer must improve reliability or observability without forcing a
  speed-first orchestration style.
- The simpler current collab behavior must remain available as the default unless there is strong
  evidence that the new path improves outcomes.

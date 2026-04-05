# Planned: Selective child-context transfer (not full-history `fork_context`)

## Why this exists

Later Codex versions introduce `fork_context` / `fork_turns` features so children can inherit part
of the parent conversation history.

That sounds attractive, but a naive backport is risky for this fork:

- full-history child forks can bloat prompts
- larger child prompts increase compaction pressure
- more copied context does not necessarily improve reasoning quality
- several later upstream fixes show this area is still subtle and brittle

For a depth-first fork, the goal is not “copy everything”. The goal is **copy only the useful
context**.

## Current fork baseline

Thread-spawn sub-agents inherit:

- model / provider / reasoning effort
- cwd
- sandbox / approval policy
- shell environment policy
- developer/base/user instructions
- shell snapshot
- execpolicy state (when the same config roots apply)

They do **not** automatically inherit the full parent turn history. The spawn prompt must still be
self-contained.

## Recommended direction

Do **not** backport upstream boolean `fork_context` directly.

Instead, build a fork-specific selective model:

1. Reuse the `/context` block model already present in this fork.
2. Let the parent choose specific blocks or recent turns to inject into the child.
3. Keep the child’s prompt assembly explicit and observable.
4. Prefer bounded transfer such as:
   - `fork_turns = 1..N`
   - selected `/context` blocks
   - optional summarized blocks instead of raw history

## Why this fits the fork better

- better control over prompt size
- easier to audit
- easier to explain
- less likely to degrade reasoning by flooding the child with stale context

## Acceptance criteria

- The child receives only explicitly selected context.
- `/context` shows exactly what was injected.
- Prompt-size growth remains bounded.
- The feature improves nuanced subtask continuity without changing the fork’s default
  “self-contained spawn prompt” baseline.

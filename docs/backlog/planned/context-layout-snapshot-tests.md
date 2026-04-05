# Planned: Snapshot-test the model-visible context layout

## Why this exists

The fork now depends heavily on prompt fidelity:

- same model / same reasoning policy
- visible `/context` blocks
- richer long-turn history
- explicit sub-agent inheritance

The cleanest way to protect that baseline is to snapshot-test what the model actually sees.

## Current fork baseline

We already have:

- `/context` for human inspection
- context/history code that decides what enters the next request
- compaction and truncation logic that can subtly change prompt shape

What we do **not** yet have is a strong regression harness around the exact model-visible layout.

## Recommended direction

Add targeted tests around:

1. `history.for_prompt()` layout and filtering
2. reasoning/tool-call/tool-output retention
3. compaction replacement behavior
4. context-block summaries vs actual prompt composition

Prefer snapshot-style fixtures that encode the full visible prompt shape, not only field-by-field
assertions.

## Why this helps

- catches accidental prompt regressions early
- protects the fork’s reasoning-focused baseline
- makes future backports safer

## Acceptance criteria

- A prompt-layout regression fails a focused test.
- The tests cover both normal turns and compacted turns.
- The tests remain tied to model-visible context, not only UI rendering.

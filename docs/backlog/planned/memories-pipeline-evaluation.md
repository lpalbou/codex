# Planned: Evaluate later `memories` pipeline before importing anything

## Why this exists

From later Codex versions onward, the product starts building an explicit `memories` pipeline on
top of normal conversation history. By `0.118`, the pipeline uses smaller/faster side models:

- phase 1: `gpt-5.1-codex-mini` with low reasoning effort
- phase 2: `gpt-5.3-codex` with medium reasoning effort

This is not just “better storage”. It is additional summarization and abstraction pressure.

## Why this matters for this fork

This fork is intentionally optimized for:

- faithful continuity
- visible evidence trails
- deeper reasoning
- minimal silent summarization

The later memories pipeline may be useful for product ergonomics, but it is not obviously aligned
with those priorities.

## Current assessment

Potential benefits:

- longer-lived derived memory structures
- less pressure on raw prompt history
- easier long-session persistence

Potential risks:

- extra lossy summarization
- more reliance on smaller/faster helper models
- less direct continuity from the original evidence trail
- harder-to-audit “what exactly shaped the answer?” behavior

## Fork recommendation

Do **not** import later memories as-is.

If we revisit this area, prefer a fork-specific design that builds on the existing `/context`
blocks:

- durable memory blocks
- explicit enable/disable controls
- visible token estimates
- optional per-block compaction
- no hidden background mutation of the live history baseline

## Acceptance criteria

- Any memory system must remain observable and operator-controlled.
- The default raw-history baseline must stay intact.
- Imported or new memory features must improve long-session quality without silently replacing
  evidence-rich context with opaque summaries.

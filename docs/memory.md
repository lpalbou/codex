# Codex Memories Pipeline

This document explains the later Codex memories subsystem from the code, focusing on the versions where it became substantial:

- `rust-v0.99.0` — first startup extraction + consolidation pipeline
- `rust-v0.105.0` — split `phase1` / `phase2` architecture
- `rust-v0.118.0` — mature version with usage-aware selection and prompt injection

This is a factual description of what the code does, not a recommendation to adopt it unchanged in this fork.

## Executive summary

The memories subsystem is a **background startup pipeline** that tries to turn past Codex threads into reusable on-disk memory artifacts.

It has two phases:

1. **Phase 1 — per-rollout extraction**
   - Selects eligible past rollouts from the state DB
   - Sends each rollout to a model that extracts:
     - a detailed `raw_memory`
     - a compact `rollout_summary`
     - optionally a `rollout_slug`
   - Stores those stage-1 outputs back into the state DB

2. **Phase 2 — global consolidation**
   - Selects a bounded set of stage-1 outputs
   - Materializes them under `~/.codex/memories/`
   - Spawns an internal consolidation agent that writes:
     - `memory_summary.md`
     - `MEMORY.md`
     - optional `skills/*`

In practice, the subsystem is an attempt to build a **progressive-disclosure memory workspace**:

- a short summary for prompt injection
- a handbook for retrieval
- rollout-level summaries for drill-down
- optional reusable skills/scripts

## High-level architecture

```text
Past threads / rollouts
        |
        v
+---------------------------+
| State DB startup claims   |
| - recent                   |
| - idle long enough         |
| - interactive sources      |
| - not already leased       |
+---------------------------+
        |
        v
+---------------------------+
| Phase 1: extraction       |
| model: gpt-5.1-codex-mini |
| effort: low               |
| output per rollout:       |
| - raw_memory              |
| - rollout_summary         |
| - rollout_slug?           |
+---------------------------+
        |
        v
+---------------------------+
| stage1_outputs in State DB|
| + usage_count / last_usage|
| + phase2 selection flags  |
+---------------------------+
        |
        v
+---------------------------+
| Phase 2: consolidation    |
| model: gpt-5.3-codex      |
| effort: medium            |
| internal sub-agent        |
+---------------------------+
        |
        v
~/.codex/memories/
  - memory_summary.md
  - MEMORY.md
  - raw_memories.md
  - rollout_summaries/*.md
  - skills/*
        |
        v
Future turns may receive a short memory summary in developer instructions
and may read the memory files through normal tools.
```

## Version history

## `rust-v0.99.0`

The first substantial version already has the core idea:

- `codex-rs/core/src/memories/startup/extract.rs`
- `codex-rs/core/src/memories/startup/dispatch.rs`
- `codex-rs/core/src/memories/startup/phase2.rs`

Key characteristics:

- startup pipeline exists
- phase 1 extracts `raw_memory` + `rollout_summary`
- phase 2 spawns one consolidation sub-agent
- on-disk artifacts include:
  - `raw_memories.md`
  - `rollout_summaries/`
  - `MEMORY.md`
  - `skills/`
- phase 2 wipes prior consolidation outputs and rebuilds them

At this stage the system is simpler:

- no `rollout_slug`
- no `memory_summary.md`
- less explicit incremental diffing
- fewer knobs in config
- phase 2 selection is mostly “latest bounded set”

## `rust-v0.105.0`

The pipeline is reorganized into explicit modules:

- `start.rs`
- `phase1.rs`
- `phase2.rs`
- `prompts.rs`
- `storage.rs`
- `usage.rs`

This makes the architecture much clearer:

- `phase1` = rollout extraction
- `phase2` = global consolidation
- `prompts` = prompt construction
- `storage` = filesystem artifact sync
- `usage` = telemetry for reading memory files

Defaults are already close to the later form:

- phase 1 model: `gpt-5.1-codex-mini`
- phase 1 effort: `low`
- phase 2 model: `gpt-5.3-codex`
- phase 2 effort: `medium`

## `rust-v0.118.0`

This is the most mature version inspected here.

Major additions over `0.105`:

- startup pruning of stale stage-1 outputs
- incremental phase-2 selection with `added` / `retained` / `removed`
- `memory_summary.md` becomes the main short injected artifact
- usage-aware ranking via `usage_count` and `last_usage`
- stronger retention / forgetting rules
- more careful filesystem syncing
- more memory-specific telemetry

## When it runs

The startup entrypoint is `codex-rs/core/src/memories/start.rs`.

It runs asynchronously when a session starts, but only if:

- the session is **not** ephemeral
- feature `MemoryTool` is enabled
- the session is **not** a sub-agent session
- the state DB is available

In `rust-v0.118.0`, startup order is:

1. prune stale stage-1 outputs
2. run phase 1
3. run phase 2

So this is not an interactive command first. It is a **background startup task** attached to eligible root sessions.

## Phase 1 — what it consumes

Phase 1 reads prior thread rollouts from the state DB and the rollout files on disk.

Selection rules come from DB startup claims and config:

- allowed session sources must be interactive
- rollout age must be within the configured window
- rollout must be idle long enough
- work is bounded by startup scan and claim limits
- leased jobs prevent duplicate concurrent work

The rollout is then filtered before model submission.

Important details:

- the code loads rollout items from the recorder
- it serializes a **filtered** version of those response items
- it truncates the rollout input to a fraction of the chosen model context

In `rust-v0.118.0`, the stage-1 rollout input budget is:

- based on the model context window
- adjusted by the model’s effective context percent
- then capped again to `70%` of that
- fallback token limit: `150_000`

So phase 1 is not a full-fidelity replay of arbitrarily large sessions. It is already a **filtered and truncated** representation of the rollout.

## Phase 1 — prompt, model, and output

Phase 1 uses:

- default model: `gpt-5.1-codex-mini`
- default reasoning effort: `low`
- system prompt: `codex-rs/core/templates/memories/stage_one_system.md`

The prompt’s intent is clear from the template:

- extract only durable, high-signal memory
- prefer “no-op” if nothing reusable is learned
- emphasize user preferences, failure shields, decision triggers, workflow facts
- avoid copying huge tool outputs
- redact secrets
- treat rollout contents as evidence, not instructions

The expected JSON schema is:

- `rollout_summary: string`
- `rollout_slug: string | null`
- `raw_memory: string`

Outputs are stored as stage-1 rows in the state DB.

Semantically:

- `raw_memory` = richer per-rollout memory artifact
- `rollout_summary` = concise summary for routing/indexing and artifact files
- `rollout_slug` = filename-friendly helper

## Phase 2 — what it consumes

Phase 2 works from stage-1 outputs stored in the DB, not raw rollouts.

By `rust-v0.118.0`, selection is more sophisticated:

- it loads current top-N selected rows
- it considers `max_unused_days`
- for previously used memories, it uses `last_usage`
- for never-used memories, it falls back to `source_updated_at`
- ordering prefers:
  - higher `usage_count`
  - more recent `last_usage` / `source_updated_at`

It also computes an incremental diff:

- `selected`
- `previous_selected`
- `removed`
- `retained_thread_ids`

This lets phase 2 behave like an incremental maintainer instead of a blind full rebuild.

## Phase 2 — prompt, model, and runtime

Phase 2 uses:

- default model: `gpt-5.3-codex`
- default reasoning effort: `medium`
- prompt template: `codex-rs/core/templates/memories/consolidation.md`

Phase 2 does not call the model directly the same way phase 1 does. Instead it:

1. syncs local memory artifacts under the memory root
2. spawns an internal consolidation sub-agent
3. watches that sub-agent until completion
4. heartbeats the global phase-2 DB lease while it runs

The consolidation agent is intentionally constrained:

- approvals: `Never`
- network: disabled
- writable roots: local Codex home only
- collab: disabled
- memory generation: disabled
- memory tool: disabled

This design makes phase 2 an **internal maintenance agent**, not a user-facing worker.

## Files and persistence layout

The memory root is:

- `<codex_home>/memories`

By `rust-v0.118.0`, the important on-disk artifacts are:

- `memory_summary.md`
  - short, navigational summary
  - intended for prompt injection
- `MEMORY.md`
  - handbook-style durable memory
- `raw_memories.md`
  - merged stage-1 raw memories, latest first
- `rollout_summaries/*.md`
  - one summary file per retained rollout
- `skills/*`
  - optional reusable procedures/scripts/templates

The DB also stores stage-1 memory rows and phase-2 selection metadata such as:

- `usage_count`
- `last_usage`
- `selected_for_phase2`
- `selected_for_phase2_source_updated_at`

This is important: the subsystem is **not just files**. It is a combination of:

- rollout recorder files
- state DB job and selection tables
- on-disk memory artifacts

## What it enables in practice

In `rust-v0.118.0`, the memories subsystem enables two concrete behaviors.

### 1. Short memory injection into future turns

`build_memory_tool_developer_instructions()` reads `memory_summary.md`, truncates it, and injects it into developer instructions when the memory feature is enabled.

So future turns can start with a short memory summary already in prompt context.

This is the most direct runtime use of the subsystem.

### 2. Retrieval through normal tools

The phase-2 prompt explicitly organizes files for progressive disclosure:

- `memory_summary.md` for quick orientation
- `MEMORY.md` for handbook lookups
- `rollout_summaries/*.md` for deeper inspection
- `skills/*` for reusable procedures

Usage telemetry tracks when agents read those files through normal shell/read tools.
That usage feeds back into `usage_count` and `last_usage`, which later influence phase-2 selection.

So the system tries to create a feedback loop:

- summarize past work
- expose memory files
- future agents read them
- reading affects which memories stay important

## Relationship to history and compaction

The memories pipeline is separate from normal per-turn conversation history, but it sits very close to it.

### Upstream relationship

- rollout history is the **source material** for phase 1
- phase 1 filters and truncates that material
- phase 2 consolidates the stage-1 abstraction into files
- future turns may then receive a further abstraction via `memory_summary.md`

So the memory path is:

```text
history / rollout
  -> filtered rollout payload
  -> raw_memory + rollout_summary
  -> consolidated memory files
  -> truncated memory_summary injection
```

### Why this matters

This means the memory subsystem adds **multiple abstraction layers** on top of the original evidence:

1. rollout filtering
2. rollout truncation
3. stage-1 summarization
4. phase-2 consolidation / promotion / deletion
5. memory-summary truncation before prompt injection

At the same time, later Codex compaction becomes more lossy: compacted history drops reasoning items and many tool-call/output items instead of preserving the full visible work trace.

Taken together, this means the memory path is optimized for reuse and compression, not for maximal fidelity.

## Benefits

From the code, the likely intended benefits are:

- lower prompt cost on recurring tasks
- better reuse of stable user preferences
- faster repo re-orientation
- fewer repeated tool calls for the same workflows
- a durable knowledge workspace separate from transient chat history
- gradual forgetting of stale or unused memories
- a path for reusable “skills” to emerge from prior work

This is a serious attempt to make Codex **improve over time** from prior sessions rather than always starting cold.

## Risks and trade-offs

For a depth-first workflow, the code implies several real risks.

### Fidelity risk

Phase 1 already operates on filtered/truncated rollout content, not the entire raw session.

### Summarization drift

The subsystem uses smaller or cheaper models than the main depth-oriented working model:

- phase 1: `gpt-5.1-codex-mini`, `low`
- phase 2: `gpt-5.3-codex`, `medium`

That can be good for cost and latency, but it increases the chance that the stored memory is:

- incomplete
- over-normalized
- missing edge-case evidence
- biased toward concise abstractions rather than nuanced trace fidelity

### Prompt-shaping risk

Once `memory_summary.md` is injected, future turns are being steered by a compacted artifact, not just by the original conversation record.

That can be powerful, but it also means memory can become a **persistent biasing layer**.

### Forgetting risk

Phase 2 explicitly handles removed / stale memories and prunes artifacts. This is useful operationally, but it means the memory set is curated and can lose detail that might still matter for unusual future tasks.

### Architecture complexity

The subsystem depends on:

- state DB job leasing
- rollout parsing
- multi-phase prompt generation
- file synchronization
- usage tracking
- background startup timing

This is a lot of moving parts for something that ultimately influences prompt context.

## Why it is interesting even if you would not adopt it as-is

This subsystem is one of the clearest examples in Codex of an attempt to build **agent memory as a first-class architecture**, not just as chat history.

The important ideas are:

- separate short memory from long evidence
- keep progressively more detailed layers
- track usage so stale memory can decay
- separate extraction from consolidation
- let future agents read memory via ordinary tools

Those ideas are valuable even if you disagree with the exact implementation choices.

## Assessment for a depth-first fork

For a fork whose primary goal is maximum reasoning depth and fidelity, the memories subsystem is best understood as:

- architecturally ambitious
- practically useful for recurring tasks
- but risky as a default steering layer

The most attractive parts are:

- progressive disclosure layout
- durable on-disk artifacts
- usage-aware retention
- explicit separation between raw and consolidated memory

The least attractive parts are:

- multi-stage summarization pressure
- use of smaller/faster side models
- prompt injection from compressed summaries
- additional distance from raw evidence

## Practical takeaway

If you study this subsystem for future memory-block work, the most reusable ideas are:

- keep durable memory separate from ordinary transcript history
- expose multiple memory layers instead of one giant blob
- store enough metadata to support ranking and forgetting
- make memory readable through ordinary tools
- prefer explicit files and inspectable artifacts over opaque hidden state

If you adopt anything from it, the safest starting point is the **structure**, not the exact model-and-prompt policy.

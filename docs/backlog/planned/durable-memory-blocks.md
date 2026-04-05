# Planned: Durable memory blocks on top of `/context`

## Why this exists

The fork already has a strong **observability** baseline for prompt context:

- `/context` shows what is likely to enter the next request
- context is decomposed into inspectable blocks
- blocks can be enabled or disabled for future turns

What does **not** exist yet is the durable memory architecture the fork ultimately needs:

- long-lived blocks outside ordinary transcript history
- operator-controlled inclusion/exclusion
- per-block compaction
- selective child-context injection

## Current fork baseline

Today’s block model is still tied to the live session history and prompt assembly.

That is useful, but it is not yet a true memory system:

- blocks are not durable first-class artifacts
- they do not have their own storage lifecycle
- they cannot be compacted independently
- they are not yet the source of selective child-context transfer

## Recommended direction

Build this as a **separate durable layer**, not by overloading ordinary transcript history.

Suggested model:

1. `MemoryBlock`
   - `id`
   - `title`
   - `description`
   - `enabled`
   - `priority`
   - `raw_payload`
   - `injection_payload`
   - `token_estimate`
   - timestamps / provenance
2. durable storage under the Codex home or per-thread memory files
3. prompt injection at prompt-build time, not by mutating baseline history
4. optional per-block compaction that preserves the raw artifact
5. future selective child-context transfer built from chosen blocks

## Why this fits the fork

- keeps the evidence-rich baseline intact
- makes memory explicit and auditable
- avoids hidden background mutation
- supports operator control over what shapes the next request

## Acceptance criteria

- Memory blocks are durable and inspectable outside the live transcript.
- The default raw-history baseline remains unchanged.
- Blocks can be enabled/disabled without deletion.
- `/context` can show both live transcript blocks and durable memory blocks clearly.
- Future child-context transfer can draw from selected blocks instead of blind full-history forks.

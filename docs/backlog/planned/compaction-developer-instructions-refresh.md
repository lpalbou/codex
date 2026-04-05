# Planned: Refresh developer instructions after compaction rewrites history

## Why this exists

Later Codex versions fix a subtle but important compaction correctness issue: when history is
rewritten during compaction, the effective developer instructions need to stay aligned with the
current session configuration.

This matters more in a depth-first fork because long sessions are exactly where compaction becomes
relevant.

## Current fork baseline

This fork preserves a richer evidence trail than later upstream versions, but compaction still
replaces the in-memory history.

That creates a correctness question:

- after replacement, are the active developer instructions still the right ones for the rebuilt
  history?

## Recommended direction

Adopt the later semantic fix, but keep the implementation small:

1. detect history-replacement compaction events
2. refresh developer instruction state after replacement
3. avoid duplicating stale or superseded developer messages

## Why this helps

- better long-session prompt correctness
- less risk of stale steering surviving a history rewrite
- safer foundation for future resume/fork work

## Acceptance criteria

- After compaction, the next prompt reflects the current active developer instructions.
- Stale replaced developer messages do not remain as hidden steering.
- The change does not reduce the fork’s evidence-rich history baseline.

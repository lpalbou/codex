# Planned: Keep parent model/profile sticky when applying sub-agent roles

## Why this exists

Later Codex versions fix an important layering issue in agent-role application: applying a role
should not silently stomp on the parent-selected provider, profile, model, or reasoning policy
unless that override is explicitly intended.

For this fork, that matters because the core promise is:

- keep `gpt-5.2`
- keep `xhigh`
- keep the parent-selected provider/profile behavior

across the whole agent tree.

## Current fork baseline

This fork already preserves runtime spawn overrides and inherits the parent provider/model/effort
for thread-spawn children.

The remaining subtle risk is **role layering**:

- built-in or future role logic can still be applied on top of the child config
- profile selection and provider/model defaults must remain predictable
- the parent’s explicit choice should remain authoritative unless a role override is deliberate and
  opt-in

## Recommended direction

Backport only the semantics, not the whole later refactor:

1. build the child config from the parent session first
2. apply role defaults conservatively
3. re-apply explicit parent-selected provider/model/reasoning/profile fields after role layering
4. keep any explicit override feature gates obvious and opt-in

## Why this helps a depth-first fork

- avoids silent child downgrades
- keeps sub-agent behavior consistent with the parent session
- preserves the operator’s explicit model policy

## Acceptance criteria

- A parent explicitly using `gpt-5.2` + `xhigh` keeps that policy in children by default.
- Role application does not silently switch provider/profile/model.
- Any intentional override remains explicit, gated, and documented.

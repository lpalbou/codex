# Planned: `tui2` parity for in-session agent-limit controls

## Why this exists

This fork adds agent fan-out controls at two levels:

- launch-time CLI flags: `--max-threads`, `--max-depth`
- in-session slash commands: `/max-threads`, `/max-depth`

In the current fork state, the launch-time flags are wired in both `tui` and `tui2`, but the
interactive slash-command flow is only implemented in the legacy `tui`.

## Current fork baseline

- `tui` supports:
  - `/max-threads`
  - `/max-depth`
  - persistence to config
  - clean session restart so the new limits actually apply
- `tui2` supports the launch-time CLI parsing, but does not yet expose equivalent in-session
  controls

## Why this matters

This is not a reasoning-quality feature by itself, but it is a control-plane consistency issue:

- operators should not get different behavior depending on frontend
- observability and orchestration controls should be consistent across the fork
- the fork’s documented behavior should match both frontends

## Recommended direction

Keep the implementation small and aligned with the existing `tui` behavior:

1. reuse the same parsing semantics for `-1` / explicit limits
2. reuse the same persistence logic
3. reuse the same “restart session to apply” semantics
4. avoid introducing a separate policy model in `tui2`

## Acceptance criteria

- `tui2` supports `/max-threads` and `/max-depth`.
- The commands display current values with no argument.
- Changing a value persists it and restarts the session cleanly.
- The visible behavior matches `tui`.

# Planned: Observational watchdog for stuck sub-agents

## Why this exists

Later upstream work explores watchdog behavior for sub-agents. That can help detect workers that
stop making progress, but it can also be counterproductive for a fork that explicitly tolerates
long-running deep work.

This backlog item exists to keep the useful part — **observability** — without importing the risky
part — automatic intervention.

## Fork policy

This fork prefers:

- long waits
- fewer retries
- better visibility
- more confidence in one careful orchestration

Because of that, an auto-close or auto-interrupt watchdog is a poor default.

## Recommended direction

If we implement a watchdog at all, it should be observational only:

- detect agents with no status or output change for a long interval
- surface that in `/agents`
- optionally add a “possible stall” marker with elapsed time
- never close or interrupt the agent automatically

## Why this is better

- keeps the user in control
- avoids killing legitimate long-running work
- improves trust and debuggability

## Acceptance criteria

- The watchdog does not alter agent behavior by default.
- `/agents` makes possible stalls obvious.
- Any later intervention feature must be explicit and opt-in.

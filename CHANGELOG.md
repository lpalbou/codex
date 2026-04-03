# Changelog

Upstream changelog: https://github.com/openai/codex/releases

## Fork changes (lpalbou/codex)

Fork notes: `docs/lpalbou-fork.md`.

This fork advocates for **depth over speed**: I would rather wait 1h+ for a single, careful,
agentic orchestration than supervise 10 fast iterations full of mistakes.

Slower runs also make the system easier to observe (I can follow what’s happening in real time),
and reduce the “tsunami” of trial-and-error text that comes from rapid, shallow retries.

Concretely, this fork aims to keep model selection + reasoning effort (for example `gpt-5.2` +
`model_reasoning_effort = "xhigh"`) predictable and consistent across Codex tasks, including
spawned sub-agents.

### Unreleased

- Default fresh-install model selection to `gpt-5.2` and default reasoning effort to `xhigh`
  (Responses-based providers).
- Add feature flag `worker_model_override` (default: `false`) to prevent `agent_type=worker`
  sub-agents from overriding the parent model to `gpt-5.2-codex`.
  - Enable to restore upstream behavior: `codex --enable worker_model_override`
- Add `/agents` TUI overlay for live sub-agent observability (status, last action, approvals, and
  context-window usage).
- Add `/save` slash command to export the full chat history to a markdown file.
- Note: `/compact` uses the remote `responses/compact` endpoint by default, which does not accept
  `reasoning.effort`. Disable `remote_compaction` to force local (Responses API) compaction:
  `codex --disable remote_compaction`

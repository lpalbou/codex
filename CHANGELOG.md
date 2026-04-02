# Changelog

Upstream changelog: https://github.com/openai/codex/releases

## Fork changes (lpalbou/codex)

This fork exists to help enforce using a single model + reasoning effort (for example
`gpt-5.2` + `model_reasoning_effort = "xhigh"`) consistently across Codex tasks, including
sub-agents.

### Unreleased

- Add feature flag `worker_model_override` (default: `false`) to prevent `agent_type=worker`
  sub-agents from overriding the parent model to `gpt-5.2-codex`.
  - Enable to restore upstream behavior: `codex --enable worker_model_override`
- Note: `/compact` uses the remote `responses/compact` endpoint by default, which does not accept
  `reasoning.effort`. Disable `remote_compaction` to force local (Responses API) compaction:
  `codex --disable remote_compaction`

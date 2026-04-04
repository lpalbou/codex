# Planned: Provider switching mid-thread (prompt sanitization + wire compatibility)

## Summary

Codex threads record a rich, OpenAI-shaped transcript (`Vec<ResponseItem>`) that is sent back to the model on the next turn. This works well when the backend remains consistent (OpenAI → OpenAI), but can break when the user switches providers mid-thread (OpenAI → OSS server such as LM Studio / Ollama) because:

- the OSS server may not accept OpenAI-specific `ResponseItem` variants/fields, and/or
- OpenAI-only payload fields (like `prompt_cache_key`) may be rejected or ignored, and/or
- the OSS server may not implement the OpenAI **Responses API** semantics well enough for real-world prompts (even if the endpoint exists).

This backlog item defines a policy and an implementation plan for “cross-provider threads” that keeps Codex robust while keeping the current default behavior unchanged.

## Why this matters

Without guardrails, a user can:

1) run a long thread on OpenAI (where encrypted reasoning and other artifacts exist), then
2) switch to a local model provider for a quick follow-up.

The follow-up request may fail (hard error) or silently degrade (wasted context budget / confusing tokens) depending on the server and the recorded history.

The goal is reliability and predictability, not speed.

## Current behavior (0.87.x fork baseline)

### What gets recorded into history (eligible for later prompts)

- Context manager records “API messages”:
  - non-system `message` items,
  - tool calls + tool outputs,
  - local shell calls,
  - web search calls,
  - `reasoning`,
  - `compaction`.
  - `GhostSnapshot` is stored but not treated as an API message.
  - See `codex-rs/core/src/context_manager/history.rs` (`record_items`, `is_api_message`).

### What gets sent back in the next request (Responses API)

- `ContextManager::for_prompt()` normalizes the history and removes `GhostSnapshot` items only.
  - See `codex-rs/core/src/context_manager/history.rs:for_prompt`.
- The resulting `Vec<ResponseItem>` is serialized and sent as the `input` field of the `/v1/responses` request.

### OpenAI-only “prompt continuity”

- If the selected model supports reasoning summaries, Codex requests `reasoning.encrypted_content` and records it in history.
- That opaque blob is then sent back verbatim on later turns.
- See `docs/backlog/planned/encrypted-reasoning-across-providers.md`.

### Chat wire API behavior (when enabled)

- When using Chat Completions wire API, the chat request builder **drops**:
  - `ResponseItem::Reasoning`,
  - `ResponseItem::WebSearchCall`,
  - `ResponseItem::Compaction`,
  - `ResponseItem::GhostSnapshot`,
  - and other non-chat artifacts.
  - See `codex-rs/codex-api/src/requests/chat.rs`.

## Observed OSS incompatibility (LM Studio example)

User report (LM Studio OpenAI-compatible server):

- Command: `cargo run --bin codex -- --provider lmstudio --model qwen3.5-4b@q8_0`
- Codex sends a `POST /v1/responses` request containing:
  - `instructions` (system prompt),
  - `input` as a list of `type="message"` items with `content=[{type:"input_text",...}]`,
  - `tools=[...]`,
  - `prompt_cache_key=...`.
- LM Studio warns about unsupported/ignored fields, then fails with:
  - `Error rendering prompt with jinja template: "No user query found in messages."`

This suggests that some OSS servers/models/templates cannot reliably render prompts from the Responses API request shape Codex uses, even for trivial prompts.

## Goals

- Provider switching mid-thread should be predictable and not “randomly break”.
- Prefer robust, simple behavior over clever heuristics.
- Preserve the existing OpenAI behavior by default (no breaking change to OpenAI threads).

## Non-goals

- Perfect feature parity across all OpenAI-compatible gateways.
- Automatically “fixing” broken model prompt templates (we can only shape inputs to be more widely compatible).

## Policy options

### Option A — Document the constraint (simplest)

Document: “When switching providers (OpenAI ↔ OSS), start a new thread.”

Pros: zero risk, zero code.
Cons: UX is worse, and users will still try to do it.

### Option B — Provider-aware prompt sanitization (recommended)

When building a request, sanitize history for the selected provider/wire API without mutating the durable transcript.

Examples:
- If provider is **not OpenAI**:
  - strip `ResponseItem::Reasoning` (especially encrypted content),
  - strip any known OpenAI-only marker items if needed,
  - consider dropping `prompt_cache_key` from the request options,
  - potentially collapse “developer” role messages into `system` where a provider does not support `developer`.

Pros: robust; preserves full transcript; minimizes user footguns.
Cons: must be very explicit and observable (no hidden magic).

### Option C — Dual history stores (powerful, larger refactor)

Maintain:
- an append-only full transcript, plus
- a provider-specific “prompt view” that can diverge (sanitized, compacted, provider-aware).

Pros: best UX in the long term.
Cons: bigger refactor; out-of-scope for an incremental fix.

## Proposed implementation plan (incremental)

1) **Add a sanitization layer at prompt-build time**
   - Build `api_prompt.input` from `ContextManager::for_prompt()` as usual, then apply:
     - `sanitize_input_for_provider(&provider, &model_info, input_items) -> Vec<ResponseItem>`.
   - This keeps on-disk rollouts unchanged and preserves the “truth” transcript.

2) **Keep sanitization rules minimal and explicit**
   - Start with the smallest safe rule set:
     - if provider is not OpenAI: drop `ResponseItem::Reasoning` entirely.
   - Expand only when we have concrete failures to justify new rules.

3) **Surface the behavior in observability**
   - `/context` should show both:
     - “recorded” items, and
     - “next request” items (after sanitization), so users can see what will be sent.

## Acceptance criteria

- Switching provider from OpenAI → OSS mid-thread does not fail due to `type="reasoning"` or `encrypted_content`.
- Local providers do not receive obviously OpenAI-only blobs by default.
- The user can understand and predict what will be sent (no invisible deletions).

## Open questions

- Should we define a “compatibility mode” for OSS providers that prefers Chat Completions when available?
- Should `prompt_cache_key` be suppressed for non-OpenAI providers by default?
- Are there other OpenAI-only `ResponseItem` variants that OSS servers reject in practice?


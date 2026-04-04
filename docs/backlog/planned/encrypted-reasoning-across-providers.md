# Planned: Encrypted reasoning across providers (OpenAI vs OSS)

## Why this exists

When Codex uses the OpenAI **Responses API** with a reasoning-capable model, it can request that the server include an opaque `reasoning.encrypted_content` field in the stream. Codex then records that `ResponseItem::Reasoning` into the thread history and sends it back on later turns.

This is likely useful only when the next request is handled by the same OpenAI backend that produced the encrypted payload. It is unclear what happens if the same thread is later routed to an **open-source / self-hosted** OpenAI-compatible server (LM Studio, Ollama, etc.).

This backlog item tracks investigation + possible compatibility work.

## Current behavior (0.87.x fork baseline)

### What is actually sent on the next turn (Responses API)

- Codex stores a filtered transcript of the turn as `Vec<ResponseItem>` in the context manager:
  - Recording filter: `is_api_message` includes non-system `message` items plus `reasoning`, tool calls, tool outputs, shell calls, web search calls, and compaction markers. `GhostSnapshot` is stored but not treated as an API message.
  - See `codex-rs/core/src/context_manager/history.rs` (`record_items`, `is_api_message`).
- When building the next model request, Codex calls `ContextManager::for_prompt()` which:
  - normalizes call/output pairing invariants, and
  - removes `GhostSnapshot` items before sending to the model.
  - See `codex-rs/core/src/context_manager/history.rs:for_prompt`.
- For the **Responses API** wire, Codex serializes and sends that `Vec<ResponseItem>` as the `input` field of the `/v1/responses` request (no local decoding/transformation of encrypted reasoning).

Implication: if the history contains `ResponseItem::Reasoning { encrypted_content: Some("…") }`,
that opaque string is sent back verbatim in the next request.

- Codex requests encrypted reasoning content when the model advertises `supports_reasoning_summaries`:
  - `include = ["reasoning.encrypted_content"]` when `reasoning.is_some()`.
  - See `codex-rs/core/src/client.rs:280` and `codex-rs/core/src/client.rs:293`.

- `ResponseItem::Reasoning` is treated as an API/history item and is eligible to be sent back in the next request:
  - Recorded because `is_api_message` returns true for `ResponseItem::Reasoning`.
  - See `codex-rs/core/src/context_manager/history.rs:298`.

- The only explicit removal before sending is `GhostSnapshot`:
  - See `codex-rs/core/src/context_manager/history.rs:72`.

- If the provider uses **Chat Completions** wire API, `ResponseItem::Reasoning` items are dropped while building the request body:
  - See `codex-rs/codex-api/src/requests/chat.rs:285`.

### Why this becomes a cross-provider risk

- “OSS” providers (`--provider lmstudio`, `--provider ollama`, etc.) typically do **not** implement any server-side semantics for `reasoning.encrypted_content`.
- Even when using an OSS provider from the start, Codex usually won’t request encrypted reasoning because the built-in `gpt-oss` model metadata has `supports_reasoning_summaries = false`.
- The problematic case is a **provider switch mid-thread** (OpenAI → OSS): the history may already contain encrypted reasoning items from earlier OpenAI turns, and those items may then be sent to a non-OpenAI backend.

## Core questions

1. **What is the contract** of `reasoning.encrypted_content`?
   - Is it intended to be sent back to the model as continuity context?
   - Is it only meaningful for OpenAI’s servers (and meaningless/invalid elsewhere)?

2. What happens if a thread containing `ResponseItem::Reasoning { encrypted_content: Some(...) }` is later sent to:
   - LM Studio Responses API (`/v1/responses`)
   - Ollama Responses API (`/v1/responses`)
   - Ollama Chat API (`/v1/chat/completions`)
   - Other OpenAI-compatible gateways (LiteLLM, vLLM OpenAI server, etc.)

3. Do newer Codex versions add any guardrails/sanitization for non-OpenAI providers?

## Hypotheses / risks

- Many OSS “OpenAI-compatible” servers are compatible at the **request envelope** level but may not accept all OpenAI-specific `ResponseItem` variants (e.g. `type="reasoning"`, `encrypted_content` fields).
- If such a server rejects unknown item types/fields, switching a thread from OpenAI → OSS provider mid-thread could fail.
- Even if accepted, the encrypted blob is likely useless to the OSS model and wastes context budget.

## Concrete “minimal repro” request shape

The smallest “does this server accept encrypted reasoning?” probe is a `/v1/responses` request containing *any* `type="reasoning"` item plus a user message, e.g.:

- `input[0] = { "type": "reasoning", "encrypted_content": "<dummy>" }`
- `input[1] = { "type": "message", "role": "user", "content": [{ "type": "input_text", "text": "hello" }] }`

Expected outcomes:
- OpenAI: accepts and may use it.
- OSS server: may reject unknown item types/fields, or accept but ignore.

## Investigation plan

### A) Version diff scan (0.87 → newer releases)

Goal: Determine whether upstream implemented provider-aware prompt sanitization or changed how OSS providers are wired.

- Compare tags `rust-v0.87.0` vs `rust-v0.92.0` vs `rust-v0.93.0` for:
  - request building (`codex-rs/core/src/client.rs`, `codex-rs/codex-api/src/requests/*.rs`)
  - history normalization (`codex-rs/core/src/context_manager/history.rs`)
  - built-in provider wiring (`codex-rs/core/src/model_provider_info.rs`)

### B) Behavioral tests against local servers

Goal: Empirically verify whether LM Studio / Ollama accept `ResponseItem::Reasoning` in Responses API input.

Minimal experiment:
- Start a local server (LM Studio or Ollama) exposing an OpenAI-compatible base URL.
- Send a raw `/v1/responses` request with:
  - `input = [ { type: "reasoning", encrypted_content: "<dummy>" }, { type: "message", ... } ]`
  - Observe: HTTP status + error payload.

If rejected:
- Retest after stripping the reasoning item.
- Retest using Chat Completions wire API (which drops reasoning items by construction).

### C) Decide what “cross-provider threads” should mean

Options (in increasing complexity):
1. **Document a constraint**: “Do not switch providers mid-thread; start a new thread when changing providers.”
2. **Provider-aware sanitization** (recommended): when provider is not OpenAI (or when `supports_reasoning_summaries` is false), strip `ResponseItem::Reasoning` (and possibly other OpenAI-specific artifacts) from the request input at prompt-build time.
3. **Transform to plain text**: replace encrypted reasoning with a short, user-visible summary message (if available), preserving “meaningful continuity” without provider coupling.

## Proposed acceptance criteria (if we implement something)

- Switching from OpenAI → OSS provider mid-thread does not crash or error due to unsupported input item types.
- The next request input is still coherent (user/assistant turns remain intact).
- `/context` clearly shows what will be sent after sanitization (no hidden behavior).

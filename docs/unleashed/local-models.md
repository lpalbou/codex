# Local models (LM Studio / Ollama)

Open Codex Unleashed supports local, OpenAI-compatible servers via built-in providers:

- `lmstudio` (Chat Completions wire API)
- `lmstudio-responses` (Responses wire API)
- `ollama` (Chat Completions wire API)
- `ollama-responses` (Responses wire API)
- `ollama-chat` (Chat Completions wire API; legacy alias)

## Quick start

Start your local server, then run:

```sh
codex-unleashed --provider ollama --model qwen2.5-coder:7b
```

## Syntax

```sh
codex-unleashed --provider <lmstudio|ollama|ollama-chat> --model <MODEL> [--base-url <URL>]
```
Also supported:

```sh
codex-unleashed --provider <lmstudio-responses|ollama-responses> --model <MODEL> [--base-url <URL>]
```

If `--base-url` is omitted, Codex uses the provider defaults:

- Ollama: `http://localhost:11434/v1`
- LM Studio: `http://localhost:1234/v1`

## Example: LM Studio

```sh
codex-unleashed --provider lmstudio --base-url http://localhost:1234/v1 --model <your-model-id>
```

## Notes

- Provider selection applies to spawned sub-agents for the run (so everything stays on the same
  backend).
- For OSS providers, Codex does not require OpenAI auth by default.

## Implementation notes (this fork)

- `codex-rs/tui/src/cli.rs`: adds `--provider` and `--base-url`.
- `codex-rs/tui/src/lib.rs`: applies provider/base-url overrides by setting `OPENAI_BASE_URL` or
  `CODEX_OSS_BASE_URL` early in startup.
- Provider defaults: `codex-rs/core/src/model_provider_info.rs`.

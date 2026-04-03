# Open Codex Unleashed

**Open Codex Unleashed** is a personal fork of OpenAI’s open-source **Codex CLI** (based on
`rust-v0.87.0`), maintained by **Laurent-Philippe Albou**.

Fork URL: https://github.com/lpalbou/codex  
Fork start date: 2026-04-03

## Why this fork exists

This fork advocates for **depth over speed**: I would rather wait 1h+ for a single, careful,
agentic orchestration than supervise 10 fast iterations full of mistakes.

Slower runs also make the system easier to observe (I can follow what’s happening in real time),
and reduce the “tsunami” of trial-and-error text that comes from rapid, shallow retries.

## What’s different (high level)

This fork focuses on:

- **Predictable model selection**: fresh installs default to **`gpt-5.2`** with
  **`model_reasoning_effort = "xhigh"`** (for Responses-based providers).
- **Consistent sub-agent routing**: prevents “worker” sub-agents from silently overriding the
  parent model (upstream hard-coded `gpt-5.2-codex` behavior can be re-enabled via feature flag).
- **Observability**: adds a `/agents` TUI overlay with live sub-agent status, task, approvals, and
  context-window usage.
- **Auditability**: adds a `/save` slash command to export the **full** chat history to Markdown
  (use `--full` to include tool calls/results; default exports only user/assistant/thoughts).

More details: `CHANGELOG.md` and `docs/lpalbou-fork.md`.

## Install (from source)

Prereqs:

- Rust toolchain (stable) + Cargo
- `just` (https://github.com/casey/just)

Steps:

```sh
git clone https://github.com/lpalbou/codex.git
cd codex
```

Build and install the `codex` binary into `~/.cargo/bin`:

```sh
cargo install --path codex-rs/cli --bin codex --locked --force
```

Run:

```sh
codex
```

If you’re developing from source (no install), run:

```sh
just codex
```

## Default model + reasoning

On a fresh install, this fork defaults to:

- model: `gpt-5.2`
- reasoning effort: `xhigh` (Responses-based providers)

You can change it at runtime with `/model`, or persist it in `~/.codex/config.toml`:

```toml
model = "gpt-5.2"
model_reasoning_effort = "xhigh"
```

## Using local models (LM Studio / Ollama)

This fork includes built-in “OSS” providers:

- `lmstudio` (wire API: Responses)
- `ollama` (wire API: Responses)
- `ollama-chat` (wire API: Chat Completions)

### Pointing Codex at a custom server URL

The built-in OSS providers read the base URL from environment variables:

- `CODEX_OSS_BASE_URL` (recommended): full base URL, e.g. `http://localhost:11434/v1`
- `CODEX_OSS_PORT`: convenience port override when `CODEX_OSS_BASE_URL` is unset

Example:

```sh
export CODEX_OSS_BASE_URL="http://localhost:11434/v1"
```

### Example: Ollama (Chat Completions)

1) Start Ollama (and ensure your model is available).
2) Point Codex at Ollama’s OpenAI-compatible endpoint:

```sh
export CODEX_OSS_BASE_URL="http://localhost:11434/v1"
```

3) Set provider + model in `~/.codex/config.toml`:

```toml
model_provider = "ollama-chat"
model = "qwen2.5-coder:7b" # example
```

4) Run `codex`.

### Example: LM Studio (Responses API)

1) Start the LM Studio local server in “OpenAI compatible” mode.
2) Point Codex at the server:

```sh
export CODEX_OSS_BASE_URL="http://localhost:1234/v1"
```

3) Set provider + model in `~/.codex/config.toml`:

```toml
model_provider = "lmstudio"
model = "your-model-id" # replace with the model name exposed by the server
```

4) Run `codex`.

> Note: `lmstudio` and `ollama` use the **Responses** wire API (`/v1/responses`). If your server
> only supports Chat Completions (`/v1/chat/completions`), prefer `ollama-chat` or add your own
> provider entry.

## Saving history

Use `/save` to write a Markdown export of the current chat history to disk:

- `/save` → `codex-<timestamp>.md` (compact: user/assistant/thoughts)
- `/save notes` → `notes.md` (compact)
- `/save --full` → includes tool calls/results, patches, and other events
- `/save --full notes` → full export to `notes.md`

## Credits / upstream

- Upstream project: OpenAI Codex CLI — https://github.com/openai/codex
- Docs: https://developers.openai.com/codex

This repository remains licensed under the [Apache-2.0 License](LICENSE). This fork is not
affiliated with OpenAI.

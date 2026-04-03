# Open Codex Unleashed

**Open Codex Unleashed** is a personal fork of OpenAI’s open-source **Codex CLI** (based on
`rust-v0.87.0`), maintained by **Laurent-Philippe Albou**.

Fork URL: https://github.com/lpalbou/codex  
Fork start date: 2026-04-03

## Why this fork exists

This fork advocates for **depth over speed**: one careful, long-running agentic orchestration (even
1h+) is often more productive than many fast iterations with avoidable mistakes.

## Quick start (side-by-side install)

```sh
git clone https://github.com/lpalbou/codex.git
cd codex
cargo install --path codex-rs/cli --bin codex-unleashed --locked --force
codex-unleashed
```

`codex-unleashed` uses `~/.codex-unleashed` by default (so it doesn’t touch your existing
`~/.codex`).

Optional: reuse existing auth (no re-login):

```sh
mkdir -p ~/.codex-unleashed
cp ~/.codex/auth.json ~/.codex-unleashed/auth.json
```

## Local models (one example)

```sh
codex-unleashed --provider ollama --model qwen2.5-coder:7b
```

## Documentation (deep dive)

- `docs/unleashed/installation.md` — install, upgrade, side-by-side config, auth reuse
- `docs/unleashed/model-control.md` — default model/effort, consistent routing, compaction notes
- `docs/unleashed/local-models.md` — LM Studio / Ollama syntax and defaults
- `docs/unleashed/agents-dashboard.md` — `/agents` dashboard + how to enable real sub-agents
- `docs/unleashed/save.md` — `/save` (compact/full) + JSONL exports for SFT/CPT

## Credits / upstream

- Upstream repo: https://github.com/openai/codex
- Upstream docs: https://developers.openai.com/codex

This repository remains licensed under the [Apache-2.0 License](LICENSE). This fork is not
affiliated with OpenAI.

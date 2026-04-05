# Installation (side-by-side)

This guide installs **Open Codex Unleashed** as `codex-unleashed` **without** replacing your
existing `codex` install.

By default, `codex-unleashed` uses its own config directory: `~/.codex-unleashed` (unless
`CODEX_HOME` is set).

## Prerequisites

- Rust toolchain (stable) + Cargo
- `git`

Optional (recommended for repo workflows):

- `just` (https://github.com/casey/just)

## Install from source (recommended)

```sh
git clone https://github.com/lpalbou/codex.git
cd codex

# Install a separate binary name.
cargo install --path codex-rs/cli --bin codex-unleashed --locked --force
```

Verify:

```sh
codex-unleashed --version
```

Run:

```sh
codex-unleashed
```

Useful launch examples:

```sh
# Keep the fork defaults: gpt-5.2 + xhigh + unlimited spawn limits
codex-unleashed

# Explicitly cap spawned-agent fan-out for one run
codex-unleashed --max-threads 8 --max-depth 2

# Route the built-in OpenAI provider through a compatible gateway
codex-unleashed --base-url http://127.0.0.1:8099/v1
```

## Optional: reuse existing authentication

Codex stores credentials in `$CODEX_HOME/auth.json`. If you already use upstream Codex (`~/.codex`)
and want `codex-unleashed` to start authenticated:

```sh
mkdir -p ~/.codex-unleashed
cp ~/.codex/auth.json ~/.codex-unleashed/auth.json
```

If you prefer to keep auth separate, skip this step; `codex-unleashed` will prompt you.

## Run from a local checkout (no install)

```sh
cd codex/codex-rs
cargo run --bin codex-unleashed
```

You can forward any CLI flags after `--`:

```sh
cargo run --bin codex-unleashed -- --enable collab
```

For example:

```sh
cargo run --bin codex-unleashed -- --enable collab --max-threads 12 --max-depth 3
```

## Uninstall

If you installed with `cargo install`:

```sh
cargo uninstall codex-cli
```

Note: this uninstalls the `codex-cli` package binaries you installed (including
`codex-unleashed`). If you also installed upstream Codex from the same Cargo package, reinstall it
afterwards.

## Implementation notes (this fork)

- `codex-rs/cli/Cargo.toml`: defines the additional `codex-unleashed` binary.
- `codex-rs/cli/src/main.rs`: auto-sets `CODEX_HOME` to `~/.codex-unleashed` when the executable
  name is `codex-unleashed` and `CODEX_HOME` is not set.

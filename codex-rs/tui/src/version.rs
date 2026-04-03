/// The current Codex CLI version as embedded at compile time.
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Fork display name shown in the TUI.
pub const FORK_NAME: &str = "Open Codex Unleashed";

/// Fork version shown in the TUI.
///
/// This is intentionally independent of `CODEX_CLI_VERSION` so the fork can have
/// its own versioning scheme while still being based on an upstream Codex tag.
pub const FORK_VERSION: &str = "0.1.0";

/// Fork URL shown in the TUI.
pub const FORK_URL: &str = "https://github.com/lpalbou/codex";

/// Fork author shown in the TUI.
pub const FORK_AUTHOR: &str = "Laurent-Philippe Albou";

/// Upstream project name used in copy.
pub const UPSTREAM_NAME: &str = "OpenAI Codex";

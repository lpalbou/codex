use clap::Parser;
use clap::ValueHint;
use codex_common::ApprovalModeCliArg;
use codex_common::CliConfigOverrides;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    /// Optional user prompt to start the session.
    #[arg(value_name = "PROMPT", value_hint = clap::ValueHint::Other)]
    pub prompt: Option<String>,

    /// Optional image(s) to attach to the initial prompt.
    #[arg(long = "image", short = 'i', value_name = "FILE", value_delimiter = ',', num_args = 1..)]
    pub images: Vec<PathBuf>,

    // Internal controls set by the top-level `codex resume` subcommand.
    // These are not exposed as user flags on the base `codex` command.
    #[clap(skip)]
    pub resume_picker: bool,

    #[clap(skip)]
    pub resume_last: bool,

    /// Internal: resume a specific recorded session by id (UUID). Set by the
    /// top-level `codex resume <SESSION_ID>` wrapper; not exposed as a public flag.
    #[clap(skip)]
    pub resume_session_id: Option<String>,

    /// Internal: show all sessions (disables cwd filtering and shows CWD column).
    #[clap(skip)]
    pub resume_show_all: bool,

    // Internal controls set by the top-level `codex fork` subcommand.
    // These are not exposed as user flags on the base `codex` command.
    #[clap(skip)]
    pub fork_picker: bool,

    #[clap(skip)]
    pub fork_last: bool,

    /// Internal: fork a specific recorded session by id (UUID). Set by the
    /// top-level `codex fork <SESSION_ID>` wrapper; not exposed as a public flag.
    #[clap(skip)]
    pub fork_session_id: Option<String>,

    /// Internal: show all sessions (disables cwd filtering and shows CWD column).
    #[clap(skip)]
    pub fork_show_all: bool,

    /// Model the agent should use.
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Model provider to use (openai, lmstudio, ollama, ollama-chat).
    ///
    /// This overrides config defaults and applies to spawned sub-agents.
    #[arg(long = "provider")]
    pub provider: Option<String>,

    /// Override the provider base URL (e.g. http://localhost:11434/v1).
    ///
    /// When used with an OSS provider, sets CODEX_OSS_BASE_URL for this run.
    /// Otherwise overrides the built-in `openai` provider base URL for this run.
    #[arg(long = "base-url")]
    pub base_url: Option<String>,

    /// Maximum number of spawned sub-agents allowed for this run.
    ///
    /// Use `-1` for unlimited and `0` to disable spawned agents.
    #[arg(long = "max-threads", allow_hyphen_values = true)]
    pub max_threads: Option<i64>,

    /// Maximum nesting depth allowed for spawned sub-agents for this run.
    ///
    /// Root sessions start at depth `0`. Use `-1` for unlimited and `0` to disable child spawns.
    #[arg(long = "max-depth", allow_hyphen_values = true)]
    pub max_depth: Option<i64>,

    /// Convenience flag to select the local open source model provider. Equivalent to -c
    /// model_provider=oss; verifies a local LM Studio or Ollama server is running.
    #[arg(long = "oss", default_value_t = false)]
    pub oss: bool,

    /// Specify which local provider to use (lmstudio, ollama, or ollama-chat).
    /// If not specified with --oss, will use config default or show selection.
    #[arg(long = "local-provider")]
    pub oss_provider: Option<String>,

    /// Configuration profile from config.toml to specify default options.
    #[arg(long = "profile", short = 'p')]
    pub config_profile: Option<String>,

    /// Select the sandbox policy to use when executing model-generated shell
    /// commands.
    #[arg(long = "sandbox", short = 's')]
    pub sandbox_mode: Option<codex_common::SandboxModeCliArg>,

    /// Configure when the model requires human approval before executing a command.
    #[arg(long = "ask-for-approval", short = 'a')]
    pub approval_policy: Option<ApprovalModeCliArg>,

    /// Convenience alias for low-friction sandboxed automatic execution (-a on-request, --sandbox workspace-write).
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    /// Skip all confirmation prompts and execute commands without sandboxing.
    /// EXTREMELY DANGEROUS. Intended solely for running in environments that are externally sandboxed.
    #[arg(
        long = "dangerously-bypass-approvals-and-sandbox",
        alias = "yolo",
        default_value_t = false,
        conflicts_with_all = ["approval_policy", "full_auto"]
    )]
    pub dangerously_bypass_approvals_and_sandbox: bool,

    /// Tell the agent to use the specified directory as its working root.
    #[clap(long = "cd", short = 'C', value_name = "DIR")]
    pub cwd: Option<PathBuf>,

    /// Enable live web search. When enabled, the native Responses `web_search` tool is available to the model (no per‑call approval).
    #[arg(long = "search", default_value_t = false)]
    pub web_search: bool,

    /// Additional directories that should be writable alongside the primary workspace.
    #[arg(long = "add-dir", value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub add_dir: Vec<PathBuf>,

    /// Disable alternate screen mode for better scrollback in terminal multiplexers like Zellij.
    /// This runs the TUI in inline mode, preserving terminal scrollback history.
    #[arg(long = "no-alt-screen", default_value_t = false)]
    pub no_alt_screen: bool,

    /// Allow showing the model migration prompt on startup (opt-in).
    ///
    /// By default, Codex suppresses this prompt and continues using the current model.
    #[arg(long = "allow-migration-prompt", default_value_t = false)]
    pub allow_migration_prompt: bool,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,
}

impl From<codex_tui::Cli> for Cli {
    fn from(cli: codex_tui::Cli) -> Self {
        Self {
            prompt: cli.prompt,
            images: cli.images,
            resume_picker: cli.resume_picker,
            resume_last: cli.resume_last,
            resume_session_id: cli.resume_session_id,
            resume_show_all: cli.resume_show_all,
            fork_picker: cli.fork_picker,
            fork_last: cli.fork_last,
            fork_session_id: cli.fork_session_id,
            fork_show_all: cli.fork_show_all,
            model: cli.model,
            provider: cli.provider,
            base_url: cli.base_url,
            max_threads: cli.max_threads,
            max_depth: cli.max_depth,
            oss: cli.oss,
            oss_provider: cli.oss_provider,
            config_profile: cli.config_profile,
            sandbox_mode: cli.sandbox_mode,
            approval_policy: cli.approval_policy,
            full_auto: cli.full_auto,
            dangerously_bypass_approvals_and_sandbox: cli.dangerously_bypass_approvals_and_sandbox,
            cwd: cli.cwd,
            web_search: cli.web_search,
            add_dir: cli.add_dir,
            no_alt_screen: cli.no_alt_screen,
            allow_migration_prompt: cli.allow_migration_prompt,
            config_overrides: cli.config_overrides,
        }
    }
}

use crate::agent::AgentStatus;
use crate::codex::Codex;
use crate::config::Config;
use crate::error::Result as CodexResult;
use crate::exec_policy::ExecPolicyManager;
use crate::protocol::Event;
use crate::protocol::Op;
use crate::protocol::Submission;
use crate::shell;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;

pub struct CodexThread {
    codex: Codex,
    rollout_path: PathBuf,
    config: Arc<Config>,
    user_shell: Arc<shell::Shell>,
    exec_policy: ExecPolicyManager,
}

/// Conduit for the bidirectional stream of messages that compose a thread
/// (formerly called a conversation) in Codex.
impl CodexThread {
    pub(crate) fn new(
        codex: Codex,
        rollout_path: PathBuf,
        config: Arc<Config>,
        user_shell: Arc<shell::Shell>,
        exec_policy: ExecPolicyManager,
    ) -> Self {
        Self {
            codex,
            rollout_path,
            config,
            user_shell,
            exec_policy,
        }
    }

    pub async fn submit(&self, op: Op) -> CodexResult<String> {
        self.codex.submit(op).await
    }

    /// Use sparingly: this is intended to be removed soon.
    pub async fn submit_with_id(&self, sub: Submission) -> CodexResult<()> {
        self.codex.submit_with_id(sub).await
    }

    pub async fn next_event(&self) -> CodexResult<Event> {
        self.codex.next_event().await
    }

    pub async fn agent_status(&self) -> AgentStatus {
        self.codex.agent_status().await
    }

    pub(crate) fn subscribe_status(&self) -> watch::Receiver<AgentStatus> {
        self.codex.agent_status.clone()
    }

    pub fn rollout_path(&self) -> PathBuf {
        self.rollout_path.clone()
    }

    pub(crate) fn config(&self) -> Arc<Config> {
        Arc::clone(&self.config)
    }

    pub(crate) fn user_shell(&self) -> Arc<shell::Shell> {
        Arc::clone(&self.user_shell)
    }

    pub(crate) fn exec_policy(&self) -> ExecPolicyManager {
        self.exec_policy.clone()
    }
}

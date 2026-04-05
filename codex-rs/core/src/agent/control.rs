use crate::agent::AgentStatus;
use crate::agent::guards::Guards;
use crate::agent::guards::SpawnedThreadRecord;
use crate::agent::status::is_final;
use crate::error::CodexErr;
use crate::error::Result as CodexResult;
use crate::session_prefix::format_subagent_notification_message;
use crate::shell_snapshot::ShellSnapshot;
use crate::thread_manager::ThreadManagerState;
use codex_protocol::ThreadId;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::user_input::UserInput;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Weak;
use tokio::sync::watch;

/// Control-plane handle for multi-agent operations.
/// `AgentControl` is held by each session (via `SessionServices`). It provides capability to
/// spawn new agents and the inter-agent communication layer.
#[derive(Clone, Default)]
pub(crate) struct AgentControl {
    /// Weak handle back to the global thread registry/state.
    /// This is `Weak` to avoid reference cycles and shadow persistence of the form
    /// `ThreadManagerState -> CodexThread -> Session -> SessionServices -> ThreadManagerState`.
    manager: Weak<ThreadManagerState>,
    state: Arc<Guards>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ListedAgent {
    pub(crate) agent_id: ThreadId,
    pub(crate) parent_id: Option<ThreadId>,
    pub(crate) depth: usize,
    pub(crate) agent_type: crate::agent::AgentRole,
    pub(crate) agent_status: AgentStatus,
    pub(crate) last_task_message: Option<String>,
}

impl AgentControl {
    /// Construct a new `AgentControl` that can spawn/message agents via the given manager state.
    pub(crate) fn new(manager: Weak<ThreadManagerState>) -> Self {
        Self {
            manager,
            ..Default::default()
        }
    }

    /// Spawn a new agent thread and submit the initial prompt.
    pub(crate) async fn spawn_agent(
        &self,
        config: crate::config::Config,
        prompt: String,
        agent_type: crate::agent::AgentRole,
        session_source: Option<SessionSource>,
    ) -> CodexResult<ThreadId> {
        let state = self.upgrade()?;
        let reservation = self.state.reserve_spawn_slot(config.agent_max_threads)?;
        let inherited_shell_snapshot = self
            .inherited_shell_snapshot_for_source(&state, session_source.as_ref())
            .await;
        let inherited_exec_policy = self
            .inherited_exec_policy_for_source(&state, session_source.as_ref(), &config)
            .await;
        let notification_source = session_source.clone();
        let parent_id = thread_spawn_parent_id(session_source.as_ref());
        let child_depth = config.agent_spawn_depth;
        let new_thread = match session_source {
            Some(session_source) => {
                state
                    .spawn_new_thread_with_source(
                        config,
                        self.clone(),
                        session_source,
                        inherited_shell_snapshot,
                        inherited_exec_policy,
                    )
                    .await?
            }
            None => state.spawn_new_thread(config, self.clone()).await?,
        };
        reservation.commit(
            new_thread.thread_id,
            SpawnedThreadRecord {
                parent_id,
                last_task_message: prompt.clone(),
                agent_type,
                depth: child_depth,
            },
        );

        // Notify a new thread has been created. This notification will be processed by clients
        // to subscribe or drain this newly created thread.
        // TODO(jif) add helper for drain
        state.notify_thread_created(new_thread.thread_id);

        self.send_prompt(new_thread.thread_id, prompt).await?;
        self.maybe_start_completion_watcher(new_thread.thread_id, notification_source);

        Ok(new_thread.thread_id)
    }

    /// Send a `user` prompt to an existing agent thread.
    pub(crate) async fn send_prompt(
        &self,
        agent_id: ThreadId,
        prompt: String,
    ) -> CodexResult<String> {
        let state = self.upgrade()?;
        let result = state
            .send_op(
                agent_id,
                Op::UserInput {
                    items: vec![UserInput::Text {
                        text: prompt.clone(),
                        // Plain text conversion has no UI element ranges.
                        text_elements: Vec::new(),
                    }],
                    final_output_json_schema: None,
                },
            )
            .await;
        if result.is_ok() {
            self.state.update_last_task_message(agent_id, prompt);
        }
        if matches!(result, Err(CodexErr::InternalAgentDied)) {
            let _ = state.remove_thread(&agent_id).await;
            self.state.release_spawned_thread(agent_id);
        }
        result
    }

    /// Interrupt the current task for an existing agent thread.
    pub(crate) async fn interrupt_agent(&self, agent_id: ThreadId) -> CodexResult<String> {
        let state = self.upgrade()?;
        state.send_op(agent_id, Op::Interrupt).await
    }

    /// Submit a shutdown request to an existing agent thread.
    pub(crate) async fn shutdown_agent(&self, agent_id: ThreadId) -> CodexResult<String> {
        let state = self.upgrade()?;
        let result = state.send_op(agent_id, Op::Shutdown {}).await;
        let _ = state.remove_thread(&agent_id).await;
        self.state.release_spawned_thread(agent_id);
        result
    }

    #[allow(dead_code)] // Will be used for collab tools.
    /// Fetch the last known status for `agent_id`, returning `NotFound` when unavailable.
    pub(crate) async fn get_status(&self, agent_id: ThreadId) -> AgentStatus {
        let Ok(state) = self.upgrade() else {
            // No agent available if upgrade fails.
            return AgentStatus::NotFound;
        };
        let Ok(thread) = state.get_thread(agent_id).await else {
            return AgentStatus::NotFound;
        };
        thread.agent_status().await
    }

    /// Subscribe to status updates for `agent_id`, yielding the latest value and changes.
    pub(crate) async fn subscribe_status(
        &self,
        agent_id: ThreadId,
    ) -> CodexResult<watch::Receiver<AgentStatus>> {
        let state = self.upgrade()?;
        let thread = state.get_thread(agent_id).await?;
        Ok(thread.subscribe_status())
    }

    pub(crate) async fn list_agents(&self) -> CodexResult<Vec<ListedAgent>> {
        let state = self.upgrade()?;
        let mut live_agents = self.state.live_agents();
        live_agents.sort_by(|left, right| {
            left.1
                .depth
                .cmp(&right.1.depth)
                .then_with(|| {
                    left.1
                        .parent_id
                        .map(|id| id.to_string())
                        .cmp(&right.1.parent_id.map(|id| id.to_string()))
                })
                .then_with(|| left.0.to_string().cmp(&right.0.to_string()))
        });

        let mut agents = Vec::with_capacity(live_agents.len());
        for (agent_id, record) in live_agents {
            let status = match state.get_thread(agent_id).await {
                Ok(thread) => thread.agent_status().await,
                Err(_) => AgentStatus::NotFound,
            };
            agents.push(ListedAgent {
                agent_id,
                parent_id: record.parent_id,
                depth: record.depth,
                agent_type: record.agent_type,
                agent_status: status,
                last_task_message: Some(record.last_task_message),
            });
        }

        Ok(agents)
    }

    fn maybe_start_completion_watcher(
        &self,
        child_thread_id: ThreadId,
        session_source: Option<SessionSource>,
    ) {
        let Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn { parent_thread_id })) =
            session_source
        else {
            return;
        };
        let control = self.clone();
        tokio::spawn(async move {
            let mut status_rx = match control.subscribe_status(child_thread_id).await {
                Ok(rx) => rx,
                Err(_) => return,
            };
            let mut status = status_rx.borrow().clone();
            while !is_final(&status) {
                if status_rx.changed().await.is_err() {
                    status = control.get_status(child_thread_id).await;
                    break;
                }
                status = status_rx.borrow().clone();
            }
            if !is_final(&status) {
                return;
            }

            let Ok(state) = control.upgrade() else {
                return;
            };
            let _ = state
                .send_op(
                    parent_thread_id,
                    Op::InjectSessionPrefix {
                        text: format_subagent_notification_message(
                            &child_thread_id.to_string(),
                            &status,
                        ),
                    },
                )
                .await;
        });
    }

    fn upgrade(&self) -> CodexResult<Arc<ThreadManagerState>> {
        self.manager
            .upgrade()
            .ok_or_else(|| CodexErr::UnsupportedOperation("thread manager dropped".to_string()))
    }

    async fn inherited_shell_snapshot_for_source(
        &self,
        state: &Arc<ThreadManagerState>,
        session_source: Option<&SessionSource>,
    ) -> Option<Arc<ShellSnapshot>> {
        let parent_thread_id = thread_spawn_parent_id(session_source)?;
        let parent_thread = state.get_thread(parent_thread_id).await.ok()?;
        parent_thread.user_shell().shell_snapshot.clone()
    }

    async fn inherited_exec_policy_for_source(
        &self,
        state: &Arc<ThreadManagerState>,
        session_source: Option<&SessionSource>,
        child_config: &crate::config::Config,
    ) -> Option<crate::exec_policy::ExecPolicyManager> {
        let parent_thread_id = thread_spawn_parent_id(session_source)?;
        let parent_thread = state.get_thread(parent_thread_id).await.ok()?;
        let parent_config = parent_thread.config();
        if !crate::exec_policy::child_uses_parent_exec_policy(&parent_config, child_config) {
            return None;
        }
        Some(parent_thread.exec_policy())
    }
}

fn thread_spawn_parent_id(session_source: Option<&SessionSource>) -> Option<ThreadId> {
    let SessionSource::SubAgent(SubAgentSource::ThreadSpawn { parent_thread_id }) = session_source?
    else {
        return None;
    };
    Some(*parent_thread_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodexAuth;
    use crate::CodexThread;
    use crate::ThreadManager;
    use crate::agent::agent_status_from_event;
    use crate::config::Config;
    use crate::config::ConfigBuilder;
    use assert_matches::assert_matches;
    use codex_protocol::protocol::ErrorEvent;
    use codex_protocol::protocol::EventMsg;
    use codex_protocol::protocol::TurnAbortReason;
    use codex_protocol::protocol::TurnAbortedEvent;
    use codex_protocol::protocol::TurnCompleteEvent;
    use codex_protocol::protocol::TurnStartedEvent;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    async fn test_config() -> (TempDir, Config) {
        let home = TempDir::new().expect("create temp dir");
        let config = ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .build()
            .await
            .expect("load default test config");
        (home, config)
    }

    struct AgentControlHarness {
        _home: TempDir,
        config: Config,
        manager: ThreadManager,
        control: AgentControl,
    }

    impl AgentControlHarness {
        async fn new() -> Self {
            let (home, config) = test_config().await;
            let manager = ThreadManager::with_models_provider_and_home(
                CodexAuth::from_api_key("dummy"),
                config.model_provider.clone(),
                config.codex_home.clone(),
            );
            let control = manager.agent_control();
            Self {
                _home: home,
                config,
                manager,
                control,
            }
        }

        async fn start_thread(&self) -> (ThreadId, Arc<CodexThread>) {
            let new_thread = self
                .manager
                .start_thread(self.config.clone())
                .await
                .expect("start thread");
            (new_thread.thread_id, new_thread.thread)
        }
    }

    #[tokio::test]
    async fn send_prompt_errors_when_manager_dropped() {
        let control = AgentControl::default();
        let err = control
            .send_prompt(ThreadId::new(), "hello".to_string())
            .await
            .expect_err("send_prompt should fail without a manager");
        assert_eq!(
            err.to_string(),
            "unsupported operation: thread manager dropped"
        );
    }

    #[tokio::test]
    async fn get_status_returns_not_found_without_manager() {
        let control = AgentControl::default();
        let got = control.get_status(ThreadId::new()).await;
        assert_eq!(got, AgentStatus::NotFound);
    }

    #[tokio::test]
    async fn on_event_updates_status_from_task_started() {
        let status = agent_status_from_event(&EventMsg::TurnStarted(TurnStartedEvent {
            model_context_window: None,
        }));
        assert_eq!(status, Some(AgentStatus::Running));
    }

    #[tokio::test]
    async fn on_event_updates_status_from_task_complete() {
        let status = agent_status_from_event(&EventMsg::TurnComplete(TurnCompleteEvent {
            last_agent_message: Some("done".to_string()),
        }));
        let expected = AgentStatus::Completed(Some("done".to_string()));
        assert_eq!(status, Some(expected));
    }

    #[tokio::test]
    async fn on_event_updates_status_from_error() {
        let status = agent_status_from_event(&EventMsg::Error(ErrorEvent {
            message: "boom".to_string(),
            codex_error_info: None,
        }));

        let expected = AgentStatus::Errored("boom".to_string());
        assert_eq!(status, Some(expected));
    }

    #[tokio::test]
    async fn on_event_updates_status_from_turn_aborted() {
        let status = agent_status_from_event(&EventMsg::TurnAborted(TurnAbortedEvent {
            reason: TurnAbortReason::Interrupted,
        }));

        let expected = AgentStatus::Errored("Interrupted".to_string());
        assert_eq!(status, Some(expected));
    }

    #[tokio::test]
    async fn on_event_updates_status_from_shutdown_complete() {
        let status = agent_status_from_event(&EventMsg::ShutdownComplete);
        assert_eq!(status, Some(AgentStatus::Shutdown));
    }

    #[tokio::test]
    async fn spawn_agent_errors_when_manager_dropped() {
        let control = AgentControl::default();
        let (_home, config) = test_config().await;
        let err = control
            .spawn_agent(
                config,
                "hello".to_string(),
                crate::agent::AgentRole::Default,
                None,
            )
            .await
            .expect_err("spawn_agent should fail without a manager");
        assert_eq!(
            err.to_string(),
            "unsupported operation: thread manager dropped"
        );
    }

    #[tokio::test]
    async fn send_prompt_errors_when_thread_missing() {
        let harness = AgentControlHarness::new().await;
        let thread_id = ThreadId::new();
        let err = harness
            .control
            .send_prompt(thread_id, "hello".to_string())
            .await
            .expect_err("send_prompt should fail for missing thread");
        assert_matches!(err, CodexErr::ThreadNotFound(id) if id == thread_id);
    }

    #[tokio::test]
    async fn get_status_returns_not_found_for_missing_thread() {
        let harness = AgentControlHarness::new().await;
        let status = harness.control.get_status(ThreadId::new()).await;
        assert_eq!(status, AgentStatus::NotFound);
    }

    #[tokio::test]
    async fn get_status_returns_pending_init_for_new_thread() {
        let harness = AgentControlHarness::new().await;
        let (thread_id, _) = harness.start_thread().await;
        let status = harness.control.get_status(thread_id).await;
        assert_eq!(status, AgentStatus::PendingInit);
    }

    #[tokio::test]
    async fn subscribe_status_errors_for_missing_thread() {
        let harness = AgentControlHarness::new().await;
        let thread_id = ThreadId::new();
        let err = harness
            .control
            .subscribe_status(thread_id)
            .await
            .expect_err("subscribe_status should fail for missing thread");
        assert_matches!(err, CodexErr::ThreadNotFound(id) if id == thread_id);
    }

    #[tokio::test]
    async fn subscribe_status_updates_on_shutdown() {
        let harness = AgentControlHarness::new().await;
        let (thread_id, thread) = harness.start_thread().await;
        let mut status_rx = harness
            .control
            .subscribe_status(thread_id)
            .await
            .expect("subscribe_status should succeed");
        assert_eq!(status_rx.borrow().clone(), AgentStatus::PendingInit);

        let _ = thread
            .submit(Op::Shutdown {})
            .await
            .expect("shutdown should submit");

        let _ = status_rx.changed().await;
        assert_eq!(status_rx.borrow().clone(), AgentStatus::Shutdown);
    }

    #[tokio::test]
    async fn send_prompt_submits_user_message() {
        let harness = AgentControlHarness::new().await;
        let (thread_id, _thread) = harness.start_thread().await;

        let submission_id = harness
            .control
            .send_prompt(thread_id, "hello from tests".to_string())
            .await
            .expect("send_prompt should succeed");
        assert!(!submission_id.is_empty());
        let expected = (
            thread_id,
            Op::UserInput {
                items: vec![UserInput::Text {
                    text: "hello from tests".to_string(),
                    text_elements: Vec::new(),
                }],
                final_output_json_schema: None,
            },
        );
        let captured = harness
            .manager
            .captured_ops()
            .into_iter()
            .find(|entry| *entry == expected);
        assert_eq!(captured, Some(expected));
    }

    #[tokio::test]
    async fn spawn_agent_creates_thread_and_sends_prompt() {
        let harness = AgentControlHarness::new().await;
        let thread_id = harness
            .control
            .spawn_agent(
                harness.config.clone(),
                "spawned".to_string(),
                crate::agent::AgentRole::Default,
                None,
            )
            .await
            .expect("spawn_agent should succeed");
        let _thread = harness
            .manager
            .get_thread(thread_id)
            .await
            .expect("thread should be registered");
        let expected = (
            thread_id,
            Op::UserInput {
                items: vec![UserInput::Text {
                    text: "spawned".to_string(),
                    text_elements: Vec::new(),
                }],
                final_output_json_schema: None,
            },
        );
        let captured = harness
            .manager
            .captured_ops()
            .into_iter()
            .find(|entry| *entry == expected);
        assert_eq!(captured, Some(expected));
    }

    #[tokio::test]
    async fn list_agents_tracks_spawned_threads_and_last_task_messages() {
        let harness = AgentControlHarness::new().await;
        let (parent_thread_id, _parent_thread) = harness.start_thread().await;
        let mut child_config = harness.config.clone();
        child_config.agent_spawn_depth = 1;
        let child_thread_id = harness
            .control
            .spawn_agent(
                child_config,
                "initial task".to_string(),
                crate::agent::AgentRole::Worker,
                Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id,
                })),
            )
            .await
            .expect("spawn child");

        harness
            .control
            .send_prompt(child_thread_id, "follow-up task".to_string())
            .await
            .expect("send follow-up");

        let agents = harness.control.list_agents().await.expect("list agents");
        assert_eq!(
            agents,
            vec![ListedAgent {
                agent_id: child_thread_id,
                parent_id: Some(parent_thread_id),
                depth: 1,
                agent_type: crate::agent::AgentRole::Worker,
                agent_status: AgentStatus::PendingInit,
                last_task_message: Some("follow-up task".to_string()),
            }]
        );
    }

    #[tokio::test]
    async fn thread_spawn_child_inherits_parent_shell_snapshot() {
        let harness = AgentControlHarness::new().await;
        let mut config = harness.config.clone();
        config
            .features
            .enable(crate::features::Feature::ShellSnapshot);
        let mut child_config = config.clone();
        child_config.agent_spawn_depth = 1;

        let parent = harness
            .manager
            .start_thread(config.clone())
            .await
            .expect("start parent");
        let child_thread_id = harness
            .control
            .spawn_agent(
                child_config,
                "spawned".to_string(),
                crate::agent::AgentRole::Default,
                Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id: parent.thread_id,
                })),
            )
            .await
            .expect("spawn child");

        let parent_snapshot = parent
            .thread
            .user_shell()
            .shell_snapshot
            .as_ref()
            .map(|snapshot| snapshot.path.clone());
        let child = harness
            .manager
            .get_thread(child_thread_id)
            .await
            .expect("get child");
        let child_snapshot = child
            .user_shell()
            .shell_snapshot
            .as_ref()
            .map(|snapshot| snapshot.path.clone());

        assert_eq!(child_snapshot, parent_snapshot);
        assert_eq!(child_snapshot.is_some(), true);
    }

    #[tokio::test]
    async fn thread_spawn_child_shares_parent_exec_policy_when_layers_match() {
        let harness = AgentControlHarness::new().await;
        let (parent_thread_id, parent_thread) = harness.start_thread().await;
        let mut child_config = harness.config.clone();
        child_config.agent_spawn_depth = 1;
        let child_thread_id = harness
            .control
            .spawn_agent(
                child_config,
                "spawned".to_string(),
                crate::agent::AgentRole::Default,
                Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id,
                })),
            )
            .await
            .expect("spawn child");
        let child_thread = harness
            .manager
            .get_thread(child_thread_id)
            .await
            .expect("get child");

        let parent_policy = parent_thread.exec_policy().current();
        let child_policy = child_thread.exec_policy().current();
        assert!(Arc::ptr_eq(&parent_policy, &child_policy));
    }
}

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use chrono::DateTime;
use chrono::Local;
use codex_core::protocol::AgentStatus;
use codex_core::protocol::CollabAgentInteractionBeginEvent;
use codex_core::protocol::CollabAgentInteractionEndEvent;
use codex_core::protocol::CollabAgentSpawnBeginEvent;
use codex_core::protocol::CollabAgentSpawnEndEvent;
use codex_core::protocol::CollabCloseBeginEvent;
use codex_core::protocol::CollabCloseEndEvent;
use codex_core::protocol::CollabWaitingBeginEvent;
use codex_core::protocol::CollabWaitingEndEvent;
use codex_core::protocol::Event;
use codex_core::protocol::EventMsg;
use codex_core::protocol::ExecCommandBeginEvent;
use codex_core::protocol::ExecCommandEndEvent;
use codex_core::protocol::McpToolCallBeginEvent;
use codex_core::protocol::McpToolCallEndEvent;
use codex_core::protocol::PatchApplyBeginEvent;
use codex_core::protocol::PatchApplyEndEvent;
use codex_core::protocol::SessionConfiguredEvent;
use codex_core::protocol::TokenCountEvent;
use codex_core::protocol::TokenUsageInfo;
use codex_core::protocol::TurnAbortedEvent;
use codex_core::protocol::TurnCompleteEvent;
use codex_core::protocol::TurnStartedEvent;
use codex_core::protocol::WebSearchBeginEvent;
use codex_core::protocol::WebSearchEndEvent;
use codex_protocol::ThreadId;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;

use crate::render::renderable::Renderable;
use crate::status::format_tokens_compact;
use crate::text_formatting::truncate_text;

const MESSAGE_PREVIEW_GRAPHEMES: usize = 200;

#[derive(Debug, Clone, Default)]
pub(crate) struct AgentsDashboard {
    revision: u64,
    agents: HashMap<ThreadId, AgentSnapshot>,
}

impl AgentsDashboard {
    pub(crate) fn track_thread_created(&mut self, thread_id: ThreadId) {
        self.ensure_agent(thread_id);
    }

    pub(crate) fn on_event(&mut self, thread_id: ThreadId, event: &Event) {
        let now = Local::now();
        {
            let agent = self.ensure_agent(thread_id);
            agent.last_event_at = now;
        }

        match &event.msg {
            EventMsg::SessionConfigured(ev) => {
                self.ensure_agent(thread_id).on_session_configured(ev, now)
            }
            EventMsg::TokenCount(ev) => self.ensure_agent(thread_id).on_token_count(ev, now),
            EventMsg::TurnStarted(ev) => self.ensure_agent(thread_id).on_turn_started(ev, now),
            EventMsg::TurnComplete(ev) => self.ensure_agent(thread_id).on_turn_complete(ev, now),
            EventMsg::TurnAborted(ev) => self.ensure_agent(thread_id).on_turn_aborted(ev, now),
            EventMsg::Error(ev) => self
                .ensure_agent(thread_id)
                .on_error(ev.message.as_str(), now),
            EventMsg::Warning(ev) => self
                .ensure_agent(thread_id)
                .on_warning(ev.message.as_str(), now),
            EventMsg::ShutdownComplete => self.ensure_agent(thread_id).on_shutdown_complete(now),
            EventMsg::UserMessage(ev) => self
                .ensure_agent(thread_id)
                .on_user_message(ev.message.as_str(), now),
            EventMsg::AgentMessage(ev) => self
                .ensure_agent(thread_id)
                .on_agent_message(ev.message.as_str(), now),
            EventMsg::ExecCommandBegin(ev) => {
                self.ensure_agent(thread_id).on_exec_command_begin(ev, now)
            }
            EventMsg::ExecCommandEnd(ev) => {
                self.ensure_agent(thread_id).on_exec_command_end(ev, now)
            }
            EventMsg::McpToolCallBegin(ev) => {
                self.ensure_agent(thread_id).on_mcp_tool_call_begin(ev, now)
            }
            EventMsg::McpToolCallEnd(ev) => {
                self.ensure_agent(thread_id).on_mcp_tool_call_end(ev, now)
            }
            EventMsg::WebSearchBegin(ev) => {
                self.ensure_agent(thread_id).on_web_search_begin(ev, now)
            }
            EventMsg::WebSearchEnd(ev) => self.ensure_agent(thread_id).on_web_search_end(ev, now),
            EventMsg::PatchApplyBegin(ev) => {
                self.ensure_agent(thread_id).on_patch_apply_begin(ev, now)
            }
            EventMsg::PatchApplyEnd(ev) => self.ensure_agent(thread_id).on_patch_apply_end(ev, now),
            EventMsg::ExecApprovalRequest(_) | EventMsg::ApplyPatchApprovalRequest(_) => {
                self.ensure_agent(thread_id).on_waiting_for_approval(now);
            }
            EventMsg::CollabAgentSpawnBegin(ev) => {
                self.ensure_agent(thread_id).on_collab_spawn_begin(ev, now)
            }
            EventMsg::CollabAgentSpawnEnd(ev) => self.on_collab_spawn_end(ev),
            EventMsg::CollabAgentInteractionBegin(ev) => self
                .ensure_agent(thread_id)
                .on_collab_interaction_begin(ev, now),
            EventMsg::CollabAgentInteractionEnd(ev) => {
                self.ensure_agent(thread_id)
                    .on_collab_interaction_end(ev, now);

                let receiver_id = ev.receiver_thread_id;
                let receiver = self.ensure_agent(receiver_id);
                receiver.last_event_at = now;
                receiver.status = ev.status.clone();
                receiver.last_action = Some(format!("received input from {thread_id}"));
                receiver.last_user_message = Some(preview_message(ev.prompt.as_str()));
            }
            EventMsg::CollabWaitingBegin(ev) => self
                .ensure_agent(thread_id)
                .on_collab_waiting_begin(ev, now),
            EventMsg::CollabWaitingEnd(ev) => {
                self.ensure_agent(thread_id).on_collab_waiting_end(ev, now);

                for (receiver_id, status) in &ev.statuses {
                    let receiver = self.ensure_agent(*receiver_id);
                    receiver.last_event_at = now;
                    receiver.status = status.clone();
                }
            }
            EventMsg::CollabCloseBegin(ev) => {
                self.ensure_agent(thread_id).on_collab_close_begin(ev, now)
            }
            EventMsg::CollabCloseEnd(ev) => {
                self.ensure_agent(thread_id).on_collab_close_end(ev, now);

                let receiver_id = ev.receiver_thread_id;
                let receiver = self.ensure_agent(receiver_id);
                receiver.last_event_at = now;
                receiver.status = ev.status.clone();
                receiver.last_action = Some(format!("closed by {thread_id}"));
            }
            _ => {}
        }

        self.bump_revision();
    }

    pub(crate) fn snapshot(&self) -> AgentsDashboardSnapshot {
        let mut agents: Vec<AgentSnapshot> = self.agents.values().cloned().collect();
        agents.sort_by_key(|agent| std::cmp::Reverse(agent.last_event_at));
        AgentsDashboardSnapshot {
            revision: self.revision,
            captured_at: Local::now(),
            agents,
        }
    }

    fn ensure_agent(&mut self, thread_id: ThreadId) -> &mut AgentSnapshot {
        self.agents
            .entry(thread_id)
            .or_insert_with(|| AgentSnapshot::new(thread_id, Local::now()))
    }

    pub(crate) fn on_collab_spawn_end(&mut self, ev: &CollabAgentSpawnEndEvent) {
        let now = Local::now();
        let prompt = preview_message(ev.prompt.as_str());
        if let Some(child_id) = ev.new_thread_id {
            let child = self.ensure_agent(child_id);
            child.parent_thread_id = Some(ev.sender_thread_id);
            child.last_event_at = now;
            child.status = ev.status.clone();
            if child.task.is_none() && !ev.prompt.trim().is_empty() {
                child.task = Some(prompt.clone());
            }
            child.last_action = Some(format!("spawned by {}", ev.sender_thread_id));

            if let Some(sender) = self.agents.get_mut(&ev.sender_thread_id) {
                sender.last_event_at = now;
                if ev.prompt.trim().is_empty() {
                    sender.last_action = Some(format!("spawned agent {child_id}"));
                } else {
                    sender.last_action = Some(format!("spawned agent {child_id}: {prompt}"));
                }
            }
        } else if let Some(sender) = self.agents.get_mut(&ev.sender_thread_id) {
            sender.last_event_at = now;
            sender.last_action = Some(format!("spawn failed: {:?}", ev.status));
        }

        self.bump_revision();
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AgentsDashboardSnapshot {
    revision: u64,
    captured_at: DateTime<Local>,
    agents: Vec<AgentSnapshot>,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentSnapshot {
    thread_id: ThreadId,
    parent_thread_id: Option<ThreadId>,
    created_at: DateTime<Local>,
    last_event_at: DateTime<Local>,
    status: AgentStatus,
    task: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffortConfig>,
    model_provider_id: Option<String>,
    rollout_path: Option<std::path::PathBuf>,
    cwd: Option<std::path::PathBuf>,
    token_info: Option<TokenUsageInfo>,
    model_context_window: Option<i64>,
    waiting_for_approval: bool,
    last_action: Option<String>,
    last_user_message: Option<String>,
    last_agent_message: Option<String>,
    last_warning: Option<String>,
    last_error: Option<String>,
}

impl AgentSnapshot {
    fn new(thread_id: ThreadId, now: DateTime<Local>) -> Self {
        Self {
            thread_id,
            parent_thread_id: None,
            created_at: now,
            last_event_at: now,
            status: AgentStatus::PendingInit,
            task: None,
            model: None,
            reasoning_effort: None,
            model_provider_id: None,
            rollout_path: None,
            cwd: None,
            token_info: None,
            model_context_window: None,
            waiting_for_approval: false,
            last_action: None,
            last_user_message: None,
            last_agent_message: None,
            last_warning: None,
            last_error: None,
        }
    }

    fn on_session_configured(&mut self, ev: &SessionConfiguredEvent, now: DateTime<Local>) {
        self.model = Some(ev.model.clone());
        self.reasoning_effort = ev.reasoning_effort;
        self.model_provider_id = Some(ev.model_provider_id.clone());
        self.rollout_path = Some(ev.rollout_path.clone());
        self.cwd = Some(ev.cwd.clone());
        self.last_event_at = now;
        self.last_action = Some("session configured".to_string());
    }

    fn on_token_count(&mut self, ev: &TokenCountEvent, now: DateTime<Local>) {
        self.token_info = ev.info.clone();
        if let Some(info) = ev.info.as_ref()
            && let Some(window) = info.model_context_window
        {
            self.model_context_window = Some(window);
        }
        self.last_event_at = now;
    }

    fn on_turn_started(&mut self, ev: &TurnStartedEvent, now: DateTime<Local>) {
        self.status = AgentStatus::Running;
        self.waiting_for_approval = false;
        if let Some(window) = ev.model_context_window {
            self.model_context_window = Some(window);
        }
        self.last_event_at = now;
    }

    fn on_turn_complete(&mut self, ev: &TurnCompleteEvent, now: DateTime<Local>) {
        self.status = AgentStatus::Completed(ev.last_agent_message.clone());
        self.waiting_for_approval = false;
        if let Some(message) = ev.last_agent_message.as_deref() {
            self.last_agent_message = Some(preview_message(message));
        }
        self.last_event_at = now;
    }

    fn on_turn_aborted(&mut self, ev: &TurnAbortedEvent, now: DateTime<Local>) {
        self.status = AgentStatus::Errored(format!("{:?}", ev.reason));
        self.waiting_for_approval = false;
        self.last_error = Some(format!("{:?}", ev.reason));
        self.last_event_at = now;
    }

    fn on_error(&mut self, message: &str, now: DateTime<Local>) {
        self.status = AgentStatus::Errored(message.to_string());
        self.waiting_for_approval = false;
        self.last_error = Some(preview_message(message));
        self.last_event_at = now;
    }

    fn on_warning(&mut self, message: &str, now: DateTime<Local>) {
        self.last_warning = Some(preview_message(message));
        self.last_event_at = now;
    }

    fn on_shutdown_complete(&mut self, now: DateTime<Local>) {
        self.status = AgentStatus::Shutdown;
        self.waiting_for_approval = false;
        self.last_event_at = now;
        self.last_action = Some("shutdown".to_string());
    }

    fn on_user_message(&mut self, message: &str, now: DateTime<Local>) {
        self.last_user_message = Some(preview_message(message));
        self.last_event_at = now;
    }

    fn on_agent_message(&mut self, message: &str, now: DateTime<Local>) {
        self.last_agent_message = Some(preview_message(message));
        self.last_event_at = now;
    }

    fn on_waiting_for_approval(&mut self, now: DateTime<Local>) {
        self.waiting_for_approval = true;
        self.last_action = Some("waiting for approval".to_string());
        self.last_event_at = now;
    }

    fn on_exec_command_begin(&mut self, ev: &ExecCommandBeginEvent, now: DateTime<Local>) {
        self.waiting_for_approval = false;
        self.last_action = Some(format!("shell: {}", join_cmd(ev.command.as_slice())));
        self.last_event_at = now;
    }

    fn on_exec_command_end(&mut self, ev: &ExecCommandEndEvent, now: DateTime<Local>) {
        self.last_action = Some(format!(
            "shell: {} (exit {})",
            join_cmd(ev.command.as_slice()),
            ev.exit_code
        ));
        self.last_event_at = now;
    }

    fn on_mcp_tool_call_begin(&mut self, ev: &McpToolCallBeginEvent, now: DateTime<Local>) {
        self.waiting_for_approval = false;
        self.last_action = Some(format!(
            "mcp: {}/{}",
            ev.invocation.server, ev.invocation.tool
        ));
        self.last_event_at = now;
    }

    fn on_mcp_tool_call_end(&mut self, ev: &McpToolCallEndEvent, now: DateTime<Local>) {
        let status = if ev.is_success() { "ok" } else { "error" };
        self.last_action = Some(format!(
            "mcp: {}/{} ({status})",
            ev.invocation.server, ev.invocation.tool
        ));
        self.last_event_at = now;
    }

    fn on_web_search_begin(&mut self, _ev: &WebSearchBeginEvent, now: DateTime<Local>) {
        self.waiting_for_approval = false;
        self.last_action = Some("web_search: running".to_string());
        self.last_event_at = now;
    }

    fn on_web_search_end(&mut self, ev: &WebSearchEndEvent, now: DateTime<Local>) {
        self.last_action = Some(format!(
            "web_search: {}",
            preview_message(ev.query.as_str())
        ));
        self.last_event_at = now;
    }

    fn on_patch_apply_begin(&mut self, ev: &PatchApplyBeginEvent, now: DateTime<Local>) {
        self.waiting_for_approval = false;
        self.last_action = Some(format!("apply_patch: {} file(s)", ev.changes.len()));
        self.last_event_at = now;
    }

    fn on_patch_apply_end(&mut self, ev: &PatchApplyEndEvent, now: DateTime<Local>) {
        let outcome = if ev.success { "ok" } else { "error" };
        self.last_action = Some(format!(
            "apply_patch: {} file(s) ({outcome})",
            ev.changes.len()
        ));
        self.last_event_at = now;
    }

    fn on_collab_spawn_begin(&mut self, ev: &CollabAgentSpawnBeginEvent, now: DateTime<Local>) {
        let prompt = preview_message(ev.prompt.as_str());
        self.last_action = Some(format!("spawning agent: {prompt}"));
        self.last_event_at = now;
    }

    fn on_collab_interaction_begin(
        &mut self,
        ev: &CollabAgentInteractionBeginEvent,
        now: DateTime<Local>,
    ) {
        let receiver_id = ev.receiver_thread_id;
        let prompt = preview_message(ev.prompt.as_str());
        self.last_action = Some(format!("sending input to {receiver_id}: {prompt}"));
        self.last_event_at = now;
    }

    fn on_collab_interaction_end(
        &mut self,
        ev: &CollabAgentInteractionEndEvent,
        now: DateTime<Local>,
    ) {
        let receiver_id = ev.receiver_thread_id;
        self.last_action = Some(format!("sent input to {receiver_id}"));
        self.last_event_at = now;
    }

    fn on_collab_waiting_begin(&mut self, ev: &CollabWaitingBeginEvent, now: DateTime<Local>) {
        self.last_action = Some(format!(
            "waiting on {} agent(s)",
            ev.receiver_thread_ids.len()
        ));
        self.last_event_at = now;
    }

    fn on_collab_waiting_end(&mut self, ev: &CollabWaitingEndEvent, now: DateTime<Local>) {
        self.last_action = Some(format!("waited on {} agent(s)", ev.statuses.len()));
        self.last_event_at = now;
    }

    fn on_collab_close_begin(&mut self, ev: &CollabCloseBeginEvent, now: DateTime<Local>) {
        let receiver_id = ev.receiver_thread_id;
        self.last_action = Some(format!("closing agent {receiver_id}"));
        self.last_event_at = now;
    }

    fn on_collab_close_end(&mut self, ev: &CollabCloseEndEvent, now: DateTime<Local>) {
        let receiver_id = ev.receiver_thread_id;
        self.last_action = Some(format!("closed agent {receiver_id}"));
        self.last_event_at = now;
    }
}

fn join_cmd(parts: &[String]) -> String {
    if parts.is_empty() {
        "<empty>".to_string()
    } else {
        parts.join(" ")
    }
}

fn preview_message(message: &str) -> String {
    let message = message.trim();
    if message.is_empty() {
        "<empty>".to_string()
    } else {
        truncate_text(message, MESSAGE_PREVIEW_GRAPHEMES)
    }
}

pub(crate) struct AgentsDashboardRenderable {
    dashboard: Arc<Mutex<AgentsDashboard>>,
    collab_enabled: bool,
    cache: RefCell<AgentsDashboardRenderCache>,
}

#[derive(Default)]
struct AgentsDashboardRenderCache {
    revision: u64,
    width: u16,
    lines: Vec<Line<'static>>,
}

impl AgentsDashboardRenderable {
    pub(crate) fn new(dashboard: Arc<Mutex<AgentsDashboard>>, collab_enabled: bool) -> Self {
        Self {
            dashboard,
            collab_enabled,
            cache: RefCell::new(AgentsDashboardRenderCache::default()),
        }
    }

    fn ensure_cache(&self, width: u16) {
        let mut cache = self.cache.borrow_mut();
        let snapshot = {
            let guard = match self.dashboard.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            if cache.revision == guard.revision && cache.width == width {
                return;
            }
            guard.snapshot()
        };

        cache.revision = snapshot.revision;
        cache.width = width;
        cache.lines = render_dashboard_lines(&snapshot, self.collab_enabled);
    }
}

impl Renderable for AgentsDashboardRenderable {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.ensure_cache(area.width);
        let cache = self.cache.borrow();
        Paragraph::new(Text::from(cache.lines.clone())).render_ref(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.ensure_cache(width);
        let cache = self.cache.borrow();
        u16::try_from(cache.lines.len()).unwrap_or(u16::MAX)
    }
}

fn render_dashboard_lines(
    snapshot: &AgentsDashboardSnapshot,
    collab_enabled: bool,
) -> Vec<Line<'static>> {
    let mut counts = AgentCounts::default();
    for agent in &snapshot.agents {
        counts.observe(&agent.status);
    }
    let spawned_count = snapshot
        .agents
        .iter()
        .filter(|agent| agent.parent_thread_id.is_some())
        .count();

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut header_spans: Vec<ratatui::text::Span<'static>> = Vec::new();
    header_spans.push("Agents: ".bold());
    header_spans.push(format!("{}", snapshot.agents.len()).bold());
    header_spans.push("  ".into());
    header_spans.push(format!("{spawned_count} spawned").dim());
    header_spans.push("  ".into());
    header_spans.extend(counts.summary_spans());
    header_spans.push("  ".into());
    header_spans.push(format!("updated {}", snapshot.captured_at.format("%H:%M:%S")).dim());
    lines.push(Line::from(header_spans));
    lines.push(Line::from(""));

    if snapshot.agents.is_empty() {
        lines.push(Line::from("No spawned agents yet.".dim()));
        if collab_enabled {
            lines.push(Line::from(
                "Tip: ask Codex to use spawn_agent (don't simulate).".dim(),
            ));
        } else {
            lines.push(Line::from(
                "Tip: enable sub-agent tools with --enable collab.".dim(),
            ));
        }
        return lines;
    }

    if spawned_count == 0 {
        lines.push(Line::from("No spawned agents yet.".dim()));
        if collab_enabled {
            lines.push(Line::from(
                "Tip: ask Codex to use spawn_agent (don't simulate).".dim(),
            ));
        } else {
            lines.push(Line::from(
                "Tip: enable sub-agent tools with --enable collab.".dim(),
            ));
        }
        lines.push(Line::from(""));
    }

    for agent in &snapshot.agents {
        lines.extend(render_agent_block(agent));
        lines.push(Line::from(""));
    }

    // Trim final empty line so the pager doesn't render an extra blank tail.
    while matches!(lines.last(), Some(line) if line.spans.is_empty()) {
        lines.pop();
    }
    lines
}

#[derive(Default)]
struct AgentCounts {
    pending: usize,
    running: usize,
    completed: usize,
    errored: usize,
    shutdown: usize,
    not_found: usize,
}

impl AgentCounts {
    fn observe(&mut self, status: &AgentStatus) {
        match status {
            AgentStatus::PendingInit => self.pending += 1,
            AgentStatus::Running => self.running += 1,
            AgentStatus::Completed(_) => self.completed += 1,
            AgentStatus::Errored(_) => self.errored += 1,
            AgentStatus::Shutdown => self.shutdown += 1,
            AgentStatus::NotFound => self.not_found += 1,
        }
    }

    fn summary_spans(&self) -> Vec<ratatui::text::Span<'static>> {
        let mut parts: Vec<ratatui::text::Span<'static>> = Vec::new();
        let mut push = |span: ratatui::text::Span<'static>| {
            if !parts.is_empty() {
                parts.push(", ".dim());
            }
            parts.push(span);
        };

        if self.running > 0 {
            push(format!("{} running", self.running).green());
        }
        if self.pending > 0 {
            push(format!("{} pending", self.pending).cyan());
        }
        if self.completed > 0 {
            push(format!("{} done", self.completed).cyan());
        }
        if self.errored > 0 {
            push(format!("{} error", self.errored).red());
        }
        if self.shutdown > 0 {
            push(format!("{} shutdown", self.shutdown).dim());
        }
        if self.not_found > 0 {
            push(format!("{} missing", self.not_found).dim());
        }

        if parts.is_empty() {
            return vec!["idle".dim()];
        }
        parts
    }
}

fn render_agent_block(agent: &AgentSnapshot) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let status_span = match &agent.status {
        AgentStatus::PendingInit => "pending".cyan(),
        AgentStatus::Running => "running".green(),
        AgentStatus::Completed(_) => "done".cyan(),
        AgentStatus::Errored(_) => "error".red(),
        AgentStatus::Shutdown => "shutdown".dim(),
        AgentStatus::NotFound => "missing".dim(),
    };

    let mut header = vec![
        "• ".dim(),
        agent.thread_id.to_string().bold(),
        "  ".into(),
        status_span,
    ];
    if let Some(parent) = agent.parent_thread_id {
        header.push("  ".into());
        header.push(format!("parent: {parent}").dim());
    }
    lines.push(header.into());

    if let Some(model) = agent.model.as_deref() {
        let effort = agent
            .reasoning_effort
            .map(format_reasoning_effort)
            .unwrap_or_else(|| "default".to_string());
        lines.push(Line::from(vec![
            "  model: ".dim(),
            model.to_string().bold(),
            " ".into(),
            format!("({effort})").dim(),
        ]));
    }

    if let Some(task) = agent.task.as_deref() {
        lines.push(Line::from(vec!["  task: ".dim(), task.to_string().into()]));
    }

    if let Some(ctx) = render_context_window_line(agent) {
        lines.push(ctx);
    }

    if agent.waiting_for_approval {
        lines.push(Line::from(vec![
            "  approvals: ".dim(),
            "waiting".cyan().bold(),
        ]));
    }

    if let Some(action) = agent.last_action.as_deref() {
        let action = preview_message(action);
        let action = action.trim();
        let mut spans: Vec<ratatui::text::Span<'static>> = vec!["  doing: ".dim()];

        if let Some(rest) = action.strip_prefix("shell:") {
            spans.push("shell".cyan().bold());
            spans.push(":".dim());
            spans.push(rest.to_string().into());
        } else if let Some(rest) = action.strip_prefix("mcp:") {
            spans.push("mcp".magenta().bold());
            spans.push(":".dim());
            spans.push(rest.to_string().into());
        } else if let Some(rest) = action.strip_prefix("apply_patch:") {
            if rest.contains("(ok)") {
                spans.push("apply_patch".green().bold());
            } else if rest.contains("(error)") {
                spans.push("apply_patch".red().bold());
            } else {
                spans.push("apply_patch".cyan().bold());
            }
            spans.push(":".dim());
            spans.push(rest.to_string().into());
        } else if let Some(rest) = action.strip_prefix("web_search:") {
            spans.push("web_search".cyan().bold());
            spans.push(":".dim());
            spans.push(rest.to_string().into());
        } else {
            spans.push(action.to_string().into());
        }

        lines.push(Line::from(spans));
    }

    if let Some(message) = agent.last_user_message.as_deref() {
        lines.push(Line::from(vec![
            "  last user: ".dim(),
            message.to_string().into(),
        ]));
    }

    if let Some(message) = agent.last_agent_message.as_deref() {
        lines.push(Line::from(vec![
            "  last assistant: ".dim(),
            message.to_string().into(),
        ]));
    }

    if let Some(message) = agent.last_warning.as_deref() {
        lines.push(Line::from(vec![
            "  warning: ".dim(),
            message.to_string().cyan(),
        ]));
    }

    if let Some(message) = agent.last_error.as_deref() {
        lines.push(Line::from(vec![
            "  error: ".dim(),
            message.to_string().red(),
        ]));
    }

    if let Some(path) = agent.rollout_path.as_ref() {
        lines.push(Line::from(vec![
            "  rollout: ".dim(),
            path.display().to_string().dim(),
        ]));
    }

    lines.push(
        Line::from(format!(
            "  updated: {} (created {})",
            agent.last_event_at.format("%H:%M:%S"),
            agent.created_at.format("%H:%M:%S"),
        ))
        .dim(),
    );

    lines
}

fn render_context_window_line(agent: &AgentSnapshot) -> Option<Line<'static>> {
    let window = agent.model_context_window.or(agent
        .token_info
        .as_ref()
        .and_then(|info| info.model_context_window))?;

    let Some(info) = agent.token_info.as_ref() else {
        return Some(
            Line::from(format!(
                "  context: usage pending (max {})",
                format_tokens_compact(window),
            ))
            .dim(),
        );
    };

    let usage = &info.last_token_usage;
    let used = usage.tokens_in_context_window();
    let left = usage.percent_of_context_window_remaining(window);
    let left_span: ratatui::text::Span<'static> = if left <= 10 {
        format!("{left}%").red()
    } else if left <= 25 {
        format!("{left}%").cyan()
    } else {
        format!("{left}%").green()
    };
    Some(Line::from(vec![
        "  context: ".dim(),
        left_span,
        " left ".dim(),
        format!(
            "({} used / {})",
            format_tokens_compact(used),
            format_tokens_compact(window),
        )
        .dim(),
    ]))
}

fn format_reasoning_effort(effort: ReasoningEffortConfig) -> String {
    match effort {
        ReasoningEffortConfig::Minimal => "minimal".to_string(),
        ReasoningEffortConfig::Low => "low".to_string(),
        ReasoningEffortConfig::Medium => "medium".to_string(),
        ReasoningEffortConfig::High => "high".to_string(),
        ReasoningEffortConfig::XHigh => "xhigh".to_string(),
        ReasoningEffortConfig::None => "default".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use codex_core::protocol::TokenUsage;
    use insta::assert_snapshot;

    fn lines_to_plain_text(lines: &[Line<'static>]) -> String {
        let mut out = String::new();
        for (idx, line) in lines.iter().enumerate() {
            for span in &line.spans {
                out.push_str(span.content.as_ref());
            }
            if idx + 1 < lines.len() {
                out.push('\n');
            }
        }
        out
    }

    #[test]
    fn agents_dashboard_snapshot() {
        let captured_at = chrono::Local
            .with_ymd_and_hms(2026, 4, 2, 12, 0, 0)
            .single()
            .expect("timestamp");
        let created_at = chrono::Local
            .with_ymd_and_hms(2026, 4, 2, 11, 59, 0)
            .single()
            .expect("timestamp");
        let updated_at = chrono::Local
            .with_ymd_and_hms(2026, 4, 2, 12, 0, 1)
            .single()
            .expect("timestamp");

        let agent1_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000001").expect("thread id");
        let agent2_id =
            ThreadId::from_string("00000000-0000-0000-0000-000000000002").expect("thread id");

        let agent1 = AgentSnapshot {
            thread_id: agent1_id,
            parent_thread_id: None,
            created_at,
            last_event_at: updated_at,
            status: AgentStatus::Running,
            task: Some("find and fix the failing test".to_string()),
            model: Some("gpt-5.2".to_string()),
            reasoning_effort: Some(ReasoningEffortConfig::XHigh),
            model_provider_id: Some("openai".to_string()),
            rollout_path: None,
            cwd: None,
            token_info: Some(TokenUsageInfo {
                total_token_usage: TokenUsage::default(),
                last_token_usage: TokenUsage {
                    total_tokens: 40_000,
                    ..TokenUsage::default()
                },
                model_context_window: Some(128_000),
            }),
            model_context_window: None,
            waiting_for_approval: true,
            last_action: Some("shell: cargo test -p codex-tui".to_string()),
            last_user_message: Some("please investigate".to_string()),
            last_agent_message: Some("working on it".to_string()),
            last_warning: None,
            last_error: None,
        };

        let agent2 = AgentSnapshot {
            thread_id: agent2_id,
            parent_thread_id: Some(agent1_id),
            created_at,
            last_event_at: created_at,
            status: AgentStatus::Completed(Some("done".to_string())),
            task: None,
            model: Some("gpt-5.2".to_string()),
            reasoning_effort: Some(ReasoningEffortConfig::High),
            model_provider_id: Some("openai".to_string()),
            rollout_path: None,
            cwd: None,
            token_info: None,
            model_context_window: Some(200_000),
            waiting_for_approval: false,
            last_action: Some("sent input to 00000000-0000-0000-0000-000000000003".to_string()),
            last_user_message: None,
            last_agent_message: Some("all good".to_string()),
            last_warning: Some("minor warning".to_string()),
            last_error: None,
        };

        let snapshot = AgentsDashboardSnapshot {
            revision: 7,
            captured_at,
            agents: vec![agent1, agent2],
        };

        let lines = render_dashboard_lines(&snapshot, true);
        assert_snapshot!("agents_dashboard", lines_to_plain_text(&lines));
    }
}

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::Mutex;

use chrono::DateTime;
use chrono::Local;
use codex_core::ResponseItem;
use codex_core::content_items_to_text;
use codex_core::protocol::ContextBlockDetailEvent;
use codex_core::protocol::ContextBlockKind;
use codex_core::protocol::ContextBlockSummary;
use codex_core::protocol::ContextOverviewEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;

use crate::render::renderable::Renderable;
use crate::status::format_tokens_compact;
use crate::text_formatting::truncate_text;

const ITEM_PREVIEW_GRAPHEMES: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ContextOverlayView {
    #[default]
    Overview,
    BlockDetail {
        block_id: String,
    },
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ContextDashboard {
    revision: u64,
    view: ContextOverlayView,
    captured_at: Option<DateTime<Local>>,
    overview: Option<ContextOverviewEvent>,
    detail: Option<ContextBlockDetailEvent>,
}

impl ContextDashboard {
    pub(crate) fn set_view(&mut self, view: ContextOverlayView) {
        if self.view == view {
            return;
        }
        if let ContextOverlayView::BlockDetail { block_id } = &view
            && self
                .detail
                .as_ref()
                .is_some_and(|detail| detail.block.id != *block_id)
        {
            self.detail = None;
        }
        self.view = view;
        self.bump_revision();
    }

    pub(crate) fn on_overview(&mut self, ev: &ContextOverviewEvent) {
        self.captured_at = Some(Local::now());
        self.overview = Some(ev.clone());
        self.bump_revision();
    }

    pub(crate) fn on_block_detail(&mut self, ev: &ContextBlockDetailEvent) {
        self.captured_at = Some(Local::now());
        self.detail = Some(ev.clone());
        self.bump_revision();
    }

    pub(crate) fn snapshot(&self) -> ContextDashboardSnapshot {
        ContextDashboardSnapshot {
            revision: self.revision,
            view: self.view.clone(),
            captured_at: self.captured_at.unwrap_or_else(Local::now),
            overview: self.overview.clone(),
            detail: self.detail.clone(),
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ContextDashboardSnapshot {
    revision: u64,
    view: ContextOverlayView,
    captured_at: DateTime<Local>,
    overview: Option<ContextOverviewEvent>,
    detail: Option<ContextBlockDetailEvent>,
}

pub(crate) struct ContextDashboardRenderable {
    dashboard: Arc<Mutex<ContextDashboard>>,
    cache: RefCell<ContextDashboardRenderCache>,
}

#[derive(Default)]
struct ContextDashboardRenderCache {
    revision: u64,
    width: u16,
    lines: Vec<Line<'static>>,
}

impl ContextDashboardRenderable {
    pub(crate) fn new(dashboard: Arc<Mutex<ContextDashboard>>) -> Self {
        Self {
            dashboard,
            cache: RefCell::new(ContextDashboardRenderCache::default()),
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
        cache.lines = render_context_lines(&snapshot, width);
    }
}

impl Renderable for ContextDashboardRenderable {
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

fn render_context_lines(snapshot: &ContextDashboardSnapshot, width: u16) -> Vec<Line<'static>> {
    match &snapshot.view {
        ContextOverlayView::Overview => render_overview_lines(snapshot, width),
        ContextOverlayView::BlockDetail { block_id } => render_detail_lines(snapshot, block_id),
    }
}

fn render_overview_lines(snapshot: &ContextDashboardSnapshot, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let Some(overview) = &snapshot.overview else {
        lines.push(Line::from(
            "Loading context snapshot… (run /context again if this stays empty)".dim(),
        ));
        return lines;
    };

    let total = overview.estimated_next_total_tokens.max(0);
    let input = overview.estimated_next_input_tokens.max(0);
    let window = overview.model_context_window.unwrap_or(0).max(0);
    let percent = if window > 0 {
        ((total.saturating_mul(100)).checked_div(window)).unwrap_or(0)
    } else {
        0
    };

    let total_fmt = format_tokens_compact(total);
    let input_fmt = format_tokens_compact(input);
    let window_fmt = if window > 0 {
        format_tokens_compact(window)
    } else {
        "?".to_string()
    };

    let mut header_spans: Vec<Span<'static>> = Vec::new();
    header_spans.push("Next request (est): ".bold());
    header_spans.push(format!("{total_fmt} tok").bold());
    header_spans.push("  ".into());
    header_spans.push(format!("input {input_fmt}").dim());
    header_spans.push("  ".into());
    if window > 0 {
        header_spans.push(format!("{percent}% of {window_fmt}").dim());
    } else {
        header_spans.push("context window unknown".dim());
    }
    header_spans.push("  ".into());
    header_spans.push(format!("updated {}", snapshot.captured_at.format("%H:%M:%S")).dim());
    lines.push(Line::from(header_spans));

    lines.push(Line::from(
        "Tip: /context <block-id> to inspect, /context disable <block-id> to exclude future turns."
            .dim(),
    ));
    lines.push(Line::from(""));

    let denominator = if window > 0 { window } else { total.max(1) };
    let bar_width = ((width as usize).saturating_sub(50)).clamp(10, 30);

    for block in &overview.blocks {
        lines.push(render_block_summary_line(block, denominator, bar_width));
    }

    lines
}

fn render_block_summary_line(
    block: &ContextBlockSummary,
    denominator: i64,
    bar_width: usize,
) -> Line<'static> {
    let status = if block.required {
        "REQ".cyan().bold()
    } else if block.enabled {
        "ON ".green().bold()
    } else {
        "OFF".red().bold()
    };

    let kind = kind_label(&block.kind);
    let kind = format!("{kind:>5}").dim();

    let id = block.id.clone().dim();
    let tokens_fmt = format_tokens_compact(block.token_estimate);
    let tokens = format!("{tokens_fmt:>6}").dim();
    let bar = render_token_bar(block, denominator, bar_width);

    Line::from(vec![
        status,
        "  ".into(),
        kind,
        "  ".into(),
        id,
        "  ".into(),
        tokens,
        " tok ".dim(),
        bar,
        "  ".into(),
        block.description.clone().into(),
    ])
}

fn kind_label(kind: &ContextBlockKind) -> &'static str {
    match kind {
        ContextBlockKind::Instructions => "INSTR",
        ContextBlockKind::Tools => "TOOLS",
        ContextBlockKind::Setup => "SETUP",
        ContextBlockKind::Update => "UPDATE",
        ContextBlockKind::Turn => "TURN",
        ContextBlockKind::Misc => "MISC",
    }
}

fn render_token_bar(block: &ContextBlockSummary, denominator: i64, width: usize) -> Span<'static> {
    let denom = denominator.max(1);
    let value = block.token_estimate.max(0);
    let filled = usize::try_from(value.saturating_mul(width as i64) / denom).unwrap_or(0);
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);

    let fill = "█".repeat(filled);
    let empty = "░".repeat(empty);
    let bar = format!("[{fill}{empty}]");

    if block.required {
        bar.cyan()
    } else if block.enabled {
        bar.green()
    } else {
        bar.red()
    }
}

fn render_detail_lines(snapshot: &ContextDashboardSnapshot, block_id: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let Some(detail) = &snapshot.detail else {
        lines.push(Line::from(format!("Loading block `{block_id}`…").dim()));
        return lines;
    };

    if detail.block.id != block_id {
        lines.push(Line::from(format!("Loading block `{block_id}`…").dim()));
        lines.push(Line::from(format!(
            "Received details for `{}`; run /context {block_id} again.",
            detail.block.id
        )));
        return lines;
    }

    let status = if detail.block.required {
        "required".cyan().bold()
    } else if detail.block.enabled {
        "enabled".green().bold()
    } else {
        "disabled".red().bold()
    };
    lines.push(Line::from(vec![
        "Block: ".bold(),
        detail.block.title.clone().bold(),
        "  ".into(),
        format!("({})", detail.block.id).dim(),
    ]));
    lines.push(Line::from(vec![
        "Status: ".bold(),
        status,
        "  ".into(),
        format!("kind {}", kind_label(&detail.block.kind)).dim(),
        "  ".into(),
        format!(
            "items {}",
            format_tokens_compact(i64::from(detail.block.item_count))
        )
        .dim(),
        "  ".into(),
        format!(
            "tokens {}",
            format_tokens_compact(detail.block.token_estimate)
        )
        .dim(),
    ]));
    lines.push(Line::from(detail.block.description.clone()));
    lines.push(Line::from(""));
    lines.push(Line::from("Items:".bold()));

    for (idx, item) in detail.items.iter().enumerate() {
        lines.extend(render_item_lines(idx, item));
    }

    lines
}

fn render_item_lines(idx: usize, item: &ResponseItem) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let prefix = format!("{idx:>4}. ");
    match item {
        ResponseItem::Message { role, content, .. } => {
            let text = content_items_to_text(content)
                .map(|text| truncate_text(text.trim(), ITEM_PREVIEW_GRAPHEMES))
                .unwrap_or_else(|| "<empty message>".to_string());
            lines.push(Line::from(vec![
                prefix.into(),
                "message".bold(),
                " ".into(),
                format!("({role})").dim(),
                ": ".into(),
                text.into(),
            ]));
        }
        ResponseItem::FunctionCall {
            name, arguments, ..
        } => {
            let args = truncate_text(arguments.trim(), ITEM_PREVIEW_GRAPHEMES);
            lines.push(Line::from(vec![
                prefix.into(),
                "function_call".bold(),
                " ".into(),
                name.clone().cyan(),
                " ".into(),
                args.dim(),
            ]));
        }
        ResponseItem::FunctionCallOutput {
            call_id, output, ..
        } => {
            let output = truncate_text(output.trim(), ITEM_PREVIEW_GRAPHEMES);
            lines.push(Line::from(vec![
                prefix.into(),
                "function_output".bold(),
                " ".into(),
                call_id.clone().dim(),
                ": ".into(),
                output.into(),
            ]));
        }
        ResponseItem::CustomToolCall { name, input, .. } => {
            let input = truncate_text(input.trim(), ITEM_PREVIEW_GRAPHEMES);
            lines.push(Line::from(vec![
                prefix.into(),
                "tool_call".bold(),
                " ".into(),
                name.clone().cyan(),
                " ".into(),
                input.dim(),
            ]));
        }
        ResponseItem::CustomToolCallOutput {
            call_id, output, ..
        } => {
            let output = truncate_text(output.trim(), ITEM_PREVIEW_GRAPHEMES);
            lines.push(Line::from(vec![
                prefix.into(),
                "tool_output".bold(),
                " ".into(),
                call_id.clone().dim(),
                ": ".into(),
                output.into(),
            ]));
        }
        ResponseItem::LocalShellCall { action, .. } => {
            let serialized = serde_json::to_string(action).unwrap_or_default();
            let preview = truncate_text(serialized.trim(), ITEM_PREVIEW_GRAPHEMES);
            lines.push(Line::from(vec![
                prefix.into(),
                "shell_call".bold(),
                ": ".into(),
                preview.into(),
            ]));
        }
        ResponseItem::WebSearchCall { action, .. } => {
            let serialized = serde_json::to_string(action).unwrap_or_default();
            let query = truncate_text(serialized.trim(), ITEM_PREVIEW_GRAPHEMES);
            lines.push(Line::from(vec![
                prefix.into(),
                "web_search".bold(),
                ": ".into(),
                query.into(),
            ]));
        }
        ResponseItem::Reasoning {
            encrypted_content, ..
        } => {
            let len = encrypted_content.as_ref().map_or(0, String::len);
            lines.push(Line::from(vec![
                prefix.into(),
                "reasoning".bold(),
                ": ".into(),
                format!("encrypted_content len={len}").dim(),
            ]));
        }
        ResponseItem::Compaction { encrypted_content } => {
            let len = encrypted_content.len();
            lines.push(Line::from(vec![
                prefix.into(),
                "compaction".bold(),
                ": ".into(),
                format!("encrypted_content len={len}").dim(),
            ]));
        }
        ResponseItem::GhostSnapshot { .. } => {
            lines.push(Line::from(vec![
                prefix.into(),
                "ghost_snapshot".bold(),
                " (not sent)".dim(),
            ]));
        }
        ResponseItem::Other => {
            lines.push(Line::from(vec![prefix.into(), "other".bold()]));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
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
    fn context_dashboard_overview_snapshot() {
        let blocks = vec![
            ContextBlockSummary {
                id: "instructions".to_string(),
                kind: ContextBlockKind::Instructions,
                title: "Instructions".to_string(),
                description: "Base/system prompt for model `gpt-5.2`.".to_string(),
                enabled: true,
                required: true,
                token_estimate: 7_000,
                item_count: 0,
            },
            ContextBlockSummary {
                id: "tools".to_string(),
                kind: ContextBlockKind::Tools,
                title: "Tools".to_string(),
                description: "12 tool(s) available for this turn.".to_string(),
                enabled: true,
                required: true,
                token_estimate: 2_000,
                item_count: 0,
            },
            ContextBlockSummary {
                id: "setup".to_string(),
                kind: ContextBlockKind::Setup,
                title: "Session setup".to_string(),
                description: "Developer permissions, user instructions, and environment context injected by Codex.".to_string(),
                enabled: true,
                required: true,
                token_estimate: 9_000,
                item_count: 4,
            },
            ContextBlockSummary {
                id: "turn:1".to_string(),
                kind: ContextBlockKind::Turn,
                title: "Turn 1".to_string(),
                description: "Write a tiny Rust program".to_string(),
                enabled: true,
                required: false,
                token_estimate: 80_000,
                item_count: 12,
            },
            ContextBlockSummary {
                id: "turn:2".to_string(),
                kind: ContextBlockKind::Turn,
                title: "Turn 2".to_string(),
                description: "Fix the failing test".to_string(),
                enabled: false,
                required: false,
                token_estimate: 60_000,
                item_count: 9,
            },
        ];
        let overview = ContextOverviewEvent {
            blocks,
            estimated_next_input_tokens: 140_000,
            estimated_next_total_tokens: 149_000,
            model_context_window: Some(258_000),
        };

        let snapshot = ContextDashboardSnapshot {
            revision: 1,
            view: ContextOverlayView::Overview,
            captured_at: Local
                .with_ymd_and_hms(2026, 4, 3, 12, 0, 0)
                .single()
                .expect("timestamp"),
            overview: Some(overview),
            detail: None,
        };

        let lines = render_overview_lines(&snapshot, 120);
        assert_snapshot!("context_dashboard_overview", lines_to_plain_text(&lines));
    }

    #[test]
    fn context_dashboard_detail_snapshot() {
        let block = ContextBlockSummary {
            id: "turn:1".to_string(),
            kind: ContextBlockKind::Turn,
            title: "Turn 1".to_string(),
            description: "Write a tiny Rust program".to_string(),
            enabled: true,
            required: false,
            token_estimate: 10_000,
            item_count: 3,
        };
        let detail = ContextBlockDetailEvent {
            block,
            items: vec![
                ResponseItem::Message {
                    id: None,
                    role: "user".to_string(),
                    content: vec![codex_core::ContentItem::InputText {
                        text: "hello world".to_string(),
                    }],
                },
                ResponseItem::FunctionCall {
                    id: None,
                    call_id: "call_1".to_string(),
                    name: "shell".to_string(),
                    arguments: "{\"cmd\":\"echo hi\"}".to_string(),
                },
                ResponseItem::FunctionCallOutput {
                    call_id: "call_1".to_string(),
                    output: codex_protocol::models::FunctionCallOutputPayload {
                        content: "hi".to_string(),
                        ..Default::default()
                    },
                },
            ],
        };

        let snapshot = ContextDashboardSnapshot {
            revision: 1,
            view: ContextOverlayView::BlockDetail {
                block_id: "turn:1".to_string(),
            },
            captured_at: Local
                .with_ymd_and_hms(2026, 4, 3, 12, 0, 0)
                .single()
                .expect("timestamp"),
            overview: None,
            detail: Some(detail),
        };

        let lines = render_detail_lines(&snapshot, "turn:1");
        assert_snapshot!("context_dashboard_detail", lines_to_plain_text(&lines));
    }
}

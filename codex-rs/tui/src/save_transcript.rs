use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::DateTime;
use chrono::Local;
use codex_protocol::ThreadId;
use codex_protocol::openai_models::ReasoningEffort;
use ratatui::text::Line;

use crate::history_cell::AgentMessageCell;
use crate::history_cell::FinalMessageSeparator;
use crate::history_cell::HistoryCell;
use crate::history_cell::McpToolCallCell;
use crate::history_cell::PatchHistoryCell;
use crate::history_cell::PlainHistoryCell;
use crate::history_cell::PlanUpdateCell;
use crate::history_cell::PrefixedWrappedHistoryCell;
use crate::history_cell::ReasoningSummaryCell;
use crate::history_cell::SessionHeaderHistoryCell;
use crate::history_cell::UnifiedExecInteractionCell;
use crate::history_cell::UserHistoryCell;
use crate::version::CODEX_CLI_VERSION;
use serde::Serialize;

pub(crate) struct SaveTranscriptMetadata {
    pub(crate) saved_at: DateTime<Local>,
    pub(crate) thread_id: Option<ThreadId>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) cwd: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveTranscriptFormat {
    Markdown,
    SftJsonl,
    CptJsonl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveTranscriptMode {
    /// Save only the conversation narrative: user, assistant, plan updates,
    /// and reasoning summaries.
    Compact,
    /// Save the full UI transcript, including tool calls/results and other
    /// non-chat events.
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveTranscriptArgs {
    pub(crate) filename: Option<String>,
    pub(crate) mode: SaveTranscriptMode,
    pub(crate) format: SaveTranscriptFormat,
}

pub(crate) fn parse_save_transcript_args(args: &str) -> SaveTranscriptArgs {
    let mut mode = SaveTranscriptMode::Compact;
    let mut format = SaveTranscriptFormat::Markdown;
    let mut parts: Vec<&str> = args.split_whitespace().collect();
    parts.retain(|part| {
        if *part == "--full" {
            mode = SaveTranscriptMode::Full;
            false
        } else if *part == "--markdown" {
            format = SaveTranscriptFormat::Markdown;
            false
        } else if *part == "--sft-jsonl" {
            format = SaveTranscriptFormat::SftJsonl;
            false
        } else if *part == "--cpt-jsonl" {
            format = SaveTranscriptFormat::CptJsonl;
            false
        } else {
            true
        }
    });

    let filename = (!parts.is_empty()).then(|| parts.join(" "));
    SaveTranscriptArgs {
        filename,
        mode,
        format,
    }
}

pub(crate) fn default_transcript_filename(saved_at: DateTime<Local>) -> String {
    format!("codex-{}.md", saved_at.format("%Y%m%dT%H%M%S"))
}

pub(crate) fn normalize_markdown_filename(filename: &str) -> String {
    let trimmed = filename.trim();
    if trimmed.to_ascii_lowercase().ends_with(".md") {
        trimmed.to_string()
    } else {
        format!("{trimmed}.md")
    }
}

pub(crate) fn default_transcript_filename_jsonl(
    saved_at: DateTime<Local>,
    format: SaveTranscriptFormat,
) -> String {
    let prefix = match format {
        SaveTranscriptFormat::Markdown => "codex",
        SaveTranscriptFormat::SftJsonl => "codex-sft",
        SaveTranscriptFormat::CptJsonl => "codex-cpt",
    };
    format!("{prefix}-{}.jsonl", saved_at.format("%Y%m%dT%H%M%S"))
}

pub(crate) fn normalize_jsonl_filename(filename: &str) -> String {
    let trimmed = filename.trim();
    if trimmed.to_ascii_lowercase().ends_with(".jsonl") {
        trimmed.to_string()
    } else {
        format!("{trimmed}.jsonl")
    }
}

pub(crate) fn export_chat_history_markdown(
    transcript_cells: &[Arc<dyn HistoryCell>],
    active_cell_lines: Option<Vec<Line<'static>>>,
    metadata: &SaveTranscriptMetadata,
    mode: SaveTranscriptMode,
) -> String {
    const EXPORT_WIDTH: u16 = 160;

    let mut output = String::new();
    output.push_str("# Codex chat history\n\n");

    output.push_str("## Metadata\n\n");
    output.push_str(&format!("- Saved at: {}\n", metadata.saved_at.to_rfc3339()));
    output.push_str(&format!("- Codex version: {CODEX_CLI_VERSION}\n"));
    output.push_str(&format!("- CWD: {}\n", metadata.cwd.display()));
    if let Some(thread_id) = metadata.thread_id {
        output.push_str(&format!("- Thread: {thread_id}\n"));
    }
    if let Some(model) = metadata.model.as_deref() {
        output.push_str(&format!("- Model: {model}\n"));
    }
    if let Some(effort) = metadata.reasoning_effort {
        output.push_str(&format!("- Reasoning effort: {effort}\n"));
    }
    output.push('\n');

    output.push_str("## History\n\n");

    let entries = export_entries(transcript_cells, mode, EXPORT_WIDTH);
    let mut entry_index = 0usize;
    for entry in entries {
        if entry.content.trim().is_empty() {
            continue;
        }
        entry_index += 1;
        write_entry_markdown(&mut output, entry_index, entry.title, &entry.content);
    }

    if mode == SaveTranscriptMode::Full
        && let Some(lines) = active_cell_lines
        && !lines.is_empty()
    {
        entry_index += 1;
        write_entry_markdown(
            &mut output,
            entry_index,
            "In-progress output",
            &lines_to_plain_text_with_prefix_stripping(&lines, ExportEntryKind::Event),
        );
    }

    output
}

pub(crate) fn export_chat_history_sft_jsonl(
    transcript_cells: &[Arc<dyn HistoryCell>],
    metadata: &SaveTranscriptMetadata,
) -> String {
    const EXPORT_WIDTH: u16 = u16::MAX;

    let messages = export_conversation_messages(transcript_cells, EXPORT_WIDTH);
    let record = SftJsonlRecord {
        id: export_id(metadata),
        created_at: metadata.saved_at.timestamp(),
        messages,
        metadata: export_metadata(metadata),
    };

    let mut out = serde_json::to_string(&record).unwrap_or_else(|_| "{}".to_string());
    out.push('\n');
    out
}

pub(crate) fn export_chat_history_cpt_jsonl(
    transcript_cells: &[Arc<dyn HistoryCell>],
    metadata: &SaveTranscriptMetadata,
) -> String {
    const EXPORT_WIDTH: u16 = u16::MAX;

    let messages = export_conversation_messages(transcript_cells, EXPORT_WIDTH);
    let text = format_cpt_text(&messages);
    let record = CptJsonlRecord {
        id: export_id(metadata),
        created_at: metadata.saved_at.timestamp(),
        text,
        metadata: export_metadata(metadata),
    };

    let mut out = serde_json::to_string(&record).unwrap_or_else(|_| "{}".to_string());
    out.push('\n');
    out
}

pub(crate) fn write_transcript_markdown(path: &Path, markdown: &str) -> io::Result<()> {
    let mut opts = OpenOptions::new();
    opts.create(true).truncate(true).write(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let mut file = opts.open(path)?;
    file.write_all(markdown.as_bytes())?;
    file.flush()
}

pub(crate) fn write_transcript_jsonl(path: &Path, jsonl: &str) -> io::Result<()> {
    let mut opts = OpenOptions::new();
    opts.create(true).truncate(true).write(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let mut file = opts.open(path)?;
    file.write_all(jsonl.as_bytes())?;
    file.flush()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportEntryKind {
    Session,
    User,
    Assistant,
    Thought,
    Plan,
    Shell,
    Patch,
    McpTool,
    Warning,
    Separator,
    Event,
}

#[derive(Debug, Clone)]
struct ExportEntry {
    kind: ExportEntryKind,
    title: &'static str,
    content: String,
}

fn export_entries(
    transcript_cells: &[Arc<dyn HistoryCell>],
    mode: SaveTranscriptMode,
    width: u16,
) -> Vec<ExportEntry> {
    let mut out: Vec<ExportEntry> = Vec::new();

    for cell in transcript_cells {
        let kind = history_cell_kind(cell.as_ref());
        if mode == SaveTranscriptMode::Compact && !kind_in_compact_export(kind, cell.as_ref()) {
            continue;
        }

        let lines = cell.transcript_lines(width);
        if lines.is_empty() {
            continue;
        }

        let mut content = lines_to_plain_text_with_prefix_stripping(&lines, kind);
        if content.trim().is_empty() {
            if let Some(prev) = out.last_mut()
                && should_merge_with_previous(prev.kind, kind, cell.as_ref())
            {
                prev.content.push('\n');
            }
            continue;
        }

        if let Some(prev) = out.last_mut()
            && should_merge_with_previous(prev.kind, kind, cell.as_ref())
        {
            if !prev.content.ends_with('\n') {
                prev.content.push('\n');
            }
            prev.content.push_str(content.trim_end_matches('\n'));
            prev.content.push('\n');
            continue;
        }

        content = content.trim_end_matches('\n').to_string();
        content.push('\n');

        out.push(ExportEntry {
            kind,
            title: export_entry_title(kind),
            content,
        });
    }

    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptMetadataJson {
    saved_at: String,
    codex_version: String,
    cwd: String,
    thread_id: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SftJsonlMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SftJsonlRecord {
    id: String,
    created_at: i64,
    messages: Vec<SftJsonlMessage>,
    metadata: TranscriptMetadataJson,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CptJsonlRecord {
    id: String,
    created_at: i64,
    text: String,
    metadata: TranscriptMetadataJson,
}

fn export_id(metadata: &SaveTranscriptMetadata) -> String {
    metadata
        .thread_id
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("codex-{}", metadata.saved_at.format("%Y%m%dT%H%M%S")))
}

fn export_metadata(metadata: &SaveTranscriptMetadata) -> TranscriptMetadataJson {
    TranscriptMetadataJson {
        saved_at: metadata.saved_at.to_rfc3339(),
        codex_version: CODEX_CLI_VERSION.to_string(),
        cwd: metadata.cwd.display().to_string(),
        thread_id: metadata.thread_id.as_ref().map(ToString::to_string),
        model: metadata.model.clone(),
        reasoning_effort: metadata.reasoning_effort.map(|effort| effort.to_string()),
    }
}

fn export_conversation_messages(
    transcript_cells: &[Arc<dyn HistoryCell>],
    width: u16,
) -> Vec<SftJsonlMessage> {
    export_entries(transcript_cells, SaveTranscriptMode::Compact, width)
        .into_iter()
        .filter_map(|entry| match entry.kind {
            ExportEntryKind::User => Some(SftJsonlMessage {
                role: "user".to_string(),
                content: entry.content.trim().to_string(),
            }),
            ExportEntryKind::Assistant => Some(SftJsonlMessage {
                role: "assistant".to_string(),
                content: entry.content.trim().to_string(),
            }),
            _ => None,
        })
        .filter(|msg| !msg.content.is_empty())
        .collect()
}

fn format_cpt_text(messages: &[SftJsonlMessage]) -> String {
    let mut out = String::new();
    for (idx, msg) in messages.iter().enumerate() {
        let label = match msg.role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            other => other,
        };

        if idx > 0 {
            out.push('\n');
            out.push('\n');
        }
        out.push_str(label);
        out.push(':');
        out.push('\n');
        out.push_str(msg.content.trim_end());
    }
    out
}

fn export_entry_title(kind: ExportEntryKind) -> &'static str {
    match kind {
        ExportEntryKind::Session => "Session",
        ExportEntryKind::User => "User",
        ExportEntryKind::Assistant => "Assistant",
        ExportEntryKind::Thought => "Thought",
        ExportEntryKind::Plan => "Plan",
        ExportEntryKind::Shell => "Shell",
        ExportEntryKind::Patch => "Patch",
        ExportEntryKind::McpTool => "MCP tool",
        ExportEntryKind::Warning => "Warning",
        ExportEntryKind::Separator => "Separator",
        ExportEntryKind::Event => "Event",
    }
}

fn kind_in_compact_export(kind: ExportEntryKind, cell: &dyn HistoryCell) -> bool {
    match kind {
        ExportEntryKind::User
        | ExportEntryKind::Assistant
        | ExportEntryKind::Thought
        | ExportEntryKind::Plan
        | ExportEntryKind::Warning => true,
        ExportEntryKind::Event => is_error_event(cell),
        ExportEntryKind::Session
        | ExportEntryKind::Shell
        | ExportEntryKind::Patch
        | ExportEntryKind::McpTool
        | ExportEntryKind::Separator => false,
    }
}

fn is_error_event(cell: &dyn HistoryCell) -> bool {
    let any = cell.as_any();
    if let Some(cell) = any.downcast_ref::<PlainHistoryCell>()
        && let Some(line) = cell.display_lines(80).first()
    {
        let text = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        return text.trim_start().starts_with('■');
    }
    false
}

fn should_merge_with_previous(
    previous: ExportEntryKind,
    current: ExportEntryKind,
    current_cell: &dyn HistoryCell,
) -> bool {
    if previous != current {
        return false;
    }

    match current {
        ExportEntryKind::Assistant => current_cell.is_stream_continuation(),
        _ => false,
    }
}

fn history_cell_kind(cell: &dyn HistoryCell) -> ExportEntryKind {
    let any = cell.as_any();
    if any.is::<SessionHeaderHistoryCell>() {
        ExportEntryKind::Session
    } else if any.is::<UserHistoryCell>() {
        ExportEntryKind::User
    } else if any.is::<AgentMessageCell>() {
        ExportEntryKind::Assistant
    } else if any.is::<ReasoningSummaryCell>() {
        ExportEntryKind::Thought
    } else if any.is::<UnifiedExecInteractionCell>() || any.is::<crate::exec_cell::ExecCell>() {
        ExportEntryKind::Shell
    } else if any.is::<PatchHistoryCell>() {
        ExportEntryKind::Patch
    } else if any.is::<McpToolCallCell>() {
        ExportEntryKind::McpTool
    } else if any.is::<PlanUpdateCell>() {
        ExportEntryKind::Plan
    } else if any.is::<PrefixedWrappedHistoryCell>() {
        ExportEntryKind::Warning
    } else if any.is::<FinalMessageSeparator>() {
        ExportEntryKind::Separator
    } else {
        ExportEntryKind::Event
    }
}

fn lines_to_plain_text_with_prefix_stripping(
    lines: &[Line<'static>],
    kind: ExportEntryKind,
) -> String {
    let mut output = String::new();
    for (idx, line) in lines.iter().enumerate() {
        let mut text = line
            .spans
            .iter()
            .map(|sp| sp.content.as_ref())
            .collect::<String>();

        if matches!(
            kind,
            ExportEntryKind::Assistant | ExportEntryKind::Thought | ExportEntryKind::User
        ) {
            text = strip_line_prefix(text, kind);
        }

        if idx > 0 {
            output.push('\n');
        }
        output.push_str(&text);
    }

    strip_leading_and_trailing_blank_lines(output)
}

fn strip_line_prefix(mut line: String, kind: ExportEntryKind) -> String {
    let prefix = match kind {
        ExportEntryKind::User => {
            if line.starts_with("› ") {
                Some("› ")
            } else if line.starts_with("  ") {
                Some("  ")
            } else {
                None
            }
        }
        ExportEntryKind::Assistant | ExportEntryKind::Thought => {
            if line.starts_with("• ") {
                Some("• ")
            } else if line.starts_with("  ") {
                Some("  ")
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some(prefix) = prefix {
        line = line.strip_prefix(prefix).unwrap_or(&line).to_string();
    }
    line
}

fn strip_leading_and_trailing_blank_lines(text: String) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    while !lines.is_empty() && lines[0].trim().is_empty() {
        lines.remove(0);
    }
    while !lines.is_empty() && lines[lines.len() - 1].trim().is_empty() {
        lines.pop();
    }
    lines.join("\n")
}

fn write_entry_markdown(output: &mut String, index: usize, title: &str, content: &str) {
    output.push_str(&format!("### {index}. {title}\n\n"));

    let fence = choose_code_fence(content);
    output.push_str(&format!("{fence}text\n"));
    output.push_str(content);
    if !content.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(&format!("{fence}\n\n"));
}

fn choose_code_fence(content: &str) -> String {
    let mut max_run: usize = 0;
    let mut current: usize = 0;
    for ch in content.chars() {
        if ch == '`' {
            current += 1;
            max_run = max_run.max(current);
        } else {
            current = 0;
        }
    }

    let fence_len = std::cmp::max(3, max_run + 1);
    "`".repeat(fence_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;

    #[test]
    fn default_transcript_filename_includes_timestamp_and_extension() {
        let saved_at = Local
            .with_ymd_and_hms(2026, 4, 3, 22, 13, 43)
            .single()
            .unwrap();
        assert_eq!(
            default_transcript_filename(saved_at),
            "codex-20260403T221343.md"
        );
    }

    #[test]
    fn normalize_markdown_filename_appends_extension_when_missing() {
        assert_eq!(normalize_markdown_filename("notes"), "notes.md");
        assert_eq!(normalize_markdown_filename("notes.md"), "notes.md");
        assert_eq!(normalize_markdown_filename("notes.MD"), "notes.MD");
        assert_eq!(
            normalize_markdown_filename("path/to/notes"),
            "path/to/notes.md"
        );
    }

    #[test]
    fn choose_code_fence_exceeds_longest_backtick_run() {
        assert_eq!(choose_code_fence("hello"), "```");
        assert_eq!(choose_code_fence("```"), "````");
        assert_eq!(choose_code_fence("``````"), "```````");
    }

    #[test]
    fn normalize_jsonl_filename_appends_extension_when_missing() {
        assert_eq!(normalize_jsonl_filename("data"), "data.jsonl");
        assert_eq!(normalize_jsonl_filename("data.jsonl"), "data.jsonl");
        assert_eq!(normalize_jsonl_filename("data.JSONL"), "data.JSONL");
    }
}

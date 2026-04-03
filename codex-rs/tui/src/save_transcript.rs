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
use crate::history_cell::HistoryCell;
use crate::history_cell::McpToolCallCell;
use crate::history_cell::PatchHistoryCell;
use crate::history_cell::PlanUpdateCell;
use crate::history_cell::SessionHeaderHistoryCell;
use crate::history_cell::UnifiedExecInteractionCell;
use crate::history_cell::UserHistoryCell;
use crate::version::CODEX_CLI_VERSION;

pub(crate) struct SaveTranscriptMetadata {
    pub(crate) saved_at: DateTime<Local>,
    pub(crate) thread_id: Option<ThreadId>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) cwd: PathBuf,
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

pub(crate) fn export_chat_history_markdown(
    transcript_cells: &[Arc<dyn HistoryCell>],
    active_cell_lines: Option<Vec<Line<'static>>>,
    metadata: &SaveTranscriptMetadata,
) -> String {
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

    let mut entry_index: usize = 0;
    for cell in transcript_cells {
        let lines = cell.transcript_lines(u16::MAX);
        if lines.is_empty() {
            continue;
        }
        entry_index += 1;
        write_entry_markdown(
            &mut output,
            entry_index,
            history_cell_title(cell.as_ref()),
            &lines_to_plain_text(&lines),
        );
    }

    if let Some(lines) = active_cell_lines
        && !lines.is_empty()
    {
        entry_index += 1;
        write_entry_markdown(
            &mut output,
            entry_index,
            "In-progress output",
            &lines_to_plain_text(&lines),
        );
    }

    output
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

fn history_cell_title(cell: &dyn HistoryCell) -> &'static str {
    let any = cell.as_any();
    if any.is::<SessionHeaderHistoryCell>() {
        "Session"
    } else if any.is::<UserHistoryCell>() {
        "User"
    } else if any.is::<AgentMessageCell>() {
        "Assistant"
    } else if any.is::<UnifiedExecInteractionCell>() {
        "Shell"
    } else if any.is::<PatchHistoryCell>() {
        "Patch"
    } else if any.is::<McpToolCallCell>() {
        "MCP tool"
    } else if any.is::<PlanUpdateCell>() {
        "Plan"
    } else {
        "Event"
    }
}

fn lines_to_plain_text(lines: &[Line<'static>]) -> String {
    let mut output = String::new();
    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            output.push('\n');
        }
        output.push_str(
            &line
                .spans
                .iter()
                .map(|sp| sp.content.as_ref())
                .collect::<String>(),
        );
    }
    output
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
}

use std::collections::HashSet;

use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ContextBlockKind;
use codex_protocol::protocol::ContextBlockSummary;
use codex_protocol::protocol::ENVIRONMENT_CONTEXT_OPEN_TAG;

use crate::context_manager::is_user_turn_boundary;
use crate::truncate::approx_token_count;
use crate::truncate::approx_tokens_from_byte_count;

#[derive(Debug, Clone)]
pub(crate) struct HistoryBlock {
    pub(crate) summary: ContextBlockSummary,
    pub(crate) range: (usize, usize),
}

pub(crate) fn build_history_blocks(
    items: &[ResponseItem],
    disabled_block_ids: &HashSet<String>,
) -> Vec<HistoryBlock> {
    if items.is_empty() {
        return Vec::new();
    }

    let first_user_turn_idx = items
        .iter()
        .position(is_user_turn_boundary)
        .unwrap_or(items.len());

    let mut blocks: Vec<HistoryBlock> = Vec::new();
    if first_user_turn_idx > 0 {
        blocks.push(make_required_block(
            "setup".to_string(),
            ContextBlockKind::Setup,
            "Session setup".to_string(),
            "Developer permissions, user instructions, and environment context injected by Codex."
                .to_string(),
            items,
            0,
            first_user_turn_idx,
        ));
    }

    let mut idx = first_user_turn_idx;
    let mut turn_counter: u32 = 0;
    while idx < items.len() {
        if is_context_update_item(&items[idx]) {
            let end = advance_while(items, idx, is_context_update_item);
            blocks.push(make_required_block(
                format!("update:{idx}"),
                ContextBlockKind::Update,
                "Turn context update".to_string(),
                "Environment and/or permissions changed between turns.".to_string(),
                items,
                idx,
                end,
            ));
            idx = end;
            continue;
        }

        if is_user_turn_boundary(&items[idx]) {
            let start = idx;
            let end = next_boundary_idx(items, start + 1);
            turn_counter = turn_counter.saturating_add(1);
            let id = format!("turn:{turn_counter}");
            let enabled = !disabled_block_ids.contains(&id);
            let first_line = first_user_message_first_line(&items[start])
                .unwrap_or_else(|| "User message".to_string());
            let title = format!("Turn {turn_counter}");
            let description = first_line;
            blocks.push(make_block(
                ContextBlockSummary {
                    id,
                    kind: ContextBlockKind::Turn,
                    title,
                    description,
                    enabled,
                    required: false,
                    token_estimate: 0,
                    item_count: 0,
                },
                items,
                start,
                end,
            ));
            idx = end;
            continue;
        }

        let start = idx;
        let end = next_boundary_idx(items, start + 1);
        blocks.push(make_required_block(
            format!("misc:{start}"),
            ContextBlockKind::Misc,
            "Misc context".to_string(),
            "Context items that are not part of a user turn.".to_string(),
            items,
            start,
            end,
        ));
        idx = end;
    }

    blocks
}

pub(crate) fn find_block_by_id<'a>(
    blocks: &'a [HistoryBlock],
    id: &str,
) -> Option<&'a HistoryBlock> {
    blocks.iter().find(|block| block.summary.id == id)
}

pub(crate) fn filter_items_for_prompt(
    items: Vec<ResponseItem>,
    disabled_block_ids: &HashSet<String>,
) -> Vec<ResponseItem> {
    if disabled_block_ids.is_empty() {
        return items;
    }

    let blocks = build_history_blocks(&items, disabled_block_ids);
    let mut filtered: Vec<ResponseItem> = Vec::with_capacity(items.len());
    for block in blocks {
        if block.summary.required || block.summary.enabled {
            filtered.extend_from_slice(&items[block.range.0..block.range.1]);
        }
    }

    // Be defensive: after filtering, re-apply normalization invariants so callers
    // never send orphan tool outputs or missing outputs.
    crate::context_manager::normalize::ensure_call_outputs_present(&mut filtered);
    crate::context_manager::normalize::remove_orphan_outputs(&mut filtered);

    filtered
}

fn make_required_block(
    id: String,
    kind: ContextBlockKind,
    title: String,
    description: String,
    items: &[ResponseItem],
    start: usize,
    end: usize,
) -> HistoryBlock {
    make_block(
        ContextBlockSummary {
            id,
            kind,
            title,
            description,
            enabled: true,
            required: true,
            token_estimate: 0,
            item_count: 0,
        },
        items,
        start,
        end,
    )
}

fn make_block(
    mut summary: ContextBlockSummary,
    items: &[ResponseItem],
    start: usize,
    end: usize,
) -> HistoryBlock {
    let token_estimate = estimate_items_tokens(&items[start..end]);
    let item_count = u32::try_from(end.saturating_sub(start)).unwrap_or(u32::MAX);
    summary.token_estimate = token_estimate;
    summary.item_count = item_count;
    HistoryBlock {
        summary,
        range: (start, end),
    }
}

fn next_boundary_idx(items: &[ResponseItem], start: usize) -> usize {
    let mut idx = start;
    while idx < items.len() {
        if is_user_turn_boundary(&items[idx]) || is_context_update_item(&items[idx]) {
            break;
        }
        idx += 1;
    }
    idx
}

fn advance_while<F>(items: &[ResponseItem], start: usize, predicate: F) -> usize
where
    F: Fn(&ResponseItem) -> bool,
{
    let mut idx = start;
    while idx < items.len() && predicate(&items[idx]) {
        idx += 1;
    }
    idx
}

fn is_context_update_item(item: &ResponseItem) -> bool {
    match item {
        ResponseItem::Message {
            role, content: _, ..
        } if role == "developer" => true,
        ResponseItem::Message { role, content, .. } if role == "user" => {
            message_starts_with_tag(content, ENVIRONMENT_CONTEXT_OPEN_TAG)
        }
        _ => false,
    }
}

fn message_starts_with_tag(content: &[ContentItem], tag: &str) -> bool {
    let [ContentItem::InputText { text }] = content else {
        return false;
    };
    text.trim_start().starts_with(tag)
}

fn first_user_message_first_line(item: &ResponseItem) -> Option<String> {
    let ResponseItem::Message { role, content, .. } = item else {
        return None;
    };
    if role != "user" {
        return None;
    }
    for content_item in content {
        let ContentItem::InputText { text } = content_item else {
            continue;
        };
        let first_line = text.lines().next().unwrap_or_default().trim();
        if !first_line.is_empty() {
            return Some(first_line.to_string());
        }
    }
    None
}

pub(crate) fn estimate_items_tokens(items: &[ResponseItem]) -> i64 {
    items.iter().fold(0i64, |acc, item| {
        acc.saturating_add(estimate_response_item_tokens(item))
    })
}

fn estimate_response_item_tokens(item: &ResponseItem) -> i64 {
    match item {
        ResponseItem::GhostSnapshot { .. } => 0,
        ResponseItem::Reasoning {
            encrypted_content: Some(content),
            ..
        }
        | ResponseItem::Compaction {
            encrypted_content: content,
        } => {
            let reasoning_bytes = estimate_reasoning_length(content.len());
            i64::try_from(approx_tokens_from_byte_count(reasoning_bytes)).unwrap_or(i64::MAX)
        }
        other => {
            let serialized = serde_json::to_string(other).unwrap_or_default();
            i64::try_from(approx_token_count(serialized.as_str())).unwrap_or(i64::MAX)
        }
    }
}

fn estimate_reasoning_length(encoded_len: usize) -> usize {
    encoded_len
        .saturating_mul(3)
        .checked_div(4)
        .unwrap_or(0)
        .saturating_sub(650)
}

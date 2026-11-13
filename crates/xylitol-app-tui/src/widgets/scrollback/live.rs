use crate::activity_fold::{
    ActivityFoldState, SegmentLevel, cluster_middle_indices, count_cluster, format_cluster_header,
    format_elapsed_secs, format_envelope_line, live_think_id_for_cluster, streaming_thought_counts,
};
#[cfg(test)]
use crate::activity_fold::{ToolActivityRole, is_path_placeholder, tool_activity_role};
use crate::bridge::{
    AskPhase, BashBlockStatus, STREAMING_THINK_ID, StreamingTailKind, UiEntry, UiModel, UiPhase,
};
use crate::layout::LayoutTheme;

use super::super::fold_hit::{FoldHitTable, FoldTarget};
use super::super::glyphs::GlyphSet;
use super::ScrollbackFold;
use super::paint::{inter_block_spacer, push_wrapped};
use xylitol_tui::truncate_to_width;

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_envelope_header_row(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    activity: &mut ActivityFoldState,
    seg: &crate::activity_fold::ActivitySegment,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    width: usize,
    need_spacer: &mut bool,
) {
    if *need_spacer {
        lines.push(inter_block_spacer(width));
    }
    let expanded = activity.level_of(&seg.id) != SegmentLevel::L3;
    let dur = activity.duration_for(&seg.id);
    let plain = format_envelope_line(glyphs, dur.as_deref(), expanded);
    let painted = theme.paint_muted(&plain);
    let row_start = lines.len();
    push_wrapped(lines, &painted, width);
    let row_end = lines.len();
    fold_hits.push(
        row_start,
        0,
        width.max(1),
        FoldTarget::Segment(seg.id.clone()),
    );
    activity
        .row_spans
        .insert(seg.id.clone(), row_start, row_end);
    *need_spacer = true;
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_cluster_header_row(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    segments: &[crate::activity_fold::ActivitySegment],
    si: usize,
    ci: usize,
    model: &UiModel,
    activity: &ActivityFoldState,
    fold: &ScrollbackFold,
    live_seg_idx: Option<usize>,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    width: usize,
    need_spacer: &mut bool,
) {
    let seg = &segments[si];
    let cl = &seg.clusters[ci];
    if *need_spacer {
        lines.push(inter_block_spacer(width));
    }
    let live_seg = live_seg_idx == Some(si);
    let progressive = is_open_live_cluster(seg, ci, live_seg);
    let mut counts = count_cluster(&model.entries, cl);
    let live_think = live_think_id_for_cluster(model, cl, progressive);
    if let Some(id) = live_think.as_deref() {
        counts = counts.with_live_think(id);
    }
    let thought_only_live =
        counts.is_thought_only() && live_think.as_deref() == Some(STREAMING_THINK_ID);
    let expanded = if thought_only_live {
        fold.thinking_effective(STREAMING_THINK_ID)
    } else {
        activity.cluster_kids_visible(&seg.id, &cl.id)
    };
    let thought_dur = counts
        .is_thought_only()
        .then(|| thought_duration_label(model, cl))
        .flatten();
    let mut plain = format_cluster_header(
        glyphs,
        &counts,
        expanded,
        progressive,
        thought_dur.as_deref(),
    );
    if counts.is_thought_only() && !expanded {
        let streaming_latest = if live_think.as_deref() == Some(STREAMING_THINK_ID) {
            model.streaming_thinking.lines().last().unwrap_or("").trim()
        } else {
            ""
        };
        let latest = if streaming_latest.is_empty() {
            latest_thinking_line(model, cl).unwrap_or("")
        } else {
            streaming_latest
        };
        if !latest.is_empty() {
            plain.push_str(" · ");
            plain.push_str(latest);
        }
    }
    let plain = truncate_to_width(&plain, width, "…", true);
    let painted = theme.paint_muted(&plain);
    let row_start = lines.len();
    push_wrapped(lines, &painted, width);
    let target = if thought_only_live {
        FoldTarget::Thinking(STREAMING_THINK_ID.to_string())
    } else {
        FoldTarget::Cluster(cl.id.clone())
    };
    fold_hits.push(row_start, 0, width.max(1), target);
    *need_spacer = true;
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_folded_streaming_thought(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    fold: &ScrollbackFold,
    text: &str,
    live_open_cluster: bool,
    live_open_cluster_thought_only: bool,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    width: usize,
) {
    if !fold.defaults.thinking_visible {
        return;
    }
    let expanded = fold.thinking_effective(STREAMING_THINK_ID);
    if live_open_cluster && live_open_cluster_thought_only {
        if expanded {
            push_wrapped(lines, &theme.paint_muted(&format!("{text}…")), width);
        }
        return;
    }

    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let header = if live_open_cluster {
        let mut plain = format!("{marker} Thinking");
        if !expanded {
            let latest = text.lines().last().unwrap_or("").trim();
            if !latest.is_empty() {
                plain.push_str(" · ");
                plain.push_str(latest);
            }
        }
        let plain = truncate_to_width(&plain, width, "…", true);
        theme.paint_muted(&plain)
    } else {
        let counts = streaming_thought_counts();
        let mut plain = format_cluster_header(glyphs, &counts, expanded, true, None);
        if !expanded {
            let latest = text.lines().last().unwrap_or("").trim();
            if !latest.is_empty() {
                plain.push_str(" · ");
                plain.push_str(latest);
            }
        }
        let plain = truncate_to_width(&plain, width, "…", true);
        theme.paint_muted(&plain)
    };
    let row_start = lines.len();
    push_wrapped(lines, &header, width);
    if expanded {
        push_wrapped(lines, &theme.paint_muted(&format!("{text}…")), width);
    }
    fold_hits.push_rect(
        row_start,
        0,
        width.max(1),
        lines.len().saturating_sub(row_start).max(1),
        FoldTarget::Thinking(STREAMING_THINK_ID.to_string()),
    );
}

fn latest_thinking_line<'a>(
    model: &'a UiModel,
    cluster: &crate::activity_fold::ActivityCluster,
) -> Option<&'a str> {
    for idx in cluster_middle_indices(&model.entries, cluster)
        .into_iter()
        .rev()
    {
        if let Some(UiEntry::Thinking { text, .. }) = model.entries.get(idx) {
            let latest = text.lines().last().unwrap_or("").trim();
            if !latest.is_empty() {
                return Some(latest);
            }
        }
    }
    None
}

fn thought_duration_label(
    model: &UiModel,
    cluster: &crate::activity_fold::ActivityCluster,
) -> Option<String> {
    let mut secs = 0u64;
    let mut found = false;
    for idx in cluster_middle_indices(&model.entries, cluster) {
        if let Some(UiEntry::Thinking {
            elapsed_secs: Some(s),
            ..
        }) = model.entries.get(idx)
        {
            found = true;
            secs = secs.saturating_add(*s);
        }
    }
    found.then(|| format_elapsed_secs(secs))
}

pub(super) fn is_open_live_cluster(
    seg: &crate::activity_fold::ActivitySegment,
    ci: usize,
    live_seg: bool,
) -> bool {
    live_seg && ci + 1 == seg.clusters.len() && seg.clusters[ci].seal_assistant_idx.is_none()
}

pub(super) fn streaming_assistant_displayable(model: &UiModel) -> bool {
    model
        .streaming_scrollback_tails()
        .iter()
        .any(|(kind, text)| {
            *kind == StreamingTailKind::Assistant && text.chars().any(|c| !c.is_whitespace())
        })
}

pub(super) fn live_window_segment_idx(
    model: &UiModel,
    segments: &[crate::activity_fold::ActivitySegment],
    has_asst_tail: bool,
) -> Option<usize> {
    if model.phase != UiPhase::Busy || has_asst_tail {
        return None;
    }
    let (si, seg) = segments.iter().enumerate().next_back()?;
    let last = seg.clusters.last()?;
    if last.seal_assistant_idx.is_some() {
        return None;
    }
    Some(si)
}

pub(super) fn is_ask_waiting(entry: &UiEntry) -> bool {
    matches!(
        entry,
        UiEntry::Ask {
            phase: AskPhase::Waiting,
            ..
        }
    )
}

pub(super) fn is_inflight_hidden(entry: &UiEntry) -> bool {
    matches!(
        entry,
        UiEntry::Tool { done: false, .. }
            | UiEntry::Bash {
                status: BashBlockStatus::Pending,
                ..
            }
    )
}

#[cfg(test)]
pub(super) fn inflight_short_label(entry: &UiEntry) -> Option<String> {
    match entry {
        UiEntry::Tool {
            done: false,
            name,
            args_preview,
            tool_path,
            ..
        } => {
            let file = tool_path
                .as_deref()
                .filter(|s| !is_path_placeholder(s))
                .unwrap_or(args_preview.as_str());
            let file = if is_path_placeholder(file) {
                None
            } else {
                Some(file)
            };
            let n = name.to_ascii_lowercase();
            let file_label = |verb: &str| match file {
                Some(file) => format!("{verb} {file}"),
                None => verb.to_string(),
            };
            let label = match tool_activity_role(&n) {
                ToolActivityRole::Edit => file_label("Editing"),
                ToolActivityRole::ExploreFile => file_label("Reading"),
                ToolActivityRole::ExploreSearch => file_label("Searching"),
                ToolActivityRole::Run => file_label("Running"),
                ToolActivityRole::Used => format!("Running {name}"),
            };
            Some(label)
        }
        UiEntry::Bash {
            status: BashBlockStatus::Pending,
            command,
            ..
        } => Some(format!("Running {command}")),
        _ => None,
    }
}

pub(super) fn live_tail_label(
    model: &UiModel,
    segments: &[crate::activity_fold::ActivitySegment],
    has_asst_tail: bool,
) -> Option<String> {
    if has_asst_tail {
        return None;
    }
    let last_cluster_unsealed = segments.last().is_none_or(|s| {
        s.clusters
            .last()
            .is_none_or(|c| c.seal_assistant_idx.is_none())
    });
    if !last_cluster_unsealed {
        return None;
    }
    if model.entries.iter().rev().any(is_ask_waiting) {
        return Some("Asking questions".into());
    }
    None
}

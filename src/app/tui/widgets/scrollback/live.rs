use crate::app::tui::activity_fold::{
    ActivityFoldState, SegmentLevel, cluster_middle_indices, count_cluster, format_cluster_header,
    format_elapsed_secs, format_envelope_line, live_think_id_for_cluster, streaming_thought_counts,
};
#[cfg(test)]
use crate::app::tui::activity_fold::{ToolActivityRole, is_path_placeholder, tool_activity_role};
use crate::app::tui::bridge::{AskPhase, BashBlockStatus, UiEntry, UiModel, UiPhase};
use crate::app::tui::layout::LayoutTheme;

use super::super::fold_hit::{FoldHitTable, FoldTarget};
use super::super::glyphs::GlyphSet;
use super::cache::marker_cols;
use super::paint::{inter_block_spacer, push_wrapped};

#[allow(clippy::too_many_arguments)] // paint planes: lines, hits, activity, theme
pub(super) fn paint_envelope_header_row(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    activity: &mut ActivityFoldState,
    seg: &crate::app::tui::activity_fold::ActivitySegment,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    width: usize,
    need_spacer: &mut bool,
) {
    if *need_spacer {
        lines.push(inter_block_spacer(width));
    }
    let expanded = activity.effective_level(&seg.id) != SegmentLevel::L3;
    let dur = activity.duration_for(&seg.id);
    let plain = format_envelope_line(glyphs, dur.as_deref(), expanded);
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let painted = theme.paint_muted(&plain);
    let row_start = lines.len();
    push_wrapped(lines, &painted, width);
    let row_end = lines.len();
    fold_hits.push(row_start, 0, mw, FoldTarget::Segment(seg.id.clone()));
    activity
        .row_spans
        .insert(seg.id.clone(), row_start, row_end);
    *need_spacer = true;
}

#[allow(clippy::too_many_arguments)] // paint planes: lines, hits, segments, activity, theme
pub(super) fn paint_cluster_header_row(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
    si: usize,
    ci: usize,
    model: &UiModel,
    activity: &ActivityFoldState,
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
    let expanded = activity.cluster_kids_visible(&seg.id, &cl.id);
    let progressive = is_open_live_cluster(seg, ci, live_seg);
    let mut counts = count_cluster(&model.entries, cl);
    if let Some(id) = live_think_id_for_cluster(model, cl, progressive) {
        counts = counts.with_live_think(id);
    }
    let thought_dur = counts
        .is_thought_only()
        .then(|| thought_duration_label(model, cl))
        .flatten();
    let plain = format_cluster_header(
        glyphs,
        &counts,
        expanded,
        progressive,
        thought_dur.as_deref(),
    );
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let painted = theme.paint_muted(&plain);
    let row_start = lines.len();
    push_wrapped(lines, &painted, width);
    fold_hits.push(row_start, 0, mw, FoldTarget::Cluster(cl.id.clone()));
    *need_spacer = true;
}

/// ActivityFold on: merge streaming Think into a Thinking bar (no second L1 header).
///
/// Cluster id matches the cluster `partition_segments` will assign when the
/// stream flushes, so a user expand survives ToolStart / MessageEnd.
#[allow(clippy::too_many_arguments)] // paint planes: lines, hits, activity, theme
pub(super) fn paint_folded_streaming_thought(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    activity: &ActivityFoldState,
    model: &UiModel,
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
    live_open_cluster: bool,
    live_open_cluster_expanded: bool,
    text: &str,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    width: usize,
) {
    if live_open_cluster {
        // Header already painted (Thinking/Thought or Editing/Exploring/…).
        // Thinking stream is a kid — no second L1 row.
        if live_open_cluster_expanded {
            push_wrapped(lines, &theme.paint_muted(&format!("{text}…")), width);
        }
        return;
    }

    let (env_id, cluster_id) = next_live_thought_cluster_id(&model.entries, segments);
    let expanded = activity.cluster_kids_visible(&env_id, &cluster_id);
    let counts = streaming_thought_counts();
    let plain = format_cluster_header(glyphs, &counts, expanded, true, None);
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let painted = theme.paint_muted(&plain);
    let row_start = lines.len();
    push_wrapped(lines, &painted, width);
    fold_hits.push(row_start, 0, mw, FoldTarget::Cluster(cluster_id));
    if expanded {
        push_wrapped(lines, &theme.paint_muted(&format!("{text}…")), width);
    }
}

fn thought_duration_label(
    model: &UiModel,
    cluster: &crate::app::tui::activity_fold::ActivityCluster,
) -> Option<String> {
    let mut secs = 0u64;
    for idx in cluster_middle_indices(&model.entries, cluster) {
        if let Some(UiEntry::Thinking {
            elapsed_secs: Some(s),
            ..
        }) = model.entries.get(idx)
        {
            secs = secs.saturating_add(*s);
        }
    }
    (secs > 0).then(|| format_elapsed_secs(secs))
}

fn next_live_thought_cluster_id(
    entries: &[UiEntry],
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
) -> (String, String) {
    if let Some(seg) = segments.last() {
        let id = format!("{}:c{}", seg.id, seg.clusters.len());
        return (seg.id.clone(), id);
    }
    let user_idx = entries
        .iter()
        .rposition(|e| matches!(e, UiEntry::User { .. }))
        .unwrap_or(0);
    let env = format!("seg-{user_idx}");
    (env.clone(), format!("{env}:c0"))
}

pub(super) fn is_open_live_cluster(
    seg: &crate::app::tui::activity_fold::ActivitySegment,
    ci: usize,
    live_seg: bool,
) -> bool {
    live_seg && ci + 1 == seg.clusters.len() && seg.clusters[ci].seal_assistant_idx.is_none()
}

pub(super) fn streaming_assistant_displayable(model: &UiModel) -> bool {
    model
        .streaming_scrollback_tails()
        .iter()
        .any(|(kind, text)| *kind == "assistant" && text.chars().any(|c| !c.is_whitespace()))
}

pub(super) fn live_window_segment_idx(
    model: &UiModel,
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
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
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
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

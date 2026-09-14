//! Live scrollback rendering — Markdown / Expandable / Diff (c476 / c668).
//!
//! Morphology SSOT: `agent_demo` + `design/{markdown,expandable,diff-block,bash-mode}.md`.
//! Not a Codex TranscriptView — lines go into the engine scrollback stack.

mod cache;
mod diff;
mod live;
mod paint;
#[cfg(test)]
mod tests;

pub use cache::ScrollbackPaintCache;
// Keep `scrollback::{StreamingAssistantPaint, find_stable_markdown_prefix_end}`
// as the public module path (widgets re-exports the latter under cfg(test)).
/// Thousands-separated token count for cross-plane compaction word forms (c2810).
pub(crate) use paint::format_token_count;
#[allow(unused_imports)]
pub use paint::{StreamingAssistantPaint, find_stable_markdown_prefix_end};

use std::collections::{HashMap, HashSet};

use crate::app::tui::activity_fold::{
    ActivityFoldState, ActivitySegment, SegmentLevel, cluster_is_thought_only,
    cluster_middle_indices, cluster_omits_header, middle_entry_indices, partition_segments,
};
use crate::app::tui::bridge::{StreamingTailKind, UiEntry, UiModel, UiPhase};
use crate::app::tui::layout::LayoutTheme;

use super::fold_hit::{FoldHitTable, FoldTarget};
use super::glyphs::GlyphSet;
use cache::{emit_block_hits, entry_fingerprint};
use live::{
    is_ask_waiting, is_inflight_hidden, is_open_live_cluster, live_tail_label,
    live_window_segment_idx, paint_cluster_header_row, paint_envelope_header_row,
    paint_folded_streaming_thought, streaming_assistant_displayable,
};
use paint::{
    PaintCtx, inter_block_spacer, key_hint, paint_ask_block, paint_assistant_block,
    paint_bash_block, paint_compaction_block, paint_diff_block, paint_error_block,
    paint_scroll_notice_block, paint_streaming_assistant, paint_thinking_block, paint_todo_block,
    paint_tool_block, paint_user_block, push_wrapped,
};

/// Fold defaults + per-block overrides (att7 / att20 / att21). Not `Copy` — holds maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackFold {
    pub thinking_expanded: bool,
    /// Alt+E — tool/diff **block** show/hide detail.
    pub tools_expanded: bool,
    /// Ctrl+O — tool/bash detail **viewport** collapsed ↔ full (orthogonal to Alt+E).
    pub tools_output_expanded: bool,
    /// Per-block output-viewport overrides (Ctrl+O hint / fold-band click, att30);
    /// prefer over [`Self::tools_output_expanded`].
    pub output_overrides: HashMap<String, bool>,
    /// Alt+E — compaction summary (default collapsed; shares chord with tools).
    pub compaction_expanded: bool,
    /// Alt+E — Todo checklist (default collapsed one-line summary; c1955).
    pub todo_expanded: bool,
    /// Per-block tools-family overrides (Tool / Diff / Ask); prefer over [`Self::tools_expanded`].
    pub tools_overrides: HashMap<String, bool>,
    /// Per-id thinking overrides; prefer over [`Self::thinking_expanded`].
    pub thinking_overrides: HashMap<String, bool>,
}

impl Default for ScrollbackFold {
    fn default() -> Self {
        Self {
            thinking_expanded: false,
            // Product default: tool bodies open; Ctrl+O still clamps viewport height.
            tools_expanded: true,
            tools_output_expanded: false,
            output_overrides: HashMap::new(),
            // Product default: compaction summary collapsed (c1730 / pi).
            compaction_expanded: false,
            // Product default: Todo checklist collapsed to summary line (c1955).
            todo_expanded: false,
            tools_overrides: HashMap::new(),
            thinking_overrides: HashMap::new(),
        }
    }
}

/// Defaults that MAY full-clear the paint cache on change.
///
/// `tools_output_expanded` / `compaction_expanded` / `todo_expanded` MUST stay
/// out of this key (ath25 / att29–att30): entry fingerprints already carry them,
/// so Compaction / Ctrl+O / Todo toggles only re-paint affected blocks.
pub type ScrollbackFoldDefaultsKey = (bool, bool);

impl ScrollbackFold {
    pub fn defaults_key(&self) -> ScrollbackFoldDefaultsKey {
        (self.thinking_expanded, self.tools_expanded)
    }

    pub fn tools_effective(&self, id: &str) -> bool {
        self.tools_overrides
            .get(id)
            .copied()
            .unwrap_or(self.tools_expanded)
    }

    pub fn thinking_effective(&self, id: &str) -> bool {
        self.thinking_overrides
            .get(id)
            .copied()
            .unwrap_or(self.thinking_expanded)
    }

    pub fn toggle_tools(&mut self, id: &str) {
        let next = !self.tools_effective(id);
        self.tools_overrides.insert(id.to_string(), next);
    }

    pub fn toggle_thinking(&mut self, id: &str) {
        let next = !self.thinking_effective(id);
        self.thinking_overrides.insert(id.to_string(), next);
    }

    pub fn output_effective(&self, id: &str) -> bool {
        self.output_overrides
            .get(id)
            .copied()
            .unwrap_or(self.tools_output_expanded)
    }

    pub fn toggle_output(&mut self, id: &str) {
        let next = !self.output_effective(id);
        self.output_overrides.insert(id.to_string(), next);
    }

    pub fn clear_output_overrides(&mut self) {
        self.output_overrides.clear();
    }

    pub fn clear_tools_overrides(&mut self) {
        self.tools_overrides.clear();
    }

    pub fn clear_thinking_overrides(&mut self) {
        self.thinking_overrides.clear();
    }
}

/// Fold planning for one paint pass (planning ↔ painting split): which entry
/// indices are skipped (L2/L3 foldable middles), where envelope / cluster header
/// rows land, thought-only middles, and the live-open-cluster state that the
/// streaming thinking paint keys off.
struct SegmentPlan {
    skip_middle: HashSet<usize>,
    skip_mid_asst: HashSet<usize>,
    envelope_summary_at: HashMap<usize, usize>,
    cluster_header_at: HashMap<usize, (usize, usize)>,
    thought_only_mids: HashSet<usize>,
    live_open_cluster: bool,
    live_open_cluster_expanded: bool,
    live_open_cluster_thought_only: bool,
}

fn plan_segments(
    model: &UiModel,
    activity: &ActivityFoldState,
    segments: &[ActivitySegment],
    live_seg_idx: Option<usize>,
) -> SegmentPlan {
    let mut plan = SegmentPlan {
        skip_middle: HashSet::new(),
        skip_mid_asst: HashSet::new(),
        envelope_summary_at: HashMap::new(),
        cluster_header_at: HashMap::new(),
        thought_only_mids: HashSet::new(),
        live_open_cluster: false,
        live_open_cluster_expanded: false,
        live_open_cluster_thought_only: false,
    };
    for (si, seg) in segments.iter().enumerate() {
        let live_seg = live_seg_idx == Some(si);
        let level = activity.effective_level(&seg.id);
        // Envelope header only when the envelope is in play (L2 expanded / L3
        // collapsed). Keep-window L0 and live window stay cluster heads only.
        if activity.settings.paints_envelope_header() && !live_seg && level != SegmentLevel::L0 {
            let mids = middle_entry_indices(&model.entries, seg);
            if let Some(&first) = mids.first() {
                plan.envelope_summary_at.insert(first, si);
            }
        }
        if level == SegmentLevel::L3 {
            let mids = middle_entry_indices(&model.entries, seg);
            plan.skip_middle.extend(mids);
            plan.skip_mid_asst
                .extend(seg.mid_assistant_idxs.iter().copied());
            continue;
        }
        for (ci, cl) in seg.clusters.iter().enumerate() {
            let mids = cluster_middle_indices(&model.entries, cl);
            let omit_header = cluster_omits_header(&model.entries, cl);
            let sealed_nonempty = mids.iter().any(|&idx| {
                model
                    .entries
                    .get(idx)
                    .is_some_and(|e| !is_inflight_hidden(e) && !is_ask_waiting(e))
            });
            let is_open = is_open_live_cluster(seg, ci, live_seg);
            let has_inflight_tools = mids
                .iter()
                .any(|&idx| model.entries.get(idx).is_some_and(is_inflight_hidden));
            let expanded = activity.cluster_kids_visible(&seg.id, &cl.id);
            // Live open cluster with tools (including inflight) gets a foldable
            // cluster header. Kids stay collapsed until the user opens them —
            // paint must not auto-expand (avoids popping bodies and per-frame HashSet writes).
            let paint_header =
                !omit_header && !(is_open && !sealed_nonempty && !has_inflight_tools);
            if paint_header && let Some(&first) = mids.first() {
                plan.cluster_header_at.insert(first, (si, ci));
            }
            let thought_only = cluster_is_thought_only(&model.entries, cl);
            if thought_only {
                plan.thought_only_mids.extend(mids.iter().copied());
            }
            if is_open {
                plan.live_open_cluster = true;
                plan.live_open_cluster_expanded = expanded;
                plan.live_open_cluster_thought_only = thought_only;
            }
            for &idx in &mids {
                let Some(entry) = model.entries.get(idx) else {
                    continue;
                };
                if is_ask_waiting(entry) {
                    continue;
                }
                if omit_header {
                    // Envelope is already expanded here; show the compaction block.
                    continue;
                }
                if !expanded {
                    plan.skip_middle.insert(idx);
                }
            }
        }
    }
    plan
}

/// Render UiModel entries into scrollback lines for the product host.
///
/// Fills `fold_hits.regions` with content-relative rows (caller adds loaded-resources
/// offset). Does not touch `scroll_top` / `transcript_rows`.
///
/// L2/L3 segments skip foldable middles and emit one summary row; segment→row
/// spans land in `activity.row_spans`, and the summary fold marker is also
/// registered on [`FoldHitTable`] as [`FoldTarget::Segment`] (att31 / c2045).
#[allow(clippy::too_many_arguments)] // fold + activity + cache + hits are distinct paint planes
pub fn render_scrollback(
    model: &UiModel,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    fold: &ScrollbackFold,
    activity: &mut ActivityFoldState,
    width: usize,
    cache: &mut ScrollbackPaintCache,
    fold_hits: &mut FoldHitTable,
) -> Vec<String> {
    let width = width.max(1);
    cache.prepare(width, fold);
    fold_hits.clear_regions();
    activity.row_spans.clear();
    let mut lines = Vec::new();

    if model.entries.is_empty() && model.streaming_scrollback_tails().is_empty() {
        cache.entries.clear();
        cache.streaming_assistant.invalidate();
        return lines;
    }

    if cache.entries.len() > model.entries.len() {
        cache.entries.truncate(model.entries.len());
    }

    let segments = partition_segments(&model.entries);
    let has_asst_tail = streaming_assistant_displayable(model);
    let live_seg_idx = live_window_segment_idx(model, &segments, has_asst_tail);
    let plan = plan_segments(model, activity, &segments, live_seg_idx);
    let paint_ctx = PaintCtx {
        fold,
        glyphs,
        theme,
        width,
    };

    let mut need_spacer = false;
    for (entry_idx, entry) in model.entries.iter().enumerate() {
        if plan.skip_mid_asst.contains(&entry_idx) {
            continue;
        }
        if let Some(&si) = plan.envelope_summary_at.get(&entry_idx) {
            paint_envelope_header_row(
                &mut lines,
                fold_hits,
                activity,
                &segments[si],
                glyphs,
                theme,
                width,
                &mut need_spacer,
            );
            if activity.effective_level(&segments[si].id) == SegmentLevel::L3 {
                continue;
            }
        }
        if plan.skip_middle.contains(&entry_idx) {
            if let Some(&(si, ci)) = plan.cluster_header_at.get(&entry_idx) {
                paint_cluster_header_row(
                    &mut lines,
                    fold_hits,
                    &segments,
                    si,
                    ci,
                    model,
                    activity,
                    live_seg_idx,
                    glyphs,
                    theme,
                    width,
                    &mut need_spacer,
                );
            }
            continue;
        }
        if let Some(&(si, ci)) = plan.cluster_header_at.get(&entry_idx) {
            paint_cluster_header_row(
                &mut lines,
                fold_hits,
                &segments,
                si,
                ci,
                model,
                activity,
                live_seg_idx,
                glyphs,
                theme,
                width,
                &mut need_spacer,
            );
        }
        let glued_to_header = plan.cluster_header_at.contains_key(&entry_idx);
        if need_spacer && !glued_to_header {
            lines.push(inter_block_spacer(width));
        }
        need_spacer = true;
        let fp = entry_fingerprint(entry, fold);
        if cache
            .entries
            .get(entry_idx)
            .is_some_and(|(f, _, _)| *f == fp)
        {
            let content_row_base = lines.len();
            let (_, block_lines, hits) = &cache.entries[entry_idx];
            emit_block_hits(fold_hits, content_row_base, hits);
            lines.extend(block_lines.iter().cloned());
            continue;
        }
        let (block_lines, block_hits) = match entry {
            UiEntry::User { text } => paint_user_block(text, paint_ctx),
            UiEntry::Assistant { text } => paint_assistant_block(text, paint_ctx),
            UiEntry::Thinking {
                id,
                text,
                elapsed_secs,
            } => paint_thinking_block(
                id,
                text,
                *elapsed_secs,
                plan.thought_only_mids.contains(&entry_idx),
                paint_ctx,
            ),
            UiEntry::Tool {
                timeout_secs,
                id,
                name,
                args_preview,
                write_content,
                display_diff,
                output,
                is_error,
                done,
                ..
            } => paint_tool_block(
                id,
                name,
                args_preview,
                *timeout_secs,
                write_content.as_deref(),
                display_diff.as_deref(),
                output,
                *is_error,
                *done,
                paint_ctx,
            ),
            UiEntry::Diff {
                summary,
                display_diff,
            } => paint_diff_block(summary, display_diff, paint_ctx),
            UiEntry::Bash {
                id,
                command,
                status,
                output,
                ..
            } => paint_bash_block(id, command, *status, output, paint_ctx),
            UiEntry::Ask {
                id,
                summary,
                detail_lines,
                phase,
                ..
            } => paint_ask_block(id, summary, detail_lines, *phase, paint_ctx),
            UiEntry::Compaction {
                status,
                summary,
                tokens_before,
                tokens_after,
                detail,
            } => paint_compaction_block(
                *status,
                summary,
                *tokens_before,
                *tokens_after,
                detail.as_deref(),
                paint_ctx,
            ),
            UiEntry::Todo {
                summary,
                detail_lines,
            } => paint_todo_block(summary, detail_lines, paint_ctx),
            UiEntry::ScrollNotice { text } => paint_scroll_notice_block(text, paint_ctx),
            UiEntry::Error { text } => paint_error_block(text, paint_ctx),
        };
        cache.entry_misses = cache.entry_misses.saturating_add(1);
        if entry_idx < cache.entries.len() {
            cache.entries.truncate(entry_idx);
        }
        let content_row_base = lines.len();
        emit_block_hits(fold_hits, content_row_base, &block_hits);
        cache.entries.push((fp, block_lines.clone(), block_hits));
        lines.extend(block_lines);
    }

    let streaming_tails = model.streaming_scrollback_tails();
    if !streaming_tails
        .iter()
        .any(|(kind, _)| *kind == StreamingTailKind::Assistant)
    {
        cache.streaming_assistant.invalidate();
    }

    for (kind, text) in streaming_tails {
        if need_spacer {
            lines.push(inter_block_spacer(width));
        }
        need_spacer = true;
        match kind {
            StreamingTailKind::Thinking => {
                if activity.settings.enabled {
                    paint_folded_streaming_thought(
                        &mut lines,
                        fold_hits,
                        fold,
                        activity,
                        model,
                        &segments,
                        plan.live_open_cluster,
                        plan.live_open_cluster_expanded,
                        plan.live_open_cluster_thought_only,
                        text,
                        glyphs,
                        theme,
                        width,
                    );
                } else {
                    // att8 / att21: without ActivityFold, streaming thinking stays expanded.
                    let marker = glyphs.unfold();
                    let header =
                        theme.paint_muted(&format!("{marker} Thinking  {}", key_hint("Ctrl+T")));
                    push_wrapped(&mut lines, &header, width);
                    push_wrapped(&mut lines, &theme.paint_muted(&format!("{text}…")), width);
                }
            }
            StreamingTailKind::Assistant => {
                lines.extend(paint_streaming_assistant(
                    text,
                    width,
                    theme,
                    &mut cache.streaming_assistant,
                ));
            }
        }
    }

    if activity.settings.enabled
        && model.phase == UiPhase::Busy
        && let Some(label) = live_tail_label(model, &segments, has_asst_tail)
    {
        if need_spacer {
            lines.push(inter_block_spacer(width));
        }
        let painted = theme.paint_muted(&label);
        let row_start = lines.len();
        push_wrapped(&mut lines, &painted, width);
        fold_hits.push(row_start, 0, width.max(1), FoldTarget::LiveTail);
    }

    lines
}

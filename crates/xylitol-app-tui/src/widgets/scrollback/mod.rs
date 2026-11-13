mod cache;
mod diff;
mod live;
mod paint;
#[cfg(test)]
mod tests;

pub use cache::ScrollbackPaintCache;
// as the public module path (widgets re-exports the latter under cfg(test)).
pub(crate) use paint::format_token_count;
#[allow(unused_imports)]
pub use paint::{StreamingAssistantPaint, find_stable_markdown_prefix_end};

use std::collections::{HashMap, HashSet};

use crate::activity_fold::{
    ActivityFoldState, ActivitySegment, SegmentLevel, VisibleItems, cluster_is_thought_only,
    cluster_middle_indices, cluster_omits_header, middle_entry_indices, partition_segments,
};
use crate::bridge::{STREAMING_THINK_ID, StreamingTailKind, UiEntry, UiModel, UiPhase};
use crate::layout::LayoutTheme;

use super::fold_hit::{FoldHitTable, FoldTarget};
use super::glyphs::GlyphSet;
use cache::{emit_block_hits, entry_fingerprint};
use live::{
    is_ask_waiting, is_inflight_hidden, is_open_live_cluster, live_tail_label,
    live_window_segment_idx, paint_cluster_header_row, paint_envelope_header_row,
    paint_folded_streaming_thought, streaming_assistant_displayable,
};
use paint::{
    PaintCtx, inter_block_spacer, paint_ask_block, paint_assistant_block, paint_bash_block,
    paint_compaction_block, paint_diff_block, paint_error_block, paint_scroll_notice_block,
    paint_streaming_assistant, paint_system_message_block, paint_thinking_block, paint_tool_block,
    paint_user_block, push_wrapped,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgsMode {
    Hidden,
    Preview,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackFoldDefaults {
    pub thinking_expanded: bool,
    pub thinking_visible: bool,
    pub streaming_thinking_expanded: bool,
    pub tools_expanded: bool,
    pub tool_header_visible: bool,
    pub tools_output_expanded: bool,
    pub tool_output_visible: bool,
    pub diff_visible: bool,
    pub args_expanded: ArgsMode,
    pub user_expanded: bool,
    pub compaction_expanded: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScrollbackFoldOverrides {
    pub output: HashMap<String, bool>,
    pub args: HashMap<String, ArgsMode>,
    pub user: HashMap<String, bool>,
    pub tools: HashMap<String, bool>,
    pub thinking: HashMap<String, bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScrollbackFold {
    pub defaults: ScrollbackFoldDefaults,
    pub overrides: ScrollbackFoldOverrides,
}

impl Default for ScrollbackFoldDefaults {
    fn default() -> Self {
        Self {
            thinking_expanded: false,
            thinking_visible: true,
            streaming_thinking_expanded: true,
            tools_expanded: true,
            tool_header_visible: true,
            tools_output_expanded: false,
            tool_output_visible: true,
            diff_visible: true,
            args_expanded: ArgsMode::Hidden,
            user_expanded: false,
            compaction_expanded: false,
        }
    }
}

pub type ScrollbackFoldDefaultsKey = (bool, bool, ArgsMode, bool);

impl ScrollbackFold {
    pub fn defaults_key(&self) -> ScrollbackFoldDefaultsKey {
        (
            self.defaults.thinking_expanded,
            self.defaults.tools_expanded,
            self.defaults.args_expanded,
            self.defaults.user_expanded,
        )
    }

    pub fn tools_effective(&self, id: &str) -> bool {
        self.overrides
            .tools
            .get(id)
            .copied()
            .unwrap_or(self.defaults.tools_expanded)
    }

    pub fn thinking_effective(&self, id: &str) -> bool {
        self.defaults.thinking_visible
            && self.overrides.thinking.get(id).copied().unwrap_or_else(|| {
                if id == STREAMING_THINK_ID {
                    self.defaults.streaming_thinking_expanded
                } else {
                    self.defaults.thinking_expanded
                }
            })
    }

    pub fn toggle_tools(&mut self, id: &str) {
        let next = !self.tools_effective(id);
        self.overrides.tools.insert(id.to_string(), next);
    }

    pub fn toggle_thinking(&mut self, id: &str) {
        let next = !self.thinking_effective(id);
        self.overrides.thinking.insert(id.to_string(), next);
    }

    pub fn output_effective(&self, id: &str) -> bool {
        self.defaults.tool_output_visible
            && self
                .overrides
                .output
                .get(id)
                .copied()
                .unwrap_or(self.defaults.tools_output_expanded)
    }

    pub fn apply_visible_items(&mut self, items: VisibleItems) {
        self.defaults.thinking_visible = items.thinking;
        self.defaults.tool_header_visible = items.tool_header;
        self.defaults.tool_output_visible = items.tool_output;
        self.defaults.diff_visible = items.diff;
        self.defaults.thinking_expanded = items.tool_output;
        self.defaults.streaming_thinking_expanded = items.tool_output;
        self.defaults.tools_expanded = items.tool_output;
        self.defaults.args_expanded = if items.tool_output {
            ArgsMode::Full
        } else {
            ArgsMode::Hidden
        };
        self.defaults.user_expanded = items.tool_output;
    }

    pub fn toggle_output(&mut self, id: &str) {
        let next = !self.output_effective(id);
        self.overrides.output.insert(id.to_string(), next);
    }

    pub fn args_effective(&self, id: &str) -> ArgsMode {
        self.overrides
            .args
            .get(id)
            .copied()
            .unwrap_or(self.defaults.args_expanded)
    }

    pub fn toggle_args(&mut self, id: &str) {
        let next = match self.args_effective(id) {
            ArgsMode::Hidden => ArgsMode::Preview,
            ArgsMode::Preview => ArgsMode::Full,
            ArgsMode::Full => ArgsMode::Hidden,
        };
        self.overrides.args.insert(id.to_string(), next);
    }

    pub fn user_effective(&self, id: &str) -> bool {
        self.overrides
            .user
            .get(id)
            .copied()
            .unwrap_or(self.defaults.user_expanded)
    }

    pub fn toggle_user(&mut self, id: &str) {
        let next = !self.user_effective(id);
        self.overrides.user.insert(id.to_string(), next);
    }

    pub fn clear_args_overrides(&mut self) {
        self.overrides.args.clear();
    }

    pub fn clear_user_overrides(&mut self) {
        self.overrides.user.clear();
    }

    pub fn clear_output_overrides(&mut self) {
        self.overrides.output.clear();
    }

    pub fn clear_tools_overrides(&mut self) {
        self.overrides.tools.clear();
    }

    pub fn clear_thinking_overrides(&mut self) {
        self.overrides.thinking.clear();
    }
}

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
        let level = activity.level_of(&seg.id);
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

#[allow(clippy::too_many_arguments)]
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
            if activity.level_of(&segments[si].id) == SegmentLevel::L3 {
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
                    fold,
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
                fold,
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
            UiEntry::User { text } => paint_user_block(text, text, paint_ctx),
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
                args_full,
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
                args_full,
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
                id: _,
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
            UiEntry::ScrollNotice { text } => paint_scroll_notice_block(text, paint_ctx),
            UiEntry::SystemMessage { text } => paint_system_message_block(text, paint_ctx),
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
                paint_folded_streaming_thought(
                    &mut lines,
                    fold_hits,
                    fold,
                    text,
                    activity.settings.enabled && plan.live_open_cluster,
                    activity.settings.enabled && plan.live_open_cluster_thought_only,
                    glyphs,
                    theme,
                    width,
                );
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

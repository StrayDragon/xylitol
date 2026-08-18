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
pub use diff::diff_fold_key;
// Keep `scrollback::{StreamingAssistantPaint, find_stable_markdown_prefix_end}`
// as the public module path (widgets re-exports the latter under cfg(test)).
#[allow(unused_imports)]
pub use paint::{StreamingAssistantPaint, find_stable_markdown_prefix_end};

use std::collections::{HashMap, HashSet};

use xylitol_tui::{Component, ExpandableOutputOptions, Markdown, TruncateFrom, mix_rgb};

use crate::app::tui::activity_fold::{
    ActivityFoldState, SegmentLevel, cluster_is_thought_only, cluster_middle_indices,
    cluster_omits_header, format_elapsed_secs, middle_entry_indices, partition_segments,
    thought_header_body,
};
use crate::app::tui::bridge::{BashBlockStatus, CompactionBlockStatus, UiEntry, UiModel, UiPhase};
use crate::app::tui::layout::LayoutTheme;

use super::fold_hit::{FoldHitTable, FoldTarget};
use super::glyphs::GlyphSet;
use cache::{CachedFoldHit, emit_block_hits, entry_fingerprint, marker_cols};
use diff::{
    HARD_TRUNCATED_EXPAND_HINT, RAILED_MARKER_COL, TOOLS_OUTPUT_PREVIEW_LINES,
    WRITE_BODY_PREVIEW_LINES, output_is_hard_truncated, paint_output_with_full_footer,
    push_expandable_with_viewport_hit, push_viewport_diff_lines,
};
use live::{
    is_ask_waiting, is_inflight_hidden, is_open_live_cluster, live_tail_label,
    live_window_segment_idx, paint_cluster_header_row, paint_envelope_header_row,
    paint_folded_streaming_thought, streaming_assistant_displayable,
};
use paint::{
    ask_rail_rgb, bash_rail_rgb, fit, format_token_count, highlight_dollar_skill_refs,
    inter_block_spacer, key_hint, paint_ask_header_line, paint_streaming_assistant,
    paint_tool_header_line, push_railed, push_wrapped, rail_inner_width, tool_rail_rgb,
};

/// Fold defaults + per-block overrides (att7 / att20 / att21). Not `Copy` — holds maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackFold {
    pub thinking_expanded: bool,
    /// Alt+E — tool/diff **block** show/hide detail.
    pub tools_expanded: bool,
    /// Ctrl+O — tool/bash detail **viewport** collapsed ↔ full (orthogonal to Alt+E).
    pub tools_output_expanded: bool,
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

    pub fn clear_tools_overrides(&mut self) {
        self.tools_overrides.clear();
    }

    pub fn clear_thinking_overrides(&mut self) {
        self.thinking_overrides.clear();
    }
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
    let mut skip_middle: HashSet<usize> = HashSet::new();
    let mut skip_mid_asst: HashSet<usize> = HashSet::new();
    let mut envelope_summary_at: HashMap<usize, usize> = HashMap::new();
    let mut cluster_header_at: HashMap<usize, (usize, usize)> = HashMap::new();
    let mut thought_only_mids: HashSet<usize> = HashSet::new();
    // Live open cluster only — used by the thinking stream paint. Do not scan
    // older turns (a prior Thought cluster must not swallow the live stream).
    let mut live_open_cluster = false;
    let mut live_open_cluster_expanded = false;
    for (si, seg) in segments.iter().enumerate() {
        let live_seg = live_seg_idx == Some(si);
        let level = activity.effective_level(&seg.id);
        // Envelope header only when the envelope is in play (L2 expanded / L3
        // collapsed). Keep-window L0 and live window stay cluster heads only.
        if activity.settings.paints_envelope_header() && !live_seg && level != SegmentLevel::L0 {
            let mids = middle_entry_indices(&model.entries, seg);
            if let Some(&first) = mids.first() {
                envelope_summary_at.insert(first, si);
            }
        }
        if level == SegmentLevel::L3 {
            let mids = middle_entry_indices(&model.entries, seg);
            skip_middle.extend(mids);
            skip_mid_asst.extend(seg.mid_assistant_idxs.iter().copied());
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
                cluster_header_at.insert(first, (si, ci));
            }
            let thought_only = cluster_is_thought_only(&model.entries, cl);
            if thought_only {
                thought_only_mids.extend(mids.iter().copied());
            }
            if is_open {
                live_open_cluster = true;
                live_open_cluster_expanded = expanded;
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
                    skip_middle.insert(idx);
                }
            }
        }
    }

    let mut need_spacer = false;
    for (entry_idx, entry) in model.entries.iter().enumerate() {
        if skip_mid_asst.contains(&entry_idx) {
            continue;
        }
        if let Some(&si) = envelope_summary_at.get(&entry_idx) {
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
        if skip_middle.contains(&entry_idx) {
            if let Some(&(si, ci)) = cluster_header_at.get(&entry_idx) {
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
        if let Some(&(si, ci)) = cluster_header_at.get(&entry_idx) {
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
        let glued_to_header = cluster_header_at.contains_key(&entry_idx);
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
        let mut block_hits = Vec::new();
        let block_lines = {
            let mut lines = Vec::new();
            match entry {
                UiEntry::User { text } => {
                    let prefix = theme.paint_user(glyphs.user());
                    let painted = highlight_dollar_skill_refs(text, theme.palette().skill_ref);
                    let body = format!("{prefix} {painted}");
                    // Flush — no user-message-bg wash, no status rail (atc8 / c1830).
                    push_wrapped(&mut lines, &body, width);
                }
                UiEntry::Assistant { text } => {
                    let mut md =
                        Markdown::new(text.clone(), 0, 0, theme.palette().markdown_theme(), None);
                    for line in md.render(width) {
                        lines.push(fit(&line, width));
                    }
                }
                UiEntry::Thinking {
                    id,
                    text,
                    elapsed_secs,
                } => {
                    if thought_only_mids.contains(&entry_idx) {
                        // Cluster header is already Thought; don't paint a second L1 row.
                        push_wrapped(&mut lines, &theme.paint_muted(text), width);
                    } else {
                        let expanded = fold.thinking_effective(id);
                        let marker = if expanded {
                            glyphs.unfold()
                        } else {
                            glyphs.fold()
                        };
                        let mw = marker_cols(marker);
                        let dur = elapsed_secs.filter(|s| *s > 0).map(format_elapsed_secs);
                        let label = thought_header_body(dur.as_deref());
                        let header =
                            theme.paint_muted(&format!("{marker} {label}  {}", key_hint("Ctrl+T")));
                        let header_row = lines.len();
                        push_wrapped(&mut lines, &header, width);
                        block_hits.push(CachedFoldHit {
                            row_offset: header_row,
                            col_start: 0,
                            col_end: mw,
                            target: FoldTarget::Thinking(id.clone()),
                        });
                        if expanded {
                            push_wrapped(&mut lines, &theme.paint_muted(text), width);
                        }
                    }
                }
                UiEntry::Tool {
                    id,
                    name,
                    args_preview,
                    write_content,
                    display_diff,
                    output,
                    is_error,
                    done,
                    ..
                } => {
                    let inner = rail_inner_width(width);
                    let expanded = fold.tools_effective(id);
                    let marker = if expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let mw = marker_cols(marker);
                    let header = paint_tool_header_line(theme, marker, name, args_preview);
                    let rgb = tool_rail_rgb(!done, *is_error, theme);
                    let mut block = Vec::new();
                    push_wrapped(&mut block, &header, inner);
                    block_hits.push(CachedFoldHit {
                        row_offset: 0,
                        col_start: RAILED_MARKER_COL,
                        col_end: RAILED_MARKER_COL + mw,
                        target: FoldTarget::Tool(id.clone()),
                    });

                    if expanded {
                        if let Some(content) = write_content
                            && !content.is_empty()
                        {
                            let total = content.lines().count().max(1);
                            let opts = ExpandableOutputOptions {
                                max_preview_lines: WRITE_BODY_PREVIEW_LINES,
                                from: TruncateFrom::Tail,
                                expand_hint: format!("{total} total, ctrl+o to expand"),
                                hint_style: None,
                            };
                            push_expandable_with_viewport_hit(
                                &mut block,
                                &mut block_hits,
                                content,
                                inner,
                                fold.tools_output_expanded,
                                &opts,
                                RAILED_MARKER_COL,
                            );
                        }

                        if !output.is_empty() {
                            let painted = paint_output_with_full_footer(output, theme, *is_error);
                            let hard = output_is_hard_truncated(output);
                            let opts = ExpandableOutputOptions {
                                max_preview_lines: TOOLS_OUTPUT_PREVIEW_LINES,
                                from: TruncateFrom::Tail,
                                expand_hint: if hard {
                                    HARD_TRUNCATED_EXPAND_HINT.into()
                                } else {
                                    "ctrl+o to expand".into()
                                },
                                hint_style: None,
                            };
                            let viewport = fold.tools_output_expanded && !hard;
                            push_expandable_with_viewport_hit(
                                &mut block,
                                &mut block_hits,
                                &painted,
                                inner,
                                viewport,
                                &opts,
                                RAILED_MARKER_COL,
                            );
                        }

                        if let Some(diff) = display_diff
                            && !diff.is_empty()
                        {
                            if !block.is_empty() {
                                block.push(String::new());
                            }
                            push_viewport_diff_lines(
                                &mut block,
                                &mut block_hits,
                                diff,
                                inner,
                                theme,
                                fold.tools_output_expanded,
                            );
                        }
                    }

                    push_railed(&mut lines, &block, width, rgb);
                }
                UiEntry::Diff {
                    summary,
                    display_diff,
                } => {
                    let inner = rail_inner_width(width);
                    let key = diff_fold_key(summary, display_diff);
                    let expanded = fold.tools_effective(&key);
                    let marker = if expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let mw = marker_cols(marker);
                    let header = paint_tool_header_line(theme, marker, "diff", summary);
                    let mut block = Vec::new();
                    push_wrapped(&mut block, &header, inner);
                    block_hits.push(CachedFoldHit {
                        row_offset: 0,
                        col_start: RAILED_MARKER_COL,
                        col_end: RAILED_MARKER_COL + mw,
                        target: FoldTarget::Diff(key),
                    });
                    let rgb = tool_rail_rgb(false, false, theme);
                    if expanded && !display_diff.is_empty() {
                        block.push(String::new());
                        push_viewport_diff_lines(
                            &mut block,
                            &mut block_hits,
                            display_diff,
                            inner,
                            theme,
                            fold.tools_output_expanded,
                        );
                    }
                    push_railed(&mut lines, &block, width, rgb);
                }
                UiEntry::Bash {
                    command,
                    status,
                    output,
                    ..
                } => {
                    let inner = rail_inner_width(width);
                    let mut block = Vec::new();
                    push_wrapped(
                        &mut block,
                        &theme.paint_success(&format!("$ {command}")),
                        inner,
                    );
                    if !output.is_empty() {
                        let body = paint_output_with_full_footer(
                            output,
                            theme,
                            matches!(status, BashBlockStatus::Error | BashBlockStatus::Cancelled),
                        );
                        let hard = output_is_hard_truncated(output);
                        let opts = ExpandableOutputOptions {
                            max_preview_lines: TOOLS_OUTPUT_PREVIEW_LINES,
                            from: TruncateFrom::Tail,
                            expand_hint: if hard {
                                HARD_TRUNCATED_EXPAND_HINT.into()
                            } else {
                                "ctrl+o to expand".into()
                            },
                            hint_style: None,
                        };
                        let expanded = fold.tools_output_expanded && !hard;
                        push_expandable_with_viewport_hit(
                            &mut block,
                            &mut block_hits,
                            &body,
                            inner,
                            expanded,
                            &opts,
                            RAILED_MARKER_COL,
                        );
                    } else if matches!(status, BashBlockStatus::Pending) {
                        push_wrapped(
                            &mut block,
                            &theme.paint_muted(&format!("Running… {}", key_hint("Esc"))),
                            inner,
                        );
                    }
                    push_railed(&mut lines, &block, width, bash_rail_rgb(*status, theme));
                }
                UiEntry::Ask {
                    id,
                    summary,
                    detail_lines,
                    phase,
                    ..
                } => {
                    let inner = rail_inner_width(width);
                    let expanded = fold.tools_effective(id);
                    let marker = if expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let mw = marker_cols(marker);
                    let header = paint_ask_header_line(theme, marker, summary, inner);
                    let mut block = vec![fit(&header, inner)];
                    block_hits.push(CachedFoldHit {
                        row_offset: 0,
                        col_start: RAILED_MARKER_COL,
                        col_end: RAILED_MARKER_COL + mw,
                        target: FoldTarget::Ask(id.clone()),
                    });
                    if expanded {
                        for line in detail_lines {
                            block.push(fit(&theme.paint_muted(line), inner));
                        }
                    }
                    push_railed(&mut lines, &block, width, ask_rail_rgb(*phase, theme));
                }
                UiEntry::Compaction {
                    status,
                    summary,
                    tokens_before,
                    detail,
                } => match status {
                    CompactionBlockStatus::Pending => {
                        push_wrapped(
                            &mut lines,
                            &theme.paint_muted("[compaction] Compacting…"),
                            width,
                        );
                    }
                    CompactionBlockStatus::Complete => {
                        let n = format_token_count(*tokens_before);
                        let marker = if fold.compaction_expanded {
                            glyphs.unfold()
                        } else {
                            glyphs.fold()
                        };
                        let mw = marker_cols(marker);
                        let header = if fold.compaction_expanded {
                            format!("{marker} [compaction] Compacted from {n} tokens")
                        } else {
                            format!(
                                "{marker} [compaction] Compacted from {n} tokens (Alt+E to expand)"
                            )
                        };
                        let header_row = lines.len();
                        push_wrapped(&mut lines, &theme.paint_muted(&header), width);
                        block_hits.push(CachedFoldHit {
                            row_offset: header_row,
                            col_start: 0,
                            col_end: mw,
                            target: FoldTarget::Compaction,
                        });
                        if fold.compaction_expanded && !summary.is_empty() {
                            lines.push(String::new());
                            push_wrapped(&mut lines, &theme.paint_muted(summary), width);
                        }
                    }
                    CompactionBlockStatus::Aborted | CompactionBlockStatus::Failed => {
                        let text = detail.as_deref().unwrap_or("compaction aborted");
                        push_wrapped(
                            &mut lines,
                            &theme.paint_muted(&format!("[compaction] {text}")),
                            width,
                        );
                    }
                },
                UiEntry::Todo {
                    summary,
                    detail_lines,
                } => {
                    let inner = rail_inner_width(width);
                    let expanded = fold.todo_expanded;
                    let marker = if expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let mw = marker_cols(marker);
                    let hint = if expanded {
                        String::new()
                    } else {
                        format!("  {}", key_hint("Alt+E"))
                    };
                    let header = format!("{marker} {summary}{hint}");
                    let mut block = vec![fit(&theme.paint_muted(&header), inner)];
                    block_hits.push(CachedFoldHit {
                        row_offset: 0,
                        col_start: RAILED_MARKER_COL,
                        col_end: RAILED_MARKER_COL + mw,
                        target: FoldTarget::Todo,
                    });
                    if expanded {
                        for line in detail_lines {
                            block.push(fit(&theme.paint_muted(line), inner));
                        }
                    }
                    let rail = {
                        let p = theme.palette();
                        mix_rgb(p.surface, p.muted, 0.72)
                    };
                    push_railed(&mut lines, &block, width, rail);
                }
                UiEntry::ScrollNotice { text } => {
                    push_wrapped(
                        &mut lines,
                        &theme.paint_muted(&format!("{} {text}", glyphs.system())),
                        width,
                    );
                }
                UiEntry::Error { text } => {
                    push_wrapped(
                        &mut lines,
                        &theme.paint_error(&format!("error: {text}")),
                        width,
                    );
                }
            }
            lines // end block paint
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
    if !streaming_tails.iter().any(|(kind, _)| *kind == "assistant") {
        cache.streaming_assistant.invalidate();
    }

    for (kind, text) in streaming_tails {
        if need_spacer {
            lines.push(inter_block_spacer(width));
        }
        need_spacer = true;
        match kind {
            "thinking" => {
                if activity.settings.enabled {
                    paint_folded_streaming_thought(
                        &mut lines,
                        fold_hits,
                        activity,
                        model,
                        &segments,
                        live_open_cluster,
                        live_open_cluster_expanded,
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
            "assistant" => {
                lines.extend(paint_streaming_assistant(
                    text,
                    width,
                    theme,
                    &mut cache.streaming_assistant,
                ));
            }
            _ => {}
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

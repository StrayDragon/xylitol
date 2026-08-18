use xylitol_tui::visible_width;

use crate::app::tui::bridge::UiEntry;

use super::super::fold_hit::{FoldHitTable, FoldTarget};
use super::diff::diff_fold_key;
use super::paint::StreamingAssistantPaint;
use super::{ScrollbackFold, ScrollbackFoldDefaultsKey};

/// Relative fold-hit within a cached block (row offset from block start).
#[derive(Debug, Clone)]
pub(super) struct CachedFoldHit {
    pub(super) row_offset: usize,
    pub(super) col_start: usize,
    pub(super) col_end: usize,
    pub(super) target: FoldTarget,
}

/// Per-entry paint cache so streaming/spinner frames do not re-Markdown the
/// entire transcript (ath25).
#[derive(Debug, Default)]
pub struct ScrollbackPaintCache {
    width: usize,
    fold_defaults: Option<ScrollbackFoldDefaultsKey>,
    pub(super) entries: Vec<(u64, Vec<String>, Vec<CachedFoldHit>)>,
    /// Test/obs: how many committed entries were freshly painted.
    pub(crate) entry_misses: u64,
    /// Streaming assistant incremental paint (ath26).
    pub(crate) streaming_assistant: StreamingAssistantPaint,
}

impl ScrollbackPaintCache {
    pub fn invalidate(&mut self) {
        self.entries.clear();
        self.width = 0;
        self.fold_defaults = None;
        self.streaming_assistant.invalidate();
        // keep entry_misses / stream counters cumulative unless cleared
    }

    #[cfg(test)]
    pub fn clear_misses(&mut self) {
        self.entry_misses = 0;
    }

    /// Full-clear only on width / fold **defaults** change — not override-map identity.
    pub(super) fn prepare(&mut self, width: usize, fold: &ScrollbackFold) {
        let key = fold.defaults_key();
        if self.width != width || self.fold_defaults != Some(key) {
            self.entries.clear();
            self.streaming_assistant.invalidate();
            self.width = width;
            self.fold_defaults = Some(key);
        }
    }
}

pub(super) fn entry_fingerprint(entry: &UiEntry, fold: &ScrollbackFold) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::mem::discriminant(entry).hash(&mut h);
    match entry {
        UiEntry::User { text }
        | UiEntry::Assistant { text }
        | UiEntry::ScrollNotice { text }
        | UiEntry::Error { text } => text.hash(&mut h),
        UiEntry::Thinking {
            id,
            text,
            elapsed_secs,
        } => {
            id.hash(&mut h);
            text.hash(&mut h);
            elapsed_secs.hash(&mut h);
            fold.thinking_effective(id).hash(&mut h);
        }
        UiEntry::Tool {
            id,
            name,
            args_preview,
            tool_path,
            write_content,
            display_diff,
            output,
            is_error,
            done,
        } => {
            id.hash(&mut h);
            name.hash(&mut h);
            args_preview.hash(&mut h);
            tool_path.hash(&mut h);
            write_content.hash(&mut h);
            display_diff.hash(&mut h);
            output.hash(&mut h);
            is_error.hash(&mut h);
            done.hash(&mut h);
            fold.tools_effective(id).hash(&mut h);
            fold.tools_output_expanded.hash(&mut h);
        }
        UiEntry::Diff {
            summary,
            display_diff,
        } => {
            summary.hash(&mut h);
            display_diff.hash(&mut h);
            let key = diff_fold_key(summary, display_diff);
            fold.tools_effective(&key).hash(&mut h);
            fold.tools_output_expanded.hash(&mut h);
        }
        UiEntry::Bash {
            command,
            status,
            output,
            exclude_from_context,
        } => {
            command.hash(&mut h);
            status.hash(&mut h);
            output.hash(&mut h);
            exclude_from_context.hash(&mut h);
            fold.tools_output_expanded.hash(&mut h);
        }
        UiEntry::Ask {
            id,
            summary,
            detail_lines,
            phase,
            expanded,
        } => {
            id.hash(&mut h);
            summary.hash(&mut h);
            detail_lines.hash(&mut h);
            phase.hash(&mut h);
            expanded.hash(&mut h);
            fold.tools_effective(id).hash(&mut h);
        }
        UiEntry::Compaction {
            status,
            summary,
            tokens_before,
            detail,
        } => {
            status.hash(&mut h);
            summary.hash(&mut h);
            tokens_before.hash(&mut h);
            detail.hash(&mut h);
            fold.compaction_expanded.hash(&mut h);
        }
        UiEntry::Todo {
            summary,
            detail_lines,
        } => {
            summary.hash(&mut h);
            detail_lines.hash(&mut h);
            fold.todo_expanded.hash(&mut h);
        }
    }
    h.finish()
}

pub(super) fn marker_cols(marker: &str) -> usize {
    visible_width(marker).max(1)
}

pub(super) fn emit_block_hits(
    fold_hits: &mut FoldHitTable,
    content_row_base: usize,
    hits: &[CachedFoldHit],
) {
    for h in hits {
        fold_hits.push(
            content_row_base.saturating_add(h.row_offset),
            h.col_start,
            h.col_end,
            h.target.clone(),
        );
    }
}

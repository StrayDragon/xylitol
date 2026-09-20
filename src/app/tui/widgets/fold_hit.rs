//! Fold hit table for ApplicationOwned mouse (c2040 / c2045 / att22 / ath33).

/// Per-block / global fold target addressed by mouse hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoldTarget {
    Tool(String),
    Diff(String),
    Ask(String),
    Thinking(String),
    /// 待办栏 doing 列翼头（in_progress）。
    TodoDoing,
    /// 待办栏 pending 列翼头。
    TodoPending,
    /// 待办栏 completed 列翼头（completed / cancelled）。
    TodoPast,
    /// Global `compaction_expanded` (Wave A / att29).
    Compaction,
    /// Per-block output-viewport fold (Ctrl+O semantics; att30); id = block fold
    /// key (Tool `toolCallId` / Diff `diff_fold_key` / Bash id).
    OutputViewport(String),
    /// Activity envelope summary (`Worked for` / att31); id = `seg-{user_idx}`.
    Segment(String),
    /// Activity cluster header; id = `seg-{user_idx}:c{ord}`.
    Cluster(String),
    /// Asking questions (Ask waiting): whole-line hit expands the live cluster.
    LiveTail,
}

/// One triangle-column hit region in content coordinates (scrollback line space).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldHitRegion {
    pub content_row: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub target: FoldTarget,
}

/// Live fold hit regions + viewport mapping for screen → content hit tests.
#[derive(Debug, Clone, Default)]
pub struct FoldHitTable {
    pub regions: Vec<FoldHitRegion>,
    pub scroll_top: usize,
    pub transcript_rows: u16,
}

impl FoldHitTable {
    pub fn clear_regions(&mut self) {
        self.regions.clear();
    }

    pub fn push(
        &mut self,
        content_row: usize,
        col_start: usize,
        col_end: usize,
        target: FoldTarget,
    ) {
        if col_end > col_start {
            self.regions.push(FoldHitRegion {
                content_row,
                col_start,
                col_end,
                target,
            });
        }
    }

    /// Map screen cell to a fold target. `content_row = scroll_top + screen_row`.
    pub fn hit(&self, screen_col: u16, screen_row: u16) -> Option<FoldTarget> {
        if screen_row >= self.transcript_rows {
            return None;
        }
        let content_row = self.scroll_top.saturating_add(screen_row as usize);
        let col = screen_col as usize;
        self.regions
            .iter()
            .find(|r| r.content_row == content_row && col >= r.col_start && col < r.col_end)
            .map(|r| r.target.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_hit_table_maps_screen_to_content() {
        let mut table = FoldHitTable {
            scroll_top: 10,
            transcript_rows: 5,
            ..FoldHitTable::default()
        };
        table.push(12, 2, 3, FoldTarget::Tool("t1".into()));
        assert_eq!(
            table.hit(2, 2),
            Some(FoldTarget::Tool("t1".into())),
            "row 2 + scroll 10 → content 12"
        );
        assert_eq!(table.hit(3, 2), None, "outside triangle column");
        assert_eq!(table.hit(2, 5), None, "below transcript pane");
    }
}

//! Fold hit table for ApplicationOwned mouse (c2040 / c2860).

use xylitol_tui::{Hit, HitId};

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

/// One hit region in content coordinates (scrollback line space).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldHitRegion {
    pub content_row: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub height: usize,
    pub target: FoldTarget,
}

/// Live fold hit regions + viewport mapping for screen → content hit tests.
#[derive(Debug, Clone, Default)]
pub struct FoldHitTable {
    pub regions: Vec<FoldHitRegion>,
    pub scroll_top: usize,
    pub transcript_rows: u16,
}

/// Left rail + gutter before a railed marker (`paint_left_rail_line`).
const RAILED_HOVER_COLS: usize = 2;

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
        self.push_rect(content_row, col_start, col_end, 1, target);
    }

    pub fn push_rect(
        &mut self,
        content_row: usize,
        col_start: usize,
        col_end: usize,
        height: usize,
        target: FoldTarget,
    ) {
        if col_end > col_start {
            self.regions.push(FoldHitRegion {
                content_row,
                col_start,
                col_end,
                height: height.max(1),
                target,
            });
        }
    }

    /// Map screen cell to a fold target. Later regions win (hint over body).
    pub fn hit(&self, screen_col: u16, screen_row: u16) -> Option<FoldTarget> {
        if screen_row >= self.transcript_rows {
            return None;
        }
        let content_row = self.scroll_top.saturating_add(screen_row as usize);
        let col = screen_col as usize;
        self.regions
            .iter()
            .rev()
            .find(|r| region_contains(r, content_row, col))
            .map(|r| r.target.clone())
    }

    pub fn pointer_click_hits(&self) -> Vec<Hit> {
        self.regions
            .iter()
            .enumerate()
            .map(|(i, r)| {
                Hit::on_line(id_of(i), r.content_row)
                    .cols(r.col_start, r.col_end)
                    .tall(r.height)
            })
            .collect()
    }

    /// Dim 块头 / 可见左边轨 / 提示带 — not the payload body.
    pub fn pointer_hover_hits(&self) -> Vec<Hit> {
        self.regions
            .iter()
            .enumerate()
            .flat_map(|(i, r)| hover_hits_for(id_of(i), r))
            .collect()
    }

    pub fn target_for_hit(&self, id: HitId) -> Option<FoldTarget> {
        let i = usize::try_from(id.checked_sub(1)?).ok()?;
        self.regions.get(i).map(|r| r.target.clone())
    }
}

fn id_of(index: usize) -> HitId {
    (index as HitId).saturating_add(1)
}

fn region_contains(r: &FoldHitRegion, content_row: usize, col: usize) -> bool {
    let height = r.height.max(1);
    content_row >= r.content_row
        && content_row < r.content_row.saturating_add(height)
        && col >= r.col_start
        && col < r.col_end
}

fn hover_hits_for(id: HitId, r: &FoldHitRegion) -> Vec<Hit> {
    match &r.target {
        FoldTarget::Tool(_) | FoldTarget::Diff(_) | FoldTarget::Ask(_) => {
            let mut out = vec![Hit::on_line(id, r.content_row).cols(r.col_start, r.col_end)];
            if r.height > 1 {
                out.push(
                    Hit::on_line(id, r.content_row)
                        .cols(0, RAILED_HOVER_COLS)
                        .tall(r.height),
                );
            }
            out
        }
        FoldTarget::Thinking(_) => {
            vec![Hit::on_line(id, r.content_row).cols(r.col_start, r.col_end)]
        }
        FoldTarget::OutputViewport(_)
        | FoldTarget::Compaction
        | FoldTarget::Segment(_)
        | FoldTarget::Cluster(_)
        | FoldTarget::LiveTail => vec![
            Hit::on_line(id, r.content_row)
                .cols(r.col_start, r.col_end)
                .tall(r.height),
        ],
        FoldTarget::TodoDoing | FoldTarget::TodoPending | FoldTarget::TodoPast => Vec::new(),
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
        assert_eq!(table.hit(3, 2), None, "outside registered columns");
        assert_eq!(table.hit(2, 5), None, "below transcript pane");
    }

    #[test]
    fn later_region_wins_and_height_covers_body() {
        let mut table = FoldHitTable {
            transcript_rows: 8,
            ..FoldHitTable::default()
        };
        table.push_rect(0, 0, 40, 4, FoldTarget::Tool("t".into()));
        table.push(3, 2, 20, FoldTarget::OutputViewport("t".into()));
        assert_eq!(
            table.hit(5, 3),
            Some(FoldTarget::OutputViewport("t".into())),
            "hint band after body"
        );
        assert_eq!(table.hit(5, 1), Some(FoldTarget::Tool("t".into())));
    }
}

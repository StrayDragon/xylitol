use crate::utils::ansi_escape_len;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use unicode_width::UnicodeWidthChar;

pub type HitId = u64;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    strum::EnumString,
    strum::Display,
    strum::IntoStaticStr,
)]
#[strum(serialize_all = "PascalCase")]
pub enum HitSpace {
    #[default]
    Content,
    Viewport,
    Chrome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    region: HitRegion,
}

impl Hit {
    #[must_use]
    pub fn on_line(id: HitId, line: usize) -> Self {
        Self {
            region: HitRegion::new(id, line, 0, 1),
        }
    }

    #[must_use]
    pub fn at_cell(id: HitId, col: u16, row: u16) -> Self {
        Self {
            region: HitRegion {
                id,
                space: HitSpace::Viewport,
                content_row: row as usize,
                col_start: col as usize,
                col_end: (col as usize).saturating_add(1),
                height: 1,
                capturing: false,
            },
        }
    }

    #[must_use]
    pub fn cols(mut self, start: usize, end: usize) -> Self {
        self.region.col_start = start;
        self.region.col_end = end;
        self
    }

    #[must_use]
    pub fn tall(self, height: usize) -> Self {
        Self {
            region: self.region.with_height(height),
        }
    }

    #[must_use]
    pub fn capturing(self) -> Self {
        Self {
            region: self.region.capturing(),
        }
    }

    #[must_use]
    pub fn space(mut self, space: HitSpace) -> Self {
        self.region.space = space;
        self
    }

    #[must_use]
    pub fn into_region(self) -> HitRegion {
        self.region
    }
}

impl From<Hit> for HitRegion {
    fn from(hit: Hit) -> Self {
        hit.region
    }
}

impl HitSpace {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitRegion {
    pub id: HitId,
    pub space: HitSpace,
    pub content_row: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub height: usize,
    pub capturing: bool,
}

impl HitRegion {
    #[must_use]
    pub fn new(id: HitId, content_row: usize, col_start: usize, col_end: usize) -> Self {
        Self {
            id,
            space: HitSpace::Content,
            content_row,
            col_start,
            col_end,
            height: 1,
            capturing: false,
        }
    }

    #[must_use]
    pub fn with_height(mut self, height: usize) -> Self {
        self.height = height.max(1);
        self
    }

    #[must_use]
    pub fn capturing(mut self) -> Self {
        self.capturing = true;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HitRegionList {
    regions: Vec<HitRegion>,
}

impl HitRegionList {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    pub fn clear(&mut self) {
        self.regions.clear();
    }

    pub fn push(&mut self, region: HitRegion) {
        if region.col_end > region.col_start {
            self.regions.push(region);
        }
    }

    pub fn mark(&mut self, hit: Hit) {
        self.push(hit.into_region());
    }

    pub fn iter(&self) -> impl Iterator<Item = HitRegion> + '_ {
        self.regions.iter().copied()
    }

    #[must_use]
    pub fn region_by_id(&self, id: HitId) -> Option<HitRegion> {
        self.regions.iter().copied().find(|r| r.id == id)
    }

    #[must_use]
    pub fn hit_at(
        &self,
        screen_col: u16,
        screen_row: u16,
        scroll_top: usize,
        transcript_rows: u16,
    ) -> Option<HitId> {
        self.region_at(screen_col, screen_row, scroll_top, transcript_rows)
            .map(|r| r.id)
    }

    #[must_use]
    pub fn capturing_at(
        &self,
        screen_col: u16,
        screen_row: u16,
        scroll_top: usize,
        transcript_rows: u16,
    ) -> Option<HitId> {
        self.region_at(screen_col, screen_row, scroll_top, transcript_rows)
            .filter(|r| r.capturing)
            .map(|r| r.id)
    }

    fn region_at(
        &self,
        screen_col: u16,
        screen_row: u16,
        scroll_top: usize,
        transcript_rows: u16,
    ) -> Option<&HitRegion> {
        if transcript_rows == 0 || screen_row >= transcript_rows {
            return None;
        }
        let col = screen_col as usize;
        let screen_row_n = screen_row as usize;
        let content_row = scroll_top.saturating_add(screen_row_n);
        self.regions.iter().rev().find(|r| {
            let height = r.height.max(1);
            let row_hit = match r.space {
                HitSpace::Content => {
                    content_row >= r.content_row
                        && content_row < r.content_row.saturating_add(height)
                }
                HitSpace::Viewport | HitSpace::Chrome => {
                    screen_row_n >= r.content_row
                        && screen_row_n < r.content_row.saturating_add(height)
                }
            };
            row_hit && col >= r.col_start && col < r.col_end
        })
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    strum::EnumString,
    strum::Display,
    strum::IntoStaticStr,
)]
#[strum(serialize_all = "PascalCase")]
pub enum PointerClickPhase {
    #[default]
    DownSwallow,
    UpIfEmptySelection,
}

impl PointerClickPhase {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PointerView {
    pub scroll_top: usize,
    pub transcript_rows: u16,
    pub overlay_capturing: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PointerOutcome {
    pub skip_selection: bool,
    pub consume: bool,
    pub hover_changed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PointerPipeline {
    regions: HitRegionList,
    hover_paint: HitRegionList,
    phase: PointerClickPhase,
    probe_armed: bool,
    hover_id: Option<HitId>,
    last_cell: Option<(u16, u16)>,
    pending_click: Option<HitId>,
    capture_latch: bool,
    down_cell: Option<(u16, u16)>,
    dragged: bool,
}

impl PointerPipeline {
    #[must_use]
    pub fn regions(&self) -> &HitRegionList {
        &self.regions
    }

    pub fn set_regions(&mut self, regions: HitRegionList) {
        self.regions = regions;
    }

    pub fn regions_mut(&mut self) -> &mut HitRegionList {
        &mut self.regions
    }

    #[must_use]
    pub fn hover_paint(&self) -> &HitRegionList {
        if self.hover_paint.is_empty() {
            &self.regions
        } else {
            &self.hover_paint
        }
    }

    pub fn set_hover_paint(&mut self, regions: HitRegionList) {
        self.hover_paint = regions;
    }

    #[must_use]
    pub fn phase(&self) -> PointerClickPhase {
        self.phase
    }

    pub fn set_phase(&mut self, phase: PointerClickPhase) {
        self.phase = phase;
    }

    #[must_use]
    pub fn probe_armed(&self) -> bool {
        self.probe_armed
    }

    pub fn arm_probe(&mut self, armed: bool) {
        self.probe_armed = armed;
        if !armed {
            self.hover_id = None;
        }
    }

    pub fn clear_hover(&mut self) {
        self.hover_id = None;
    }

    pub fn forget_hover(&mut self) {
        self.hover_id = None;
        self.last_cell = None;
    }

    #[must_use]
    pub fn hover_id(&self) -> Option<HitId> {
        self.hover_id
    }

    pub fn reprobe_hover(&mut self, view: PointerView) -> bool {
        if !self.probe_armed {
            return false;
        }
        let hit = self
            .last_cell
            .and_then(|(col, row)| self.hit_at(col, row, view));
        let changed = hit != self.hover_id;
        self.hover_id = hit;
        changed
    }

    pub fn take_click(&mut self) -> Option<HitId> {
        self.pending_click.take()
    }

    #[must_use]
    pub fn peek_click(&self) -> Option<HitId> {
        self.pending_click
    }

    #[must_use]
    pub fn capture_latched(&self) -> bool {
        self.capture_latch
    }

    #[must_use]
    pub fn hit_at(&self, col: u16, row: u16, view: PointerView) -> Option<HitId> {
        self.regions
            .hit_at(col, row, view.scroll_top, view.transcript_rows)
    }

    fn latch_chrome_click(
        &mut self,
        hit: Option<HitId>,
        down: Option<(u16, u16)>,
        col: u16,
        row: u16,
        overlay_capturing: bool,
    ) {
        if let Some(id) = self.chrome_click_id(hit, down, col, row, overlay_capturing) {
            self.pending_click = Some(id);
        }
    }

    fn chrome_click_id(
        &self,
        hit: Option<HitId>,
        down: Option<(u16, u16)>,
        col: u16,
        row: u16,
        overlay_capturing: bool,
    ) -> Option<HitId> {
        if overlay_capturing || down != Some((col, row)) || self.dragged {
            return None;
        }
        let id = hit?;
        self.regions
            .region_by_id(id)
            .filter(|region| region.space == HitSpace::Chrome)
            .map(|region| region.id)
    }

    pub fn on_mouse(&mut self, event: &MouseEvent, view: PointerView) -> PointerOutcome {
        let (col, row) = (event.column, event.row);
        self.last_cell = Some((col, row));
        let hit = self.hit_at(col, row, view);
        let capturing_region = self
            .regions
            .capturing_at(col, row, view.scroll_top, view.transcript_rows)
            .is_some();

        match event.kind {
            MouseEventKind::Moved => {
                if !self.probe_armed {
                    return PointerOutcome::default();
                }
                let hover_changed = hit != self.hover_id;
                self.hover_id = hit;
                PointerOutcome {
                    hover_changed,
                    ..PointerOutcome::default()
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.down_cell = Some((col, row));
                self.dragged = false;
                if self.capture_latch {
                    return PointerOutcome {
                        skip_selection: true,
                        consume: !view.overlay_capturing,
                        ..PointerOutcome::default()
                    };
                }
                if view.overlay_capturing || capturing_region {
                    self.capture_latch = true;
                    return PointerOutcome {
                        skip_selection: true,
                        consume: capturing_region && !view.overlay_capturing,
                        ..PointerOutcome::default()
                    };
                }
                if self.phase == PointerClickPhase::DownSwallow
                    && let Some(id) = hit
                {
                    self.pending_click = Some(id);
                    return PointerOutcome {
                        skip_selection: true,
                        consume: true,
                        ..PointerOutcome::default()
                    };
                }
                PointerOutcome::default()
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(down) = self.down_cell
                    && down != (col, row)
                {
                    self.dragged = true;
                }
                if self.capture_latch {
                    return PointerOutcome {
                        skip_selection: true,
                        consume: !view.overlay_capturing,
                        ..PointerOutcome::default()
                    };
                }
                PointerOutcome::default()
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let was_capture = self.capture_latch;
                self.capture_latch = false;
                let down = self.down_cell.take();
                if let Some(cell) = down
                    && cell != (col, row)
                {
                    self.dragged = true;
                }
                if was_capture {
                    self.latch_chrome_click(hit, down, col, row, view.overlay_capturing);
                    return PointerOutcome {
                        skip_selection: true,
                        consume: !view.overlay_capturing,
                        ..PointerOutcome::default()
                    };
                }
                if self.phase == PointerClickPhase::UpIfEmptySelection
                    && !self.dragged
                    && let Some(id) = hit
                {
                    self.pending_click = Some(id);
                    return PointerOutcome {
                        skip_selection: true,
                        consume: true,
                        ..PointerOutcome::default()
                    };
                }
                PointerOutcome::default()
            }
            _ => PointerOutcome::default(),
        }
    }
}

pub fn apply_hover_tint(
    visible: &mut [String],
    scroll_top: usize,
    regions: &HitRegionList,
    hover_id: HitId,
) {
    for region in regions.iter().filter(|r| r.id == hover_id) {
        let height = region.height.max(1);
        for offset in 0..height {
            let vis_i = match region.space {
                HitSpace::Content => {
                    let content_row = region.content_row.saturating_add(offset);
                    if content_row < scroll_top {
                        continue;
                    }
                    content_row - scroll_top
                }
                HitSpace::Viewport | HitSpace::Chrome => region.content_row.saturating_add(offset),
            };
            if let Some(line) = visible.get_mut(vis_i) {
                *line = emphasize_hover_columns(line, region.col_start, region.col_end);
            }
        }
    }
}

fn emphasize_hover_columns(line: &str, from: usize, to: usize) -> String {
    if to <= from || line.is_empty() {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len() + 16);
    let bytes = line.as_bytes();
    let mut col = 0usize;
    let mut started = false;
    let mut finished = false;
    let mut i = 0usize;
    while i < line.len() {
        if let Some(len) = ansi_escape_len(bytes, i) {
            let seq = &line[i..i + len];
            if started && !finished && is_sgr(seq) {
                out.push_str(&emphasize_sgr_in_hover(seq));
            } else {
                out.push_str(seq);
            }
            i += len;
            continue;
        }
        let c = line[i..].chars().next().expect("char boundary");
        let cw = c.width().unwrap_or(0);
        if started && !finished && col >= to {
            out.push_str("\x1b[22m");
            finished = true;
        }
        if !started && col < to && col + cw.max(1) > from {
            out.push_str("\x1b[22m\x1b[1m");
            started = true;
        }
        out.push(c);
        col += cw;
        i += c.len_utf8();
    }
    if started && !finished {
        out.push_str("\x1b[22m");
    }
    out
}

fn is_sgr(seq: &str) -> bool {
    seq.as_bytes().first() == Some(&0x1b)
        && seq.as_bytes().get(1) == Some(&b'[')
        && seq.as_bytes().last() == Some(&b'm')
}

fn emphasize_sgr_in_hover(seq: &str) -> String {
    if seq == "\x1b[2m" {
        return "\x1b[1m".to_string();
    }
    if seq.ends_with(";2m") {
        let mut s = seq.to_string();
        s.truncate(s.len() - 3);
        return format!("{s};22;1m");
    }
    let mut s = seq.to_string();
    s.truncate(s.len() - 1);
    s.push_str(";1m");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn view(scroll_top: usize) -> PointerView {
        PointerView {
            scroll_top,
            transcript_rows: 8,
            overlay_capturing: false,
        }
    }

    #[test]
    fn empty_list_hits_nothing() {
        let list = HitRegionList::new();
        assert!(list.hit_at(0, 0, 0, 8).is_none());
    }

    #[test]
    fn hit_follows_live_scroll_not_registration_snapshot() {
        let mut list = HitRegionList::new();
        list.push(HitRegion::new(7, 5, 0, 4));
        assert_eq!(
            list.hit_at(1, 5, 0, 8),
            Some(7),
            "scroll 0: screen 5 → content 5"
        );
        assert_eq!(
            list.hit_at(1, 5, 3, 8),
            None,
            "scroll 3: same screen row is content 8, not frozen at 5"
        );
        assert_eq!(
            list.hit_at(1, 2, 3, 8),
            Some(7),
            "scroll 3: screen 2 → content 5"
        );
    }

    #[test]
    fn later_region_wins_z_order() {
        let mut list = HitRegionList::new();
        list.push(HitRegion::new(1, 0, 0, 8));
        list.push(HitRegion::new(2, 0, 0, 4));
        assert_eq!(list.hit_at(1, 0, 0, 8), Some(2));
        assert_eq!(list.hit_at(5, 0, 0, 8), Some(1));
    }

    #[test]
    fn click_phase_strum_roundtrip() {
        assert_eq!(PointerClickPhase::DownSwallow.as_str(), "DownSwallow");
        assert_eq!(
            "UpIfEmptySelection".parse::<PointerClickPhase>().unwrap(),
            PointerClickPhase::UpIfEmptySelection
        );
    }

    #[test]
    fn down_swallow_reports_on_down_not_up() {
        let mut pipe = PointerPipeline::default();
        pipe.regions_mut().push(HitRegion::new(3, 0, 0, 5));
        let down = pipe.on_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 1, 0),
            view(0),
        );
        assert!(down.skip_selection && down.consume);
        assert_eq!(pipe.take_click(), Some(3));
        let up = pipe.on_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 1, 0), view(0));
        assert!(!up.skip_selection || pipe.peek_click().is_none());
        assert!(pipe.take_click().is_none());
    }

    #[test]
    fn up_if_empty_reports_on_up_without_drag() {
        let mut pipe = PointerPipeline::default();
        pipe.set_phase(PointerClickPhase::UpIfEmptySelection);
        pipe.regions_mut().push(HitRegion::new(9, 0, 0, 5));
        let down = pipe.on_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 1, 0),
            view(0),
        );
        assert!(!down.skip_selection);
        assert!(pipe.take_click().is_none());
        let up = pipe.on_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 1, 0), view(0));
        assert!(up.skip_selection);
        assert_eq!(pipe.take_click(), Some(9));
    }

    #[test]
    fn drag_never_reports_click() {
        let mut pipe = PointerPipeline::default();
        pipe.set_phase(PointerClickPhase::UpIfEmptySelection);
        pipe.regions_mut().push(HitRegion::new(1, 0, 0, 8));
        let _ = pipe.on_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            view(0),
        );
        let _ = pipe.on_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 4, 0),
            view(0),
        );
        let _ = pipe.on_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 4, 0), view(0));
        assert!(pipe.take_click().is_none());
    }

    #[test]
    fn capture_latches_until_up() {
        let mut pipe = PointerPipeline::default();
        pipe.regions_mut()
            .push(HitRegion::new(4, 0, 0, 6).capturing());
        let down = pipe.on_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 1, 0),
            view(0),
        );
        assert!(down.skip_selection);
        assert!(pipe.capture_latched());
        let drag = pipe.on_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 5, 1),
            view(0),
        );
        assert!(drag.skip_selection);
        let up = pipe.on_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 5, 1), view(0));
        assert!(up.skip_selection);
        assert!(!pipe.capture_latched());
    }

    #[test]
    fn chrome_capture_reports_click_on_up() {
        let mut pipe = PointerPipeline::default();
        pipe.regions_mut().push(
            Hit::at_cell(7, 1, 0)
                .cols(0, 6)
                .capturing()
                .space(HitSpace::Chrome)
                .into_region(),
        );
        let down = pipe.on_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 1, 0),
            view(0),
        );
        assert!(down.skip_selection);
        assert!(pipe.take_click().is_none(), "chrome click lands on up");
        let up = pipe.on_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 1, 0), view(0));
        assert!(up.skip_selection);
        assert_eq!(pipe.take_click(), Some(7));
    }

    #[test]
    fn chrome_capture_drag_does_not_report_click() {
        let mut pipe = PointerPipeline::default();
        pipe.regions_mut().push(
            Hit::at_cell(7, 1, 0)
                .cols(0, 6)
                .capturing()
                .space(HitSpace::Chrome)
                .into_region(),
        );
        let _ = pipe.on_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 1, 0),
            view(0),
        );
        let _ = pipe.on_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 3, 0),
            view(0),
        );
        let _ = pipe.on_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 3, 0), view(0));
        assert!(pipe.take_click().is_none());
    }

    #[test]
    fn component_overlay_capture_does_not_report_click() {
        let mut pipe = PointerPipeline::default();
        pipe.regions_mut()
            .push(HitRegion::new(4, 0, 0, 6).capturing());
        let _ = pipe.on_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 1, 0),
            view(0),
        );
        let _ = pipe.on_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 1, 0), view(0));
        assert!(
            pipe.take_click().is_none(),
            "component overlays keep handling their own clicks"
        );
    }

    #[test]
    fn unarmed_moved_does_not_change_hover() {
        let mut pipe = PointerPipeline::default();
        pipe.regions_mut().push(HitRegion::new(1, 0, 0, 4));
        let out = pipe.on_mouse(&mouse(MouseEventKind::Moved, 1, 0), view(0));
        assert!(!out.hover_changed);
        assert!(pipe.hover_id().is_none());
    }

    #[test]
    fn armed_same_id_moved_is_not_hover_changed() {
        let mut pipe = PointerPipeline::default();
        pipe.arm_probe(true);
        pipe.regions_mut().push(HitRegion::new(1, 0, 0, 4));
        let first = pipe.on_mouse(&mouse(MouseEventKind::Moved, 1, 0), view(0));
        assert!(first.hover_changed);
        let second = pipe.on_mouse(&mouse(MouseEventKind::Moved, 2, 0), view(0));
        assert!(!second.hover_changed);
        assert_eq!(pipe.hover_id(), Some(1));
    }

    #[test]
    fn reprobe_uses_last_cell_against_live_scroll() {
        let mut pipe = PointerPipeline::default();
        pipe.arm_probe(true);
        pipe.regions_mut().push(HitRegion::new(1, 5, 0, 4));
        let _ = pipe.on_mouse(&mouse(MouseEventKind::Moved, 1, 5), view(0));
        assert_eq!(pipe.hover_id(), Some(1));
        assert!(pipe.reprobe_hover(view(3)));
        assert!(
            pipe.hover_id().is_none(),
            "sticky id must not follow scrolled content away from the pointer cell"
        );
    }

    #[test]
    fn clear_hover_drops_id_but_reprobe_can_restore() {
        let mut pipe = PointerPipeline::default();
        pipe.arm_probe(true);
        pipe.regions_mut().push(HitRegion::new(1, 0, 0, 4));
        let _ = pipe.on_mouse(&mouse(MouseEventKind::Moved, 1, 0), view(0));
        pipe.clear_hover();
        assert!(pipe.hover_id().is_none());
        assert!(pipe.reprobe_hover(view(0)));
        assert_eq!(pipe.hover_id(), Some(1));
    }

    #[test]
    fn hover_tint_uses_bold_not_invert() {
        let mut list = HitRegionList::new();
        list.push(HitRegion::new(1, 0, 0, 5));
        let mut visible = vec!["hello".into()];
        apply_hover_tint(&mut visible, 0, &list, 1);
        assert!(
            visible[0].contains("\x1b[1m"),
            "hover must emphasize (bold), got {:?}",
            visible[0]
        );
        assert!(
            !visible[0].contains("\x1b[7m"),
            "hover must not reuse selection invert"
        );
    }

    #[test]
    fn hover_tint_replaces_dim_with_bold() {
        let mut list = HitRegionList::new();
        list.push(HitRegion::new(1, 0, 0, 8));
        let mut visible = vec!["\x1b[2mdim title\x1b[22m".into()];
        apply_hover_tint(&mut visible, 0, &list, 1);
        assert!(
            visible[0].contains("\x1b[22m\x1b[1m"),
            "hover over dim text must reset dim then bold, got {:?}",
            visible[0]
        );
    }

    #[test]
    fn hit_space_strum_roundtrip() {
        assert_eq!(HitSpace::Content.as_str(), "Content");
        assert_eq!("Viewport".parse::<HitSpace>().unwrap(), HitSpace::Viewport);
    }

    #[test]
    fn host_on_line_follows_live_scroll() {
        let mut list = HitRegionList::new();
        list.mark(Hit::on_line(7, 5).cols(0, 4));
        assert_eq!(list.hit_at(1, 5, 0, 8), Some(7));
        assert_eq!(list.hit_at(1, 5, 3, 8), None);
        assert_eq!(list.hit_at(1, 2, 3, 8), Some(7));
    }

    #[test]
    fn host_at_cell_stays_on_screen_when_scrolled() {
        let mut list = HitRegionList::new();
        list.mark(Hit::at_cell(11, 1, 0));
        assert_eq!(
            list.hit_at(1, 0, 0, 8),
            Some(11),
            "viewport hit at screen (1,0)"
        );
        assert_eq!(
            list.hit_at(1, 0, 12, 8),
            Some(11),
            "same screen cell must keep the id after scroll"
        );
        assert_eq!(
            list.hit_at(1, 2, 12, 8),
            None,
            "a different screen row must miss"
        );
    }
}

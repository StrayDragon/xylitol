use crate::tui::{Component, InputEvent};
use crate::utils::visible_width;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionChip {
    pub label: String,
    pub active: bool,
    pub disabled: bool,
}

impl ActionChip {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            active: false,
            disabled: false,
        }
    }

    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct ActionRowTheme {
    pub idle: Box<dyn Fn(&str) -> String>,
    pub active: Box<dyn Fn(&str) -> String>,
}

impl Default for ActionRowTheme {
    fn default() -> Self {
        Self {
            idle: Box::new(|s| s.to_string()),
            active: Box::new(|s| format!("\u{1b}[7m{s}\u{1b}[27m")),
        }
    }
}

pub struct ActionRow {
    chips: Vec<ActionChip>,
    theme: ActionRowTheme,
    ranges: Vec<(usize, usize, usize)>,
    last_width: usize,
}

impl ActionRow {
    pub fn new(chips: Vec<ActionChip>) -> Self {
        Self {
            chips,
            theme: ActionRowTheme::default(),
            ranges: Vec::new(),
            last_width: 0,
        }
    }

    #[must_use]
    pub fn with_theme(mut self, theme: ActionRowTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn chips(&self) -> &[ActionChip] {
        &self.chips
    }

    pub fn set_chips(&mut self, chips: Vec<ActionChip>) {
        self.chips = chips;
        self.invalidate();
    }

    pub fn chip_col_ranges(&self) -> Vec<(usize, usize, usize)> {
        self.ranges.clone()
    }

    pub fn chip_at(&self, col: usize) -> Option<usize> {
        self.ranges
            .iter()
            .find(|(start, end, _)| col >= *start && col < *end)
            .map(|(_, _, index)| *index)
    }

    pub fn last_width(&self) -> usize {
        self.last_width
    }

    fn glyph(chip: &ActionChip) -> String {
        format!("[{}]", chip.label)
    }

    fn layout(&self, width: usize) -> Vec<(usize, usize, usize)> {
        let mut ranges = Vec::new();
        let mut col = 0usize;
        let mut painted_any = false;
        for (index, chip) in self.chips.iter().enumerate() {
            if chip.disabled {
                continue;
            }
            let w = visible_width(&Self::glyph(chip));
            let gap = usize::from(painted_any);
            if col + gap + w > width {
                break;
            }
            col += gap;
            ranges.push((col, col + w, index));
            col += w;
            painted_any = true;
        }
        ranges
    }
}

impl Component for ActionRow {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.last_width = width;
        self.ranges = self.layout(width);
        let mut out = String::new();
        let mut prev_end: Option<usize> = None;
        for (start, end, index) in self.chip_col_ranges() {
            if let Some(prev) = prev_end {
                out.push_str(&" ".repeat(start - prev));
            }
            let chip = &self.chips[index];
            let glyph = Self::glyph(chip);
            out.push_str(&if chip.active {
                (self.theme.active)(&glyph)
            } else {
                (self.theme.idle)(&glyph)
            });
            prev_end = Some(end);
        }
        vec![out]
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn invalidate(&mut self) {
        self.ranges.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::strip_ansi_codes;

    fn chips() -> Vec<ActionChip> {
        vec![
            ActionChip::new("详情"),
            ActionChip::new("侧栏").with_active(true),
            ActionChip::new("重置折叠"),
        ]
    }

    fn visible(lines: &[String]) -> String {
        strip_ansi_codes(&lines.concat())
    }

    #[test]
    fn ranges_stay_inside_width_and_never_overlap() {
        let mut row = ActionRow::new(chips());
        let lines = row.render(40);
        assert_eq!(lines.len(), 1, "one line only");
        let ranges = row.chip_col_ranges();
        assert_eq!(ranges.len(), 3);
        for (i, (start, end, index)) in ranges.iter().enumerate() {
            assert_eq!(*index, i);
            assert!(start < end, "empty range {start}..{end}");
            assert!(*end <= 40, "range {start}..{end} exceeds width");
            if let Some((_, prev_end, _)) = i.checked_sub(1).map(|j| ranges[j]) {
                assert!(*start >= prev_end, "range {i} overlaps the previous one");
            }
        }
        assert_eq!(visible(&lines).matches('[').count(), 3);
    }

    #[test]
    fn active_chip_is_visually_distinct() {
        let mut row = ActionRow::new(chips());
        let lines = row.render(40);
        let painted = (ActionRowTheme::default().active)("[侧栏]");
        assert!(
            lines[0].contains(&painted),
            "active chip must invert; got {:?}",
            lines[0]
        );
        let idle = (ActionRowTheme::default().active)("[详情]");
        assert!(!lines[0].contains(&idle), "idle chip must not invert");
    }

    #[test]
    fn disabled_chip_is_neither_drawn_nor_mapped() {
        let mut row = ActionRow::new(vec![
            ActionChip::new("详情"),
            ActionChip::new("侧栏").with_disabled(true),
            ActionChip::new("重置折叠"),
        ]);
        let lines = row.render(40);
        let ranges = row.chip_col_ranges();
        assert_eq!(ranges.len(), 2);
        assert!(
            ranges.iter().all(|(_, _, index)| *index != 1),
            "disabled chip must not appear in ranges: {ranges:?}"
        );
        let text = visible(&lines);
        assert!(!text.contains("侧栏"), "disabled text leaked: {text:?}");
        assert!(text.contains("详情") && text.contains("重置折叠"));
    }

    #[test]
    fn narrow_width_drops_whole_chips_from_the_tail() {
        let mut row = ActionRow::new(chips());
        let lines = row.render(12);
        assert_eq!(lines.len(), 1, "row must stay a single line");
        let ranges = row.chip_col_ranges();
        assert_eq!(ranges.len(), 1, "only the first chip fits in 12 cols");
        let text = visible(&lines);
        assert!(text.contains("详情"));
        assert!(!text.contains("侧栏"), "dropped chip text: {text:?}");
        assert!(!text.contains("重置折叠"), "dropped chip text: {text:?}");
    }

    #[test]
    fn zero_width_renders_one_empty_line() {
        let mut row = ActionRow::new(chips());
        let lines = row.render(0);
        assert_eq!(lines, vec![String::new()]);
        assert!(row.chip_col_ranges().is_empty());
    }

    #[test]
    fn chip_at_maps_columns_back_to_indices() {
        let mut row = ActionRow::new(chips());
        let _ = row.render(40);
        let (start, end, _) = row.chip_col_ranges()[1];
        assert_eq!(row.chip_at(start), Some(1));
        assert_eq!(row.chip_at(end - 1), Some(1));
        assert_eq!(row.chip_at(end), None);
    }
}

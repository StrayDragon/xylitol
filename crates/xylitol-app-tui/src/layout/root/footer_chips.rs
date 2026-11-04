use super::UiRoot;

fn fold_has_overrides(fold: &crate::widgets::ScrollbackFold) -> bool {
    [
        &fold.tools_overrides,
        &fold.thinking_overrides,
        &fold.output_overrides,
    ]
    .iter()
    .any(|m| !m.is_empty())
}

fn fold_differs_from_defaults(fold: &crate::widgets::ScrollbackFold) -> bool {
    let defaults = crate::widgets::ScrollbackFold::default();
    [
        fold.thinking_expanded != defaults.thinking_expanded,
        fold.tools_expanded != defaults.tools_expanded,
        fold.tools_output_expanded != defaults.tools_output_expanded,
        fold.compaction_expanded != defaults.compaction_expanded,
    ]
    .iter()
    .any(|d| *d)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterAction {
    Sidebar,
    ResetFold,
    FoldSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FooterChipHit {
    start: usize,
    end: usize,
    pub action: FooterAction,
}

impl UiRoot {
    fn top_chip_actions(&self) -> Vec<(FooterAction, &'static str, bool)> {
        let mut chips = Vec::new();
        if self.session_sidebar_available() {
            chips.push((FooterAction::Sidebar, "侧栏", self.session_sidebar_on));
        }
        chips
    }

    fn bottom_chip_actions(&self) -> Vec<(FooterAction, &'static str, bool)> {
        let mut chips = Vec::new();
        if self.reset_chip_enabled() {
            chips.push((FooterAction::ResetFold, "重置折叠", false));
        }
        chips.push((FooterAction::FoldSummary, "▤ 摘要", false));
        chips
    }

    pub(crate) fn footer_chip_actions(&self) -> Vec<(FooterAction, &'static str, bool)> {
        let mut chips = self.top_chip_actions();
        chips.extend(self.bottom_chip_actions());
        chips
    }

    fn paint_dock_chip(&self, border: &dyn Fn(&str) -> String, text: &str, active: bool) -> String {
        if active {
            return self.theme.paint_chip(text, true);
        }
        border(text)
    }

    fn reset_chip_enabled(&self) -> bool {
        let fold = self.fold();
        fold_has_overrides(&fold) || fold_differs_from_defaults(&fold)
    }

    pub(super) fn append_border_chips(
        &self,
        line: &str,
        width: usize,
    ) -> (String, Vec<FooterChipHit>) {
        if width == 0 {
            return (line.to_string(), Vec::new());
        }
        let chips = self.top_chip_actions();
        let stage_name = self
            .ui_model
            .stage_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let mut title = stage_name.map(|name| format!(" {name} "));
        let mut labels = Vec::with_capacity(chips.len());
        for (action, label, active) in &chips {
            labels.push((*action, format!("[{label}]"), *active));
        }
        let chip_w: usize = labels
            .iter()
            .map(|(_, text, _)| xylitol_tui::visible_width(text))
            .sum::<usize>()
            + labels.len().saturating_sub(1);
        if labels.is_empty() || chip_w + 6 > width {
            return (line.to_string(), Vec::new());
        }
        let title_w = title
            .as_deref()
            .map(xylitol_tui::visible_width)
            .unwrap_or(0);
        if chip_w + title_w + usize::from(title.is_some()) + 6 > width {
            title = None;
        }
        let title_w = title
            .as_deref()
            .map(xylitol_tui::visible_width)
            .unwrap_or(0);
        let title_gap = usize::from(title.is_some());
        let total_w = chip_w + title_w + title_gap;
        let dash_w = width.saturating_sub(total_w + 3);
        let border = self.operation_zone_paint();
        let mut out = if dash_w == 0 {
            String::new()
        } else {
            border(&"─".repeat(dash_w))
        };
        out.insert(0, '╭');
        out.push(' ');
        let mut col = dash_w + 1;
        let mut hits = Vec::with_capacity(labels.len());
        for (i, (action, text, active)) in labels.iter().enumerate() {
            if i > 0 {
                out.push(' ');
                col += 1;
            }
            let w = xylitol_tui::visible_width(text);
            out.push_str(&self.paint_dock_chip(&*border, text, *active));
            hits.push(FooterChipHit {
                start: col,
                end: col + w,
                action: *action,
            });
            col += w;
        }
        if let Some(title) = title {
            out.push(' ');
            out.push_str(&self.theme.paint_muted(&title));
        }
        out.push('╮');
        (out, hits)
    }

    pub(super) fn paint_bottom_chrome(&self, width: usize) -> (String, Vec<FooterChipHit>) {
        if width == 0 {
            return (String::new(), Vec::new());
        }
        let chips = self.bottom_chip_actions();
        let mut labels = Vec::with_capacity(chips.len());
        for (action, label, active) in &chips {
            labels.push((*action, format!("[{label}]"), *active));
        }
        let chip_w: usize = labels
            .iter()
            .map(|(_, text, _)| xylitol_tui::visible_width(text))
            .sum::<usize>()
            + labels.len().saturating_sub(1);
        if labels.is_empty() || chip_w + 4 > width {
            return (String::new(), Vec::new());
        }
        let dash_w = width.saturating_sub(chip_w + 3);
        let border = self.operation_zone_paint();
        let mut out = String::from("╰");
        if dash_w > 0 {
            out.push_str(&border(&"─".repeat(dash_w)));
        }
        out.push(' ');
        let mut col = dash_w + 2;
        let mut hits = Vec::with_capacity(labels.len());
        for (i, (action, text, active)) in labels.iter().enumerate() {
            if i > 0 {
                out.push(' ');
                col += 1;
            }
            let w = xylitol_tui::visible_width(text);
            out.push_str(&self.paint_dock_chip(&*border, text, *active));
            hits.push(FooterChipHit {
                start: col,
                end: col + w,
                action: *action,
            });
            col += w;
        }
        out.push('╯');
        (out, hits)
    }

    pub(crate) fn try_click_painted_chip(&mut self, col: usize, line: &str) -> bool {
        let plain = xylitol_tui::utils::strip_ansi_codes(line);
        let width = xylitol_tui::visible_width(&plain);
        if plain.contains('◨') && col >= width.saturating_sub(2) {
            return true;
        }
        let action = self
            .chip_hits_for_painted(line)
            .into_iter()
            .find(|h| col >= h.start && col < h.end)
            .map(|h| h.action);
        match action {
            Some(action) => self.click_footer_action(action),
            None => false,
        }
    }

    pub(super) fn chip_hits_for_painted(&self, line: &str) -> Vec<FooterChipHit> {
        let plain = xylitol_tui::utils::strip_ansi_codes(line);
        let mut hits = Vec::new();
        let mut cursor = 0usize;
        for (action, label, _) in self.footer_chip_actions() {
            let chip = format!("[{label}]");
            let Some(pos) = plain[cursor..].find(chip.as_str()) else {
                continue;
            };
            let byte_start = cursor + pos;
            let start = xylitol_tui::visible_width(&plain[..byte_start]);
            hits.push(FooterChipHit {
                start,
                end: start + xylitol_tui::visible_width(&chip),
                action,
            });
            cursor = byte_start + chip.len();
        }
        hits
    }

    pub(crate) fn click_footer_action(&mut self, action: FooterAction) -> bool {
        match action {
            FooterAction::Sidebar => {
                if self.session_sidebar_on {
                    self.close_session_sidebar();
                } else {
                    self.request_session_sidebar();
                }
            }
            FooterAction::FoldSummary => {
                self.cycle_display_level();
            }
            FooterAction::ResetFold => {
                if !self.reset_chip_enabled() {
                    return false;
                }
                self.reset_tools_blocks();
            }
        }
        true
    }

    #[cfg(test)]
    pub fn footer_chip_actions_for_test(&self) -> Vec<(FooterAction, &'static str, bool)> {
        self.footer_chip_actions()
    }

    #[cfg(test)]
    pub fn footer_chip_columns_for_test(&self) -> Vec<(usize, usize, FooterAction)> {
        self.last_footer_hits
            .iter()
            .map(|h| (h.start, h.end, h.action))
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::ScrollbackFold;

    fn chip_row_for(level: &str) -> String {
        let mut root = super::super::UiRoot::new();
        root.set_term_cols(120);
        root.set_thinking_level_ui(level.to_string());
        root.append_border_chips("", 120).0
    }

    const SGR: &str = "\x1b[38;2;";

    fn rgb_after(line: &str, needle: &str) -> (u8, u8, u8) {
        let at = line.find(needle).expect("needle");
        let prefix = &line[..at];
        let start = prefix.rfind(SGR).expect("truecolor SGR");
        let rest = &prefix[start + SGR.len()..];
        let end = rest.find('m').expect("SGR terminator");
        let mut parts = rest[..end].split(';');
        (
            parts.next().unwrap().parse().unwrap(),
            parts.next().unwrap().parse().unwrap(),
            parts.next().unwrap().parse().unwrap(),
        )
    }

    fn first_sgr(line: &str) -> (u8, u8, u8) {
        let start = line.find(SGR).expect("truecolor SGR");
        let rest = &line[start + SGR.len()..];
        let end = rest.find('m').expect("SGR terminator");
        let mut parts = rest[..end].split(';');
        (
            parts.next().unwrap().parse().unwrap(),
            parts.next().unwrap().parse().unwrap(),
            parts.next().unwrap().parse().unwrap(),
        )
    }

    #[test]
    fn chip_row_follows_the_thinking_border_level() {
        let palette = crate::layout::LayoutTheme::product_dark().palette();
        let off = first_sgr(&chip_row_for("off"));
        let high = first_sgr(&chip_row_for("high"));
        assert_ne!(
            off, high,
            "chip row must change colour with the thinking level: off={off:?} high={high:?}"
        );
        assert_ne!(
            off,
            (palette.muted.r, palette.muted.g, palette.muted.b),
            "off chip row must not be the plain muted token"
        );
    }

    #[test]
    fn inactive_chip_shares_the_border_colour() {
        let row = chip_row_for("high");
        let dashes = first_sgr(&row);
        let chip = rgb_after(&row, "[侧栏]");
        assert_eq!(
            chip, dashes,
            "inactive chip and border must share one colour"
        );
    }

    #[test]
    fn default_fold_has_no_overrides_and_matches_defaults() {
        let fold = ScrollbackFold::default();
        assert!(!fold_has_overrides(&fold));
        assert!(!fold_differs_from_defaults(&fold));
    }

    #[test]
    fn each_override_map_flags_overrides() {
        for apply in [
            |f: &mut ScrollbackFold| {
                f.tools_overrides.insert("t".into(), true);
            },
            |f: &mut ScrollbackFold| {
                f.thinking_overrides.insert("t".into(), true);
            },
            |f: &mut ScrollbackFold| {
                f.output_overrides.insert("t".into(), true);
            },
        ] {
            let mut fold = ScrollbackFold::default();
            apply(&mut fold);
            assert!(fold_has_overrides(&fold));
            assert!(
                !fold_differs_from_defaults(&fold),
                "override maps must not be confused with default flags"
            );
        }
    }

    #[test]
    fn each_default_flag_flip_marks_a_difference() {
        for apply in [
            |f: &mut ScrollbackFold| f.thinking_expanded = !f.thinking_expanded,
            |f: &mut ScrollbackFold| f.tools_expanded = !f.tools_expanded,
            |f: &mut ScrollbackFold| f.tools_output_expanded = !f.tools_output_expanded,
            |f: &mut ScrollbackFold| f.compaction_expanded = !f.compaction_expanded,
        ] {
            let mut fold = ScrollbackFold::default();
            apply(&mut fold);
            assert!(fold_differs_from_defaults(&fold));
            assert!(
                !fold_has_overrides(&fold),
                "default flags must not be confused with override maps"
            );
        }
    }
}

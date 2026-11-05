use super::UiRoot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterAction {
    Sidebar,
    FoldSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FooterChipHit {
    start: usize,
    end: usize,
    pub action: FooterAction,
}

impl UiRoot {
    fn bottom_chip_actions(&self) -> Vec<(FooterAction, &'static str, bool)> {
        vec![(FooterAction::FoldSummary, "▤ 摘要", false)]
    }

    pub(crate) fn footer_chip_actions(&self) -> Vec<(FooterAction, &'static str, bool)> {
        self.bottom_chip_actions()
    }

    fn paint_dock_chip(&self, border: &dyn Fn(&str) -> String, text: &str, active: bool) -> String {
        if active {
            return self.theme.paint_chip(text, true);
        }
        border(text)
    }

    pub(super) fn append_border_chips(
        &self,
        line: &str,
        width: usize,
    ) -> (String, Vec<FooterChipHit>) {
        if width == 0 {
            return (line.to_string(), Vec::new());
        }
        let stage_name = self
            .ui_model
            .stage_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let plain = xylitol_tui::utils::strip_ansi_codes(line);
        let Some(title) = stage_name.map(|name| format!(" {name} ")) else {
            if !plain.contains("more") {
                return (line.to_string(), Vec::new());
            }
            let interior =
                xylitol_tui::truncate_to_width(&plain, width.saturating_sub(2), "…", false);
            let pad = width
                .saturating_sub(2)
                .saturating_sub(xylitol_tui::visible_width(&interior));
            let border = self.operation_zone_paint();
            let mut out = border("╭");
            out.push_str(&border(&interior));
            out.push_str(&border(&"─".repeat(pad)));
            out.push_str(&border("╮"));
            return (out, Vec::new());
        };
        let title_w = xylitol_tui::visible_width(&title);
        if title_w + 4 > width {
            return (line.to_string(), Vec::new());
        }
        let plain = xylitol_tui::utils::strip_ansi_codes(line);
        let interior = if plain.contains("more") {
            plain
        } else {
            plain
                .trim_start_matches('╭')
                .trim_end_matches('╮')
                .to_string()
        };
        let avail = width.saturating_sub(2 + title_w);
        let interior = xylitol_tui::truncate_to_width(&interior, avail, "…", false);
        let pad = avail.saturating_sub(xylitol_tui::visible_width(&interior));
        let border = self.operation_zone_paint();
        let mut out = border("╭");
        out.push_str(&border(&interior));
        out.push_str(&border(&"─".repeat(pad)));
        out.push_str(&self.theme.paint_muted(&title));
        out.push_str(&border("╮"));
        (out, Vec::new())
    }

    pub(super) fn paint_bottom_chrome(
        &self,
        line: &str,
        width: usize,
    ) -> (String, Vec<FooterChipHit>) {
        if width == 0 {
            return (line.to_string(), Vec::new());
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
            return (line.to_string(), Vec::new());
        }
        let plain = xylitol_tui::utils::strip_ansi_codes(line);
        let interior = if plain.contains("more") {
            plain
        } else {
            plain
                .trim_start_matches('╰')
                .trim_end_matches('╯')
                .to_string()
        };
        let avail = width.saturating_sub(2 + chip_w + 1);
        let interior = xylitol_tui::truncate_to_width(&interior, avail, "…", false);
        let pad = avail.saturating_sub(xylitol_tui::visible_width(&interior));
        let border = self.operation_zone_paint();
        let mut out = border("╰");
        out.push_str(&border(&interior));
        out.push_str(&border(&"─".repeat(pad)));
        out.push(' ');
        let mut col = 1 + avail + 1;
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
        out.push_str(&border("╯"));
        (out, hits)
    }

    pub(crate) fn try_click_painted_chip(&mut self, col: usize, line: &str) -> bool {
        let plain = xylitol_tui::utils::strip_ansi_codes(line);
        let width = xylitol_tui::visible_width(&plain);
        if plain.contains('◨') && col >= width.saturating_sub(2) {
            self.click_footer_action(FooterAction::Sidebar);
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
                let had_body = !self.ui_model.entries.is_empty();
                self.cycle_display_level();
                if !had_body {
                    let level = self.activity.settings.display_level;
                    self.push_client_notice(
                        super::ClientNoticeKind::Info,
                        format!("显示等级: {level:?}"),
                    );
                }
            }
        }
        true
    }

    pub(crate) fn try_right_click_summary(&mut self, col: usize, line: &str) -> bool {
        let Some(_) = self
            .chip_hits_for_painted(line)
            .into_iter()
            .find(|h| h.action == FooterAction::FoldSummary && col >= h.start && col < h.end)
        else {
            return false;
        };
        let changed = self.reset_fold_to_initial();
        if changed {
            self.push_client_notice(super::ClientNoticeKind::Success, "重置折叠成功".to_string());
        } else {
            self.push_client_notice(
                super::ClientNoticeKind::Info,
                "折叠已是初始状态".to_string(),
            );
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
    fn chip_row_for(level: &str) -> String {
        let mut root = super::super::UiRoot::new();
        root.set_term_cols(120);
        root.set_thinking_level_ui(level.to_string());
        root.paint_bottom_chrome("", 120).0
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
        let chip = rgb_after(&row, "[▤ 摘要]");
        assert_eq!(
            chip, dashes,
            "inactive chip and border must share one colour"
        );
    }
}

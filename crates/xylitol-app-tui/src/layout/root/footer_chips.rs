use super::UiRoot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterAction {
    Details,
    Sidebar,
    ResetFold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FooterChipHit {
    start: usize,
    end: usize,
    pub action: FooterAction,
}

impl UiRoot {
    pub(crate) fn footer_chip_actions(&self) -> Vec<(FooterAction, &'static str, bool)> {
        let mut chips = vec![(FooterAction::Details, "详情", self.session_popup_open)];
        if self.session_sidebar_available() {
            chips.push((FooterAction::Sidebar, "侧栏", self.session_sidebar_on));
        }
        chips.push((FooterAction::ResetFold, "重置折叠", false));
        chips
    }

    fn reset_chip_enabled(&self) -> bool {
        let fold = self.fold();
        let defaults = crate::widgets::ScrollbackFold::default();
        !fold.tools_overrides.is_empty()
            || !fold.thinking_overrides.is_empty()
            || !fold.output_overrides.is_empty()
            || fold.thinking_expanded != defaults.thinking_expanded
            || fold.tools_expanded != defaults.tools_expanded
            || fold.tools_output_expanded != defaults.tools_output_expanded
            || fold.compaction_expanded != defaults.compaction_expanded
    }

    pub(super) fn append_border_chips(
        &self,
        line: &str,
        width: usize,
    ) -> (String, Vec<FooterChipHit>) {
        if width == 0 {
            return (line.to_string(), Vec::new());
        }
        let chips = self.footer_chip_actions();
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
        let dash_w = width.saturating_sub(chip_w + 1);
        let mut out = if dash_w == 0 {
            String::new()
        } else {
            self.theme.paint_muted(&"─".repeat(dash_w))
        };
        out.push(' ');
        let mut col = dash_w + 1;
        let mut hits = Vec::with_capacity(labels.len());
        for (i, (action, text, active)) in labels.iter().enumerate() {
            if i > 0 {
                out.push(' ');
                col += 1;
            }
            let w = xylitol_tui::visible_width(text);
            out.push_str(&self.theme.paint_chip(text, *active));
            hits.push(FooterChipHit {
                start: col,
                end: col + w,
                action: *action,
            });
            col += w;
        }
        (out, hits)
    }

    pub(crate) fn try_click_painted_chip(&mut self, col: usize, line: &str) -> bool {
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
            FooterAction::Details => {
                if self.session_popup_open {
                    self.close_session_popup();
                } else {
                    self.request_session_popup();
                }
            }
            FooterAction::Sidebar => {
                if self.session_sidebar_on {
                    self.close_session_sidebar();
                } else {
                    self.request_session_sidebar();
                }
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

//! Editor border + thinking-level UI sync for [`UiRoot`] (c1150 / ati15).

use xylitol_tui::{ThinkingBorderLevel, apply_thinking_border};

use super::UiRoot;
use crate::domain::types::ThinkingLevel;

impl UiRoot {
    /// Sync UI thinking level (border + footer); silent — no transcript.
    pub fn set_thinking_level_ui(&mut self, level: ThinkingLevel) {
        self.thinking_level = level;
        self.sync_editor_border();
        self.refresh_footer_from_queue(
            self.ui_model.queue.steer_count,
            self.ui_model.queue.follow_up_count,
        );
    }

    pub fn thinking_level(&self) -> ThinkingLevel {
        self.thinking_level
    }

    pub fn take_pending_thinking_cycle(&mut self) -> bool {
        std::mem::take(&mut self.pending_thinking_cycle)
    }

    /// Map domain thinking level → package border level by `as_str`.
    fn thinking_border_level(&self) -> ThinkingBorderLevel {
        ThinkingBorderLevel::parse(self.thinking_level.as_str()).unwrap_or(ThinkingBorderLevel::Off)
    }

    /// Sync operation-zone border: bash accent overrides; else thinking level.
    pub fn sync_editor_border(&mut self) {
        let bash = self.editor.get_text().trim_start().starts_with('!');
        self.bash_mode = bash;
        if bash {
            self.editor.set_border_color(self.theme.bash_border_color());
        } else {
            let palette = self.theme.palette();
            let level = self.thinking_border_level();
            apply_thinking_border(&mut self.editor, &palette, level);
        }
    }
}

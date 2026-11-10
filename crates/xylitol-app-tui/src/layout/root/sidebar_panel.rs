use xylitol_tui::{Component, InputEvent, truncate_to_width, visible_width};

use crate::layout::LayoutTheme;

pub struct SidebarPanel {
    theme: LayoutTheme,
}

impl SidebarPanel {
    pub fn new(theme: LayoutTheme) -> Self {
        Self { theme }
    }
}

impl Component for SidebarPanel {
    fn render(&mut self, width: usize) -> Vec<String> {
        if width == 0 {
            return Vec::new();
        }
        let title = "┌ 会话侧栏";
        let tw = visible_width(title);
        let title_line = self.theme.paint_muted(&format!(
            "{title} {}",
            "─".repeat(width.saturating_sub(tw + 1))
        ));
        vec![
            truncate_to_width(&title_line, width, "", false),
            self.theme.paint_muted("实验特性 · 施工中"),
            self.theme.paint_muted("正式业务后启用"),
        ]
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn invalidate(&mut self) {}
}

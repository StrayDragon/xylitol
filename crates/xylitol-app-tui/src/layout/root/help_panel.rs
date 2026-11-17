use crossterm::event::{KeyCode, KeyEvent};
use xylitol_tui::{
    BorderedPanel, Component, KeybindingsManager, fg_rgb, truncate_to_width, visible_width,
};

use crate::keybindings::with_keybindings;
use crate::layout::LayoutTheme;

use super::UiRoot;

struct HelpItem {
    keys: String,
    label: String,
}

struct HelpGroup {
    title: &'static str,
    items: Vec<HelpItem>,
}

struct StyledLine {
    plain: String,
    painted: String,
}

pub(crate) struct HelpPanel {
    open: bool,
    scroll: usize,
    max_scroll: usize,
}

impl HelpPanel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            scroll: 0,
            max_scroll: 0,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(crate) fn open(&mut self) {
        self.open = true;
        self.scroll = 0;
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.scroll = 0;
    }

    pub(crate) fn set_scroll(&mut self, scroll: usize) {
        self.scroll = scroll.min(self.max_scroll);
    }

    pub(crate) fn set_max_scroll(&mut self, max_scroll: usize) {
        self.max_scroll = max_scroll;
        self.scroll = self.scroll.min(max_scroll);
    }

    pub(crate) fn scroll_by(&mut self, delta: isize) {
        let next = if delta < 0 {
            self.scroll.saturating_sub(delta.unsigned_abs())
        } else {
            self.scroll.saturating_add(delta as usize)
        };
        self.scroll = next.min(self.max_scroll);
    }
}

impl UiRoot {
    pub(crate) fn hint_visible(&self) -> bool {
        self.help.is_open() || (self.slot.is_editor() && self.editor.get_text().is_empty())
    }

    pub(crate) fn hint_rows(&self) -> usize {
        1
    }

    pub(crate) fn help_open(&self) -> bool {
        self.help.is_open()
    }

    #[cfg(test)]
    pub(crate) fn help_scroll(&self) -> usize {
        self.help.scroll()
    }

    pub(crate) fn open_help(&mut self) {
        self.help.open();
        self.bump_upper_gen();
    }

    pub(crate) fn close_help(&mut self) {
        self.help.close();
        self.bump_upper_gen();
    }

    pub(crate) fn handle_help_key(&mut self, key: &KeyEvent) -> bool {
        if self.help.is_open() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => {
                    self.close_help();
                    true
                }
                KeyCode::Up => {
                    self.help.scroll_by(-1);
                    true
                }
                KeyCode::Down => {
                    self.help.scroll_by(1);
                    true
                }
                KeyCode::PageUp => {
                    self.help.scroll_by(-(self.help_viewport_rows() as isize));
                    true
                }
                KeyCode::PageDown => {
                    self.help.scroll_by(self.help_viewport_rows() as isize);
                    true
                }
                _ => true,
            }
        } else if key.code == KeyCode::Char('?')
            && self.slot.is_editor()
            && self.editor.get_text().is_empty()
        {
            self.open_help();
            true
        } else {
            false
        }
    }

    fn help_viewport_rows(&self) -> usize {
        let base_dock = self
            .last_dock_rows
            .saturating_sub(self.last_help_rows)
            .max(4);
        self.term_rows
            .saturating_sub(base_dock)
            .saturating_sub(2)
            .max(3)
    }

    pub(super) fn render_hint_slot(&mut self, width: usize) -> Vec<String> {
        if !self.hint_visible() {
            return vec![String::new()];
        }
        let (head, tail) = if self.help.is_open() {
            ("?", " / Esc close")
        } else {
            ("?", " shortcuts")
        };
        let painted = format!(
            "{}{}",
            fg_rgb(self.theme.palette().accent, head),
            self.theme.paint_muted(tail)
        );
        let line = truncate_to_width(&painted, width.max(1), "…", false);
        vec![line]
    }

    pub(super) fn render_help_slot(&mut self, width: usize, max_rows: usize) -> Vec<String> {
        if width < 12 || max_rows < 5 {
            return Vec::new();
        }
        let content_w = width.saturating_sub(4).max(1);
        crate::keybindings::ensure_product_catalog();
        let content = with_keybindings(|kb| build_help_content(self.theme, content_w, kb));
        let viewport = self
            .help_viewport_rows()
            .min(max_rows.saturating_sub(2))
            .max(3);
        self.help
            .set_max_scroll(content.len().saturating_sub(viewport));
        let mut panel = BorderedPanel::new("快捷键");
        panel.set_lines(content);
        panel.set_viewport(viewport);
        panel.set_scroll(self.help.scroll());
        let theme = self.theme;
        panel.set_border_paint(move |text| theme.paint_muted(text));
        let accent = self.theme.palette().accent;
        panel.set_title_paint(move |text| fg_rgb(accent, text));
        let out = panel.render(width);
        self.help.set_scroll(panel.scroll());
        out
    }
}

fn build_help_content(
    theme: LayoutTheme,
    content_w: usize,
    kb: &KeybindingsManager,
) -> Vec<String> {
    let groups = help_groups(kb);
    let terminal_w = content_w.saturating_add(4);
    let cols: usize = if terminal_w >= 100 {
        3
    } else if terminal_w >= 70 {
        2
    } else {
        1
    };
    let columns: Vec<Vec<StyledLine>> = match cols {
        3 => groups
            .iter()
            .map(|group| group_lines(theme, group, content_w))
            .collect(),
        2 => vec![
            merge_lines(
                group_lines(theme, &groups[0], content_w),
                group_lines(theme, &groups[1], content_w),
            ),
            group_lines(theme, &groups[2], content_w),
        ],
        _ => vec![
            groups
                .iter()
                .flat_map(|group| group_lines(theme, group, content_w))
                .collect(),
        ],
    };
    let gap = 2usize;
    let col_w = (content_w.saturating_sub(gap * cols.saturating_sub(1))) / cols;
    let max_rows = columns.iter().map(Vec::len).max().unwrap_or(0);
    let mut rows = Vec::with_capacity(max_rows);
    for i in 0..max_rows {
        let mut painted = String::new();
        for (j, column) in columns.iter().enumerate() {
            if j > 0 {
                painted.push_str("  ");
            }
            if let Some(line) = column.get(i) {
                painted.push_str(&line.painted);
                let pad = col_w.saturating_sub(visible_width(&line.plain));
                painted.push_str(&" ".repeat(pad));
            } else {
                painted.push_str(&" ".repeat(col_w));
            }
        }
        rows.push(truncate_to_width(&painted, content_w, "…", false));
    }
    rows
}

fn merge_lines(mut first: Vec<StyledLine>, second: Vec<StyledLine>) -> Vec<StyledLine> {
    first.push(StyledLine {
        plain: String::new(),
        painted: String::new(),
    });
    first.extend(second);
    first
}

fn group_lines(theme: LayoutTheme, group: &HelpGroup, _width: usize) -> Vec<StyledLine> {
    let key_w = group
        .items
        .iter()
        .map(|item| visible_width(&item.keys))
        .max()
        .unwrap_or(0);
    let mut lines = Vec::with_capacity(group.items.len() + 1);
    lines.push(StyledLine {
        plain: group.title.to_string(),
        painted: fg_rgb(theme.palette().accent, group.title),
    });
    for item in &group.items {
        let pad = key_w.saturating_sub(visible_width(&item.keys));
        lines.push(StyledLine {
            plain: format!("{}{}  {}", item.keys, " ".repeat(pad), item.label),
            painted: format!(
                "{}{}  {}",
                theme.paint_warning(&item.keys),
                " ".repeat(pad),
                fg_rgb(theme.palette().on_surface, &item.label)
            ),
        });
    }
    lines
}

fn help_groups(kb: &KeybindingsManager) -> Vec<HelpGroup> {
    vec![
        HelpGroup {
            title: "输入",
            items: input_items(kb),
        },
        HelpGroup {
            title: "会话",
            items: session_items(kb),
        },
        HelpGroup {
            title: "显示与槽",
            items: display_items(kb),
        },
    ]
}

fn input_items(kb: &KeybindingsManager) -> Vec<HelpItem> {
    let mut items = vec![
        static_item("/", "命令"),
        static_item("@", "文件"),
        static_item("$", "技能"),
        static_item("!", "Shell"),
    ];
    for id in [
        "tui.input.newLine",
        "tui.input.submit",
        "app.editor.external",
        "app.paste.image",
        "app.clear",
        "app.interrupt",
        "app.message.followUp",
        "app.message.dequeue",
    ] {
        if let Some(item) = item(kb, id) {
            items.push(item);
        }
    }
    items.push(static_item("Esc Esc", "会话树"));
    items.push(static_item("?", "快捷键"));
    items
}

fn session_items(kb: &KeybindingsManager) -> Vec<HelpItem> {
    let mut items = Vec::new();
    for id in [
        "app.session.fork",
        "app.tree.editLabel",
        "app.tree.toggleLabelTimestamp",
        "app.session.rename",
        "app.session.delete",
        "app.session.toggleSort",
        "app.session.toggleNamedFilter",
        "app.session.togglePath",
        "app.session.toggleId",
    ] {
        if let Some(item) = item(kb, id) {
            items.push(item);
        }
    }
    items
}

fn display_items(kb: &KeybindingsManager) -> Vec<HelpItem> {
    let mut items = Vec::new();
    for id in [
        "app.slot.pageUp",
        "app.slot.pageDown",
        "tui.tree.foldOrUp",
        "tui.tree.unfoldOrDown",
        "app.tree.filter.default",
        "app.tree.filter.noTools",
        "app.tree.filter.userOnly",
        "app.tree.filter.labeledOnly",
        "app.tree.filter.all",
        "app.tree.filter.cycleForward",
        "app.tree.filter.cycleBackward",
    ] {
        if let Some(item) = item(kb, id) {
            items.push(item);
        }
    }
    items
}

fn item(kb: &KeybindingsManager, id: &'static str) -> Option<HelpItem> {
    let keys = kb.get_keys(id);
    if keys.is_empty() {
        return None;
    }
    Some(HelpItem {
        keys: crate::layout::key_format::format_keys(&keys),
        label: label_for(kb, id),
    })
}

fn static_item(keys: &'static str, label: &'static str) -> HelpItem {
    HelpItem {
        keys: keys.to_string(),
        label: label.to_string(),
    }
}

fn label_for(kb: &KeybindingsManager, id: &'static str) -> String {
    let label = match id {
        "tui.input.newLine" => "换行",
        "tui.input.submit" => "提交",
        "app.editor.external" => "外部编辑器",
        "app.paste.image" => "粘贴图片",
        "app.clear" => "清空 / 退出",
        "app.interrupt" => "中止",
        "app.message.followUp" => "排队 follow-up",
        "app.message.dequeue" => "取回排队",
        "app.session.fork" => "分叉",
        "app.tree.editLabel" => "编辑标签",
        "app.tree.toggleLabelTimestamp" => "标签时间",
        "app.session.rename" => "重命名",
        "app.session.delete" => "删除",
        "app.session.toggleSort" => "排序",
        "app.session.toggleNamedFilter" => "命名过滤",
        "app.session.togglePath" => "路径列",
        "app.session.toggleId" => "会话 id 列",
        "app.slot.pageUp" => "详情上翻",
        "app.slot.pageDown" => "详情下翻",
        "tui.tree.foldOrUp" => "折叠 / 上移",
        "tui.tree.unfoldOrDown" => "展开 / 下移",
        "app.tree.filter.default" => "默认过滤",
        "app.tree.filter.noTools" => "隐藏工具",
        "app.tree.filter.userOnly" => "仅用户",
        "app.tree.filter.labeledOnly" => "仅标签",
        "app.tree.filter.all" => "全部",
        "app.tree.filter.cycleForward" => "过滤循环",
        "app.tree.filter.cycleBackward" => "反向循环",
        _ => "",
    };
    if label.is_empty() {
        kb.get_definition(id)
            .and_then(|def| def.description)
            .unwrap_or(id)
            .to_string()
    } else {
        label.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::build_product_keybindings;
    use xylitol_tui::KeybindingsConfig;

    #[test]
    fn custom_keybinding_is_reflected_in_help_panel() {
        let mut user = KeybindingsConfig::new();
        user.insert("app.interrupt".into(), vec!["ctrl+x".into()]);
        let kb = build_product_keybindings(user);
        let groups = help_groups(&kb);
        let interrupt = groups[0]
            .items
            .iter()
            .find(|item| item.label == "中止")
            .expect("interrupt item");
        assert_eq!(interrupt.keys, "Ctrl+X");
    }

    #[test]
    fn wide_width_uses_three_columns() {
        let kb = build_product_keybindings(KeybindingsConfig::new());
        let content = build_help_content(LayoutTheme::product_dark(), 120, &kb);
        assert!(!content.is_empty());
        assert!(
            content.iter().any(|line| line.contains("输入")),
            "input group header must render: {content:?}"
        );
    }
}

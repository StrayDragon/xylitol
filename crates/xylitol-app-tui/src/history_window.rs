use crate::bridge::{UiEntry, UiModel};

pub const HISTORY_PAGE: usize = 200;
pub const HISTORY_WINDOW: usize = 800;

#[derive(Debug, Clone, Default)]
pub struct LoadedHistory {
    pub session_id: String,
    pub entries: Vec<xylitol_plugin_sdk::session::SessionEntry>,
    pub first_entry_id: Option<String>,
    pub has_more: bool,
}

impl LoadedHistory {
    pub fn new(
        session_id: &str,
        entries: Vec<xylitol_plugin_sdk::session::SessionEntry>,
        has_more: bool,
    ) -> Self {
        let first_entry_id = entries
            .iter()
            .find_map(|e| e.entry_id().map(str::to_string));
        Self {
            session_id: session_id.to_string(),
            entries,
            first_entry_id,
            has_more,
        }
    }

    pub fn merge_older(
        &mut self,
        older: Vec<xylitol_plugin_sdk::session::SessionEntry>,
        has_more: bool,
    ) {
        let mut merged = Vec::with_capacity(older.len() + self.entries.len());
        let mut seen = std::collections::HashSet::new();
        for entry in older.into_iter().chain(self.entries.drain(..)) {
            let Some(id) = entry.entry_id().map(str::to_string) else {
                merged.push(entry);
                continue;
            };
            if seen.insert(id) {
                merged.push(entry);
            }
        }
        self.first_entry_id = merged.iter().find_map(|e| e.entry_id().map(str::to_string));
        self.entries = merged;
        self.has_more = has_more;
    }
}

pub fn load_earlier_notice_text() -> String {
    format!("↑ 加载更早的 {HISTORY_PAGE} 条（还有更早内容）")
}

pub fn collapsed_notice_text() -> String {
    "↑ 更早内容已收起（点击展开）".to_string()
}

fn notice_text(has_earlier: bool, collapsed: bool) -> Option<String> {
    if has_earlier {
        Some(load_earlier_notice_text())
    } else if collapsed {
        Some(collapsed_notice_text())
    } else {
        None
    }
}

fn apply_notice(model: &mut UiModel, text: Option<String>) {
    let notice_now = matches!(model.entries.first(), Some(UiEntry::ScrollNotice { .. }));
    match (text, notice_now) {
        (Some(text), true) => {
            if let Some(UiEntry::ScrollNotice { text: current }) = model.entries.first_mut() {
                *current = text;
            }
        }
        (Some(text), false) => model.entries.insert(0, UiEntry::ScrollNotice { text }),
        (None, true) => {
            model.entries.remove(0);
        }
        (None, false) => {}
    }
}

pub fn trim_to_window(model: &mut UiModel, has_earlier: bool) {
    model.has_earlier = has_earlier;
    let notice_present = matches!(model.entries.first(), Some(UiEntry::ScrollNotice { .. }));
    let body_len = model.entries.len() - usize::from(notice_present);

    let drop = body_len.saturating_sub(HISTORY_WINDOW);
    if drop > 0 {
        let start = usize::from(notice_present) + drop;
        model.entries.drain(..start);
    }

    apply_notice(model, notice_text(has_earlier, drop > 0));
}

pub fn show_notice(model: &mut UiModel, has_earlier: bool) {
    model.has_earlier = has_earlier;
    apply_notice(model, notice_text(has_earlier, false));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &str) -> UiEntry {
        UiEntry::User {
            text: text.to_string(),
        }
    }

    fn model_with(n: usize) -> UiModel {
        let mut model = UiModel::default();
        model.entries = (0..n).map(|i| user(&format!("m{i}"))).collect();
        model
    }

    #[test]
    fn short_history_gets_no_notice() {
        let mut model = model_with(50);
        trim_to_window(&mut model, false);
        assert_eq!(model.entries.len(), 50);
        assert!(
            !matches!(model.entries.first(), Some(UiEntry::ScrollNotice { .. })),
            "a short session must never show a pagination hint"
        );
        assert!(!model.has_earlier);
    }

    #[test]
    fn window_overflow_replaces_oldest_with_notice() {
        let mut model = model_with(HISTORY_WINDOW + HISTORY_PAGE);
        trim_to_window(&mut model, true);
        assert!(
            matches!(model.entries.first(), Some(UiEntry::ScrollNotice { .. })),
            "the oldest page must collapse into a clickable notice"
        );
        assert_eq!(
            model.entries.len(),
            HISTORY_WINDOW + 1,
            "body stays at the window cap, plus the notice row"
        );
        assert!(model.has_earlier);
    }

    #[test]
    fn earlier_on_disk_shows_notice_without_overflow() {
        let mut model = model_with(HISTORY_PAGE);
        trim_to_window(&mut model, true);
        assert!(
            matches!(model.entries.first(), Some(UiEntry::ScrollNotice { .. })),
            "unloaded history needs an affordance even below the window cap"
        );
        assert_eq!(model.entries.len(), HISTORY_PAGE + 1);
    }

    #[test]
    fn overflow_without_earlier_on_disk_uses_collapsed_copy() {
        let mut model = model_with(HISTORY_WINDOW + HISTORY_PAGE);
        trim_to_window(&mut model, false);
        match model.entries.first() {
            Some(UiEntry::ScrollNotice { text }) => {
                assert_eq!(text, &collapsed_notice_text());
                assert!(
                    !text.contains("加载更早"),
                    "a full window with nothing earlier on disk must not pretend to load more"
                );
            }
            other => panic!("expected a collapsed notice, got {other:?}"),
        }
    }

    #[test]
    fn notice_disappears_once_nothing_earlier_remains() {
        let mut model = model_with(HISTORY_WINDOW);
        trim_to_window(&mut model, true);
        assert!(matches!(
            model.entries.first(),
            Some(UiEntry::ScrollNotice { .. })
        ));
        trim_to_window(&mut model, false);
        assert!(
            !matches!(model.entries.first(), Some(UiEntry::ScrollNotice { .. })),
            "no earlier entries means no hint"
        );
    }

    #[test]
    fn repeated_trims_do_not_stack_notices() {
        let mut model = model_with(HISTORY_WINDOW + 2 * HISTORY_PAGE);
        trim_to_window(&mut model, true);
        trim_to_window(&mut model, true);
        let notices = model
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::ScrollNotice { .. }))
            .count();
        assert_eq!(notices, 1, "the hint must never duplicate");
    }

    #[test]
    fn show_notice_updates_or_removes_without_trimming() {
        let mut model = model_with(HISTORY_WINDOW + HISTORY_PAGE);
        trim_to_window(&mut model, false);
        let body_before = model.entries.len() - 1;
        show_notice(&mut model, true);
        assert_eq!(model.entries.len(), body_before + 1);
        match model.entries.first() {
            Some(UiEntry::ScrollNotice { text }) => {
                assert_eq!(text, &load_earlier_notice_text());
            }
            other => panic!("expected load-earlier notice, got {other:?}"),
        }
        show_notice(&mut model, false);
        assert!(
            !matches!(model.entries.first(), Some(UiEntry::ScrollNotice { .. })),
            "no earlier history must clear the hint"
        );
        assert_eq!(model.entries.len(), body_before);
    }

    #[test]
    fn merge_older_dedupes_by_entry_id() {
        use xylitol_plugin_sdk::session::{EntryBase, SessionEntry};

        fn entry(id: &str) -> SessionEntry {
            SessionEntry::Custom(xylitol_plugin_sdk::session::CustomEntry {
                base: EntryBase {
                    entry_type: "custom".into(),
                    id: id.into(),
                    parent_id: None,
                    timestamp: 0,
                },
                custom_type: "test".into(),
                data: serde_json::Value::Null,
            })
        }

        let mut history = LoadedHistory::new("s", vec![entry("b"), entry("c")], true);
        history.merge_older(vec![entry("a"), entry("b")], false);
        assert_eq!(history.entries.len(), 3, "duplicates must not be re-added");
        assert_eq!(history.first_entry_id.as_deref(), Some("a"));
        assert!(!history.has_more);
    }
}

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
}

pub fn load_earlier_notice_text() -> String {
    format!("↑ 加载更早的 {HISTORY_PAGE} 条")
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

    let want_notice = has_earlier || drop > 0;
    let notice_now = matches!(model.entries.first(), Some(UiEntry::ScrollNotice { .. }));
    match (want_notice, notice_now) {
        (true, false) => model.entries.insert(
            0,
            UiEntry::ScrollNotice {
                text: load_earlier_notice_text(),
            },
        ),
        (false, true) => {
            model.entries.remove(0);
        }
        _ => {}
    }
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
}

//! Empty / stub slot widgets for [`UiRoot`] construction (ath12 split).

use xylitol_tui::components::select_list::{SelectItem, SelectList, SelectListLayoutOptions};
use xylitol_tui::{TreeSelector, TreeSelectorOptions};

use super::super::theme::LayoutTheme;
use crate::app::tui::layout::DEFAULT_MAX_VISIBLE;
use crate::app::tui::session_resume::SessionResumePanel;

pub(super) fn empty_tree_selector(theme: LayoutTheme) -> TreeSelector {
    TreeSelector::new(
        Vec::new(),
        theme.tree_selector_theme(),
        TreeSelectorOptions {
            max_visible: DEFAULT_MAX_VISIBLE,
            unicode_connectors: true,
            include_node: None,
            active_id: None,
            status_suffix: None,
        },
    )
}

pub(super) fn empty_models_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        Vec::new(),
        DEFAULT_MAX_VISIBLE,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(24),
            max_primary_column_width: Some(48),
            truncate_primary: None,
        },
    )
}

pub(super) fn empty_themes_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        Vec::new(),
        DEFAULT_MAX_VISIBLE,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(12),
            max_primary_column_width: Some(24),
            truncate_primary: None,
        },
    )
}

pub(super) fn empty_mcp_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        Vec::new(),
        DEFAULT_MAX_VISIBLE,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(24),
            max_primary_column_width: Some(72),
            truncate_primary: None,
        },
    )
}

pub(super) fn import_confirm_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        vec![SelectItem::new("yes", "Yes"), SelectItem::new("no", "No")],
        DEFAULT_MAX_VISIBLE,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(8),
            max_primary_column_width: Some(24),
            truncate_primary: None,
        },
    )
}

pub(super) fn empty_session_resume_panel(theme: LayoutTheme) -> SessionResumePanel {
    SessionResumePanel::new(theme)
}

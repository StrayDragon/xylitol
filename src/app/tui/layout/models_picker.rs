//! Models picker: model + xylitol thinking levels (c1470).

use crate::app::core::driver::ModelInfo;
use crate::protocol::model::{THINKING_OFF, thinking_levels_are_adjustable};
use xylitol_tui::SelectItem;
use xylitol_tui::visible_width;

/// One row in the models picker catalog.
#[derive(Debug, Clone)]
pub struct ModelPickerRow {
    pub id: String,
    pub label: String,
    pub levels: Vec<String>,
    pub provisional: String,
}

impl ModelPickerRow {
    pub fn from_info(m: &ModelInfo, current_id: Option<&str>, current_thinking: String) -> Self {
        let levels = if m.thinking_levels.is_empty() {
            vec![THINKING_OFF.into()]
        } else {
            m.thinking_levels.clone()
        };
        let adjustable = thinking_levels_are_adjustable(&levels);
        let is_current = current_id.is_some_and(|id| id == m.id);
        let provisional = if !adjustable {
            THINKING_OFF.into()
        } else if is_current && levels.iter().any(|level| level == &current_thinking) {
            current_thinking
        } else {
            levels
                .last()
                .cloned()
                .unwrap_or_else(|| THINKING_OFF.into())
        };
        let label = if m.display_name.is_empty() {
            m.id.clone()
        } else {
            m.display_name.clone()
        };
        Self {
            id: m.id.clone(),
            label,
            levels,
            provisional,
        }
    }

    pub fn adjustable(&self) -> bool {
        thinking_levels_are_adjustable(&self.levels)
    }

    pub fn cycle_provisional(&mut self, forward: bool) {
        if !self.adjustable() || self.levels.is_empty() {
            return;
        }
        let idx = self
            .levels
            .iter()
            .position(|level| level == &self.provisional)
            .unwrap_or(0);
        let next = if forward {
            (idx + 1) % self.levels.len()
        } else {
            (idx + self.levels.len() - 1) % self.levels.len()
        };
        self.provisional = self.levels[next].clone();
    }

    fn levels_desc(&self, focused: bool, width_budget: usize) -> String {
        if !self.adjustable() {
            return "—".into();
        }
        let wide = format_levels_wide(&self.levels, &self.provisional);
        let needed = visible_width(&self.label) + 2 + visible_width(&wide);
        if focused && needed <= width_budget.max(1) {
            wide
        } else {
            self.provisional.clone()
        }
    }

    pub fn to_select_item(&self, focused: bool, width_budget: usize) -> SelectItem {
        let mark = if focused { "* " } else { "  " };
        let label = format!("{mark}{}", self.label);
        SelectItem::new(self.id.clone(), label)
            .with_description(self.levels_desc(focused, width_budget))
    }
}

fn format_levels_wide(levels: &[String], active: &str) -> String {
    levels
        .iter()
        .map(|l| {
            if l == active {
                format!("[{l}]")
            } else {
                l.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Pending confirm from models picker.
#[derive(Debug, Clone)]
pub struct PendingModelChoice {
    pub model_id: String,
    pub thinking: String,
}

/// Next-turn cue copy for selected ≠ active (c1470).
pub fn status_next_turn_cue_text(
    active_model: &str,
    active_thinking: &str,
    selected_model: &str,
    selected_thinking: &str,
) -> Option<String> {
    if selected_model != active_model {
        return Some(format!("Next turn: {selected_model}"));
    }
    if selected_thinking != active_thinking {
        return Some(format!("Next turn thinking: {}", selected_thinking));
    }
    None
}

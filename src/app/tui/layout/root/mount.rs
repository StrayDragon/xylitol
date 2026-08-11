//! Shared root mount helpers + too-small hint (split from `UiRoot` god file).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[cfg(test)]
use xylitol_tui::TreeNode;
use xylitol_tui::{
    Component, InputEvent, InputListenerResult, TUI, Terminal, truncate_to_width, visible_width,
};

use super::UiRoot;
use crate::app::tui::host::{LayoutMode, TOO_SMALL_HINT};

/// Shared root so InputListeners and the focused Component see the same state.
pub struct SharedUiRoot(pub Rc<RefCell<UiRoot>>);

impl Component for SharedUiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.0.borrow_mut().render(width)
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.0.borrow_mut().handle_input(event);
    }

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        self.0.borrow().editor_wants_rerender(event)
    }

    fn take_pending_clipboard(&mut self) -> Vec<String> {
        Component::take_pending_clipboard(&mut *self.0.borrow_mut())
    }

    fn mode_b_dock_rows_hint(&self) -> Option<usize> {
        Some(self.0.borrow().last_mode_b_dock_rows())
    }

    fn wants_pointer_motion(&self) -> bool {
        self.0.borrow().editor.is_selection_dragging()
    }

    fn invalidate(&mut self) {
        self.0.borrow_mut().invalidate();
    }

    fn tick(&mut self) -> bool {
        self.0.borrow_mut().tick()
    }
}

/// Full-screen hint when the terminal is too small.
pub struct TooSmallHint;

impl Component for TooSmallHint {
    fn render(&mut self, width: usize) -> Vec<String> {
        let msg = TOO_SMALL_HINT;
        if width == 0 {
            return vec![String::new()];
        }
        // CJK: `chars().count()` under-counts columns and tripped the width
        // invariant (RenderError → host exit) so recovery from tiny sizes stuck.
        let msg_w = visible_width(msg);
        if msg_w >= width {
            return vec![truncate_to_width(msg, width, "", true)];
        }
        let left = (width - msg_w) / 2;
        let right = width - msg_w - left;
        vec![format!("{}{msg}{}", " ".repeat(left), " ".repeat(right))]
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn invalidate(&mut self) {}
}

/// Build root children for a layout mode (no shared state — simple tests).
#[cfg(test)]
pub fn build_root(mode: LayoutMode) -> Vec<Box<dyn Component>> {
    match mode {
        LayoutMode::Ready => vec![Box::new(UiRoot::new())],
        LayoutMode::TooSmall => vec![Box::new(TooSmallHint)],
    }
}

/// Shared root + rebuild closure that keeps the same `UiRoot` across min-size flips.
pub fn shared_ui_root_rebuild(
    root: Rc<RefCell<UiRoot>>,
) -> impl FnMut(LayoutMode) -> Vec<Box<dyn Component>> + 'static {
    move |mode| match mode {
        LayoutMode::Ready => vec![Box::new(SharedUiRoot(root.clone()))],
        LayoutMode::TooSmall => vec![Box::new(TooSmallHint)],
    }
}

/// Register pre-focus Ctrl+C / Esc (editor-slot overlays). Busy Esc abort is host-side (c480).
pub fn install_ui_root_key_listeners<T: Terminal>(
    root: &Rc<RefCell<UiRoot>>,
    quit_flag: &Arc<AtomicBool>,
    tui: &mut TUI<T>,
) {
    let root = root.clone();
    let quit_flag = quit_flag.clone();
    tui.add_input_listener(move |event| {
        let InputEvent::Key(key) = &event else {
            return InputListenerResult::Continue;
        };
        if crate::app::tui::keybindings::matches_binding(key, "app.clear") {
            root.borrow_mut().on_ctrl_c(&quit_flag);
            return InputListenerResult::Consumed;
        }
        if crate::app::tui::keybindings::matches_binding(key, "app.interrupt")
            && root.borrow_mut().on_escape()
        {
            return InputListenerResult::Consumed;
        }
        InputListenerResult::Continue
    });
}

#[cfg(test)]
pub(crate) fn sample_tree_nodes_for_test() -> Vec<TreeNode> {
    vec![
        TreeNode::new("root", "session · product").with_children([
            TreeNode::new("meta1", "fake / model").with_kind("meta"),
            TreeNode::new("u1", "hello")
                .with_kind("user")
                .with_annotation("keep")
                .with_child(
                    TreeNode::new("a1", "plan")
                        .with_kind("assistant")
                        .with_children([
                            TreeNode::new("t1", "read").with_kind("tool"),
                            TreeNode::new("a2", "done")
                                .with_kind("assistant")
                                .with_child(TreeNode::new("u2", "next").with_kind("user")),
                        ]),
                ),
            TreeNode::new("fork", "alternate")
                .with_kind("user")
                .with_child(TreeNode::new("af", "fork leaf").with_kind("assistant")),
        ]),
    ]
}

//! Product TUI root layout — transcript / editor|tree slot / footer.
//!
//! Named `UiRoot` (not `shell`/`scene`) to avoid clashing with bash /
//! `infra::process::shell` and to read as the product component tree root.
//!
//! # Freeze (c491 stub)
//!
//! Session tree here is a **static fake** for slot-replace smoke only
//! (double Esc / Esc close / Enter `travel → {id}`). Do **not** extend this
//! stub with live graphs, filters, or Driver travel until the demo-first gate
//! in `AGENTS.md` is explicitly opened. Morphology SSOT: `agent_demo`.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use xylitol_tui::components::editor::{Editor, EditorOptions, EditorTheme};
use xylitol_tui::components::text::Text;
use xylitol_tui::{
    Component, Focusable, InputEvent, InputListenerResult, SystemClock, TUI, Terminal, TreeNode,
    TreeSelector, TreeSelectorOptions, TreeSelectorTheme, matches_key_event,
};

use super::host::{LayoutMode, TOO_SMALL_HINT};

/// Fake session tree for the **c491 stub only** (frozen).
/// Real graph / Driver travel waits for demo-first gate open — see `AGENTS.md`.
fn sample_session_tree() -> Vec<TreeNode> {
    vec![
        TreeNode::new("root", "session · product").with_children([
            TreeNode::new("u1", "user: hello").with_child(
                TreeNode::new("a1", "assistant: plan").with_children([
                    TreeNode::new("t1", "tool: read"),
                    TreeNode::new("a2", "assistant: done")
                        .with_child(TreeNode::new("u2", "user: next")),
                ]),
            ),
            TreeNode::new("fork", "user: alternate")
                .with_child(TreeNode::new("af", "assistant: fork leaf")),
        ]),
    ]
}

fn product_tree_selector(active_id: &str) -> TreeSelector {
    TreeSelector::new(
        sample_session_tree(),
        TreeSelectorTheme::default(),
        TreeSelectorOptions {
            max_visible: 10,
            unicode_connectors: true,
            include_node: None,
            active_id: Some(active_id.into()),
            status_suffix: Some("[stub]".into()),
        },
    )
}

/// Root UI: transcript placeholder + bordered editor|tree slot + footer.
pub struct UiRoot {
    transcript: Text,
    editor: Editor,
    footer: Text,
    tree_open: bool,
    tree: TreeSelector,
    last_esc_at: Option<Instant>,
}

impl UiRoot {
    pub fn new() -> Self {
        let mut editor = Editor::new(
            EditorTheme::default(),
            EditorOptions::default(),
            Box::new(SystemClock),
        );
        editor.set_focused(true);
        Self {
            transcript: Text::new(
                "(empty transcript — bridge lands in a later change)".into(),
                0,
                0,
            ),
            editor,
            footer: Text::new(
                "double Esc tree · esc close/abort · ctrl+c clear/quit · /exit".into(),
                0,
                0,
            ),
            tree_open: false,
            tree: product_tree_selector("u2"),
            last_esc_at: None,
        }
    }

    /// Current editor text (harness / listeners).
    pub fn editor_text(&self) -> String {
        self.editor.get_text()
    }

    /// Replace editor text (harness).
    pub fn set_editor_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text.into());
    }

    pub fn tree_open(&self) -> bool {
        self.tree_open
    }

    /// Ctrl+C: clear editor when non-empty; otherwise signal quit via `quit_flag`.
    pub fn on_ctrl_c(&mut self, quit_flag: &AtomicBool) {
        if self.tree_open {
            self.close_session_tree();
            return;
        }
        if !self.editor.get_text().is_empty() {
            self.editor.set_text(String::new());
            return;
        }
        quit_flag.store(true, Ordering::SeqCst);
    }

    /// Esc: close tree; or empty-editor double-Esc opens tree. Returns true if consumed.
    pub fn on_escape(&mut self) -> bool {
        if self.tree_open {
            self.close_session_tree();
            return true;
        }
        if self.editor.get_text().is_empty() {
            let now = Instant::now();
            if let Some(prev) = self.last_esc_at
                && now.duration_since(prev) < Duration::from_millis(500)
            {
                self.last_esc_at = None;
                self.open_session_tree();
                return true;
            }
            self.last_esc_at = Some(now);
            // First Esc alone: not yet a double — leave for future abort (c480).
            return false;
        }
        self.last_esc_at = None;
        false
    }

    pub fn open_session_tree(&mut self) {
        self.tree = product_tree_selector("u2");
        self.tree_open = true;
        self.footer = Text::new("esc close · ↑↓ · Enter travel".into(), 0, 0);
    }

    pub fn close_session_tree(&mut self) {
        self.tree_open = false;
        self.footer = Text::new(
            "double Esc tree · esc close/abort · ctrl+c clear/quit · /exit".into(),
            0,
            0,
        );
    }

    /// Harness: open tree without double-Esc timing.
    pub fn open_session_tree_for_test(&mut self) {
        self.editor.set_text(String::new());
        self.open_session_tree();
    }

    fn append_transcript_line(&mut self, line: impl Into<String>) {
        let prev = self.transcript.text();
        let next = if prev.is_empty() {
            line.into()
        } else {
            format!("{prev}\n{}", line.into())
        };
        self.transcript.set_text(next);
    }

    fn render_editor_slot(&mut self, width: usize) -> Vec<String> {
        if self.tree_open {
            let mut lines = Vec::new();
            lines.push(" Session tree".to_string());
            lines.push(" Up/Down  Enter travel  Esc close  (double Esc)".to_string());
            lines.extend(self.tree.render(width.max(1)));
            return lines;
        }
        self.editor.render(width.max(1))
    }
}

impl Default for UiRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for UiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.extend(self.transcript.render(width));
        lines.push(String::new());
        let border = "─".repeat(width.clamp(1, 80));
        lines.push(border.clone());
        lines.extend(self.render_editor_slot(width));
        lines.push(border);
        lines.extend(self.footer.render(width));
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        if self.tree_open {
            let InputEvent::Key(ref key) = event else {
                return;
            };
            if matches_key_event(key, "enter") {
                let id = self.tree.selected_id().unwrap_or("?").to_string();
                self.append_transcript_line(format!("travel → {id}"));
                self.close_session_tree();
                return;
            }
            if matches_key_event(key, "up")
                || matches_key_event(key, "down")
                || matches_key_event(key, "pageUp")
                || matches_key_event(key, "pageDown")
                || matches_key_event(key, "left")
                || matches_key_event(key, "right")
            {
                self.tree.handle_input(event);
            }
            return;
        }
        self.editor.handle_input(event);
    }

    fn invalidate(&mut self) {
        self.transcript.invalidate();
        self.editor.invalidate();
        self.footer.invalidate();
        self.tree.invalidate();
    }

    fn tick(&mut self) -> bool {
        self.editor.tick()
    }
}

/// Shared root so InputListeners and the focused Component see the same state.
pub struct SharedUiRoot(pub Rc<RefCell<UiRoot>>);

impl Component for SharedUiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.0.borrow_mut().render(width)
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.0.borrow_mut().handle_input(event);
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
            return vec![msg.into()];
        }
        let pad = width.saturating_sub(msg.chars().count()) / 2;
        vec![format!("{}{msg}", " ".repeat(pad))]
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

/// Register pre-focus Ctrl+C / Esc (session tree). Streaming Esc abort waits for c480.
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
        if matches_key_event(key, "ctrl+c") {
            root.borrow_mut().on_ctrl_c(&quit_flag);
            return InputListenerResult::Consumed;
        }
        if matches_key_event(key, "escape") && root.borrow_mut().on_escape() {
            return InputListenerResult::Consumed;
        }
        InputListenerResult::Continue
    });
}

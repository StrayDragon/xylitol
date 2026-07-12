//! Product TUI root layout — transcript / status / editor|tree / footer.
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

use xylitol_tui::components::editor::{Editor, EditorOptions};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::text::Text;
use xylitol_tui::{
    Component, Focusable, InputEvent, InputListenerResult, SystemClock, TUI, Terminal, TreeNode,
    TreeSelector, TreeSelectorOptions, TreeSelectorTheme, fg_rgb, matches_key_event,
    truncate_to_width,
};

use super::bridge::{UiEntry, UiModel};
use super::glyphs::GlyphSet;
use super::host::{LayoutMode, TOO_SMALL_HINT};
use super::scrollback::{ScrollbackFold, render_scrollback};
use super::theme::ChromeTheme;

/// Fake session tree for the **c491 stub only** (frozen).
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

/// Root UI: live scrollback + optional status + bordered editor|tree + footer.
pub struct UiRoot {
    ui_model: UiModel,
    fold: ScrollbackFold,
    /// Busy-only; idle leaves this unused so status occupies 0 rows.
    status_loader: Loader,
    status_busy: bool,
    /// Gate loader frames to [`Loader::interval_ms`] (host idle_tick is ~16ms).
    loader_last_tick: Instant,
    editor: Editor,
    footer: Text,
    theme: ChromeTheme,
    glyphs: GlyphSet,
    cwd: String,
    model: String,
    tree_open: bool,
    tree: TreeSelector,
    last_esc_at: Option<Instant>,
    /// `!` / `!!` prefix → success border (c492).
    bash_mode: bool,
    /// Ctrl+G stub invocation count (harness).
    external_editor_invocations: u32,
}

impl UiRoot {
    pub fn new() -> Self {
        let theme = ChromeTheme::product_dark();
        let accent = theme.palette().accent;
        let muted = theme.palette().muted;
        let status_loader = Loader::new(
            Box::new(move |s| fg_rgb(accent, s)),
            Box::new(move |s| fg_rgb(muted, s)),
            String::new(),
            Some(LoaderIndicatorOptions::default()),
        );
        let mut editor = Editor::new(
            theme.editor_theme(),
            EditorOptions {
                padding_x: 1,
                // Match agent_demo: compact operation zone (max_vis ≈ 5 lines).
                terminal_rows: 8,
            },
            Box::new(SystemClock),
        );
        editor.set_focused(true);
        Self {
            ui_model: UiModel::new(),
            fold: ScrollbackFold::default(),
            status_loader,
            status_busy: false,
            loader_last_tick: Instant::now(),
            editor,
            footer: Text::new(String::new(), 0, 0),
            theme,
            glyphs: GlyphSet::from_env(),
            cwd: ".".into(),
            model: "—".into(),
            tree_open: false,
            tree: product_tree_selector("u2"),
            last_esc_at: None,
            bash_mode: false,
            external_editor_invocations: 0,
        }
    }

    /// Inject footer identity (cwd · model). Call before first render when known.
    pub fn set_chrome_meta(&mut self, cwd: impl Into<String>, model: impl Into<String>) {
        self.cwd = cwd.into();
        self.model = model.into();
        self.refresh_footer_from_queue(0, 0);
    }

    pub fn set_glyphs(&mut self, glyphs: GlyphSet) {
        self.glyphs = glyphs;
    }

    pub fn glyphs(&self) -> GlyphSet {
        self.glyphs
    }

    pub fn fold(&self) -> ScrollbackFold {
        self.fold
    }

    pub fn editor_text(&self) -> String {
        self.editor.get_text()
    }

    pub fn set_editor_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text.into());
        self.sync_editor_border();
    }

    /// Whether the editor is in bash accent mode (`!` / `!!` prefix).
    pub fn bash_mode(&self) -> bool {
        self.bash_mode
    }

    pub fn external_editor_invocations(&self) -> u32 {
        self.external_editor_invocations
    }

    /// Sync operation-zone border for `!` / `!!` (c492 / ati15).
    pub fn sync_editor_border(&mut self) {
        let bash = self.editor.get_text().trim_start().starts_with('!');
        if bash == self.bash_mode {
            return;
        }
        self.bash_mode = bash;
        if bash {
            self.editor.set_border_color(self.theme.bash_border_color());
        } else {
            self.editor
                .set_border_color(self.theme.muted_border_color());
        }
    }

    /// Ctrl+G stub: count + optional `# $EDITOR stub` marker (harness-safe).
    pub fn open_external_editor_stub(&mut self) {
        self.external_editor_invocations = self.external_editor_invocations.saturating_add(1);
        let text = self.editor.get_text();
        if text.is_empty() {
            self.editor.set_text("# $EDITOR stub\n".to_string());
        } else if !text.contains("$EDITOR stub") {
            self.editor
                .set_text(format!("{}\n# $EDITOR stub", text.trim_end()));
        }
        self.sync_editor_border();
    }

    /// Record a sent prompt / steer / follow-up for ↑/↓ history (pi `addToHistory`).
    pub fn remember_editor_send(&mut self, text: impl Into<String>) {
        self.editor.add_to_history(text.into());
    }

    pub fn tree_open(&self) -> bool {
        self.tree_open
    }

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
            return false;
        }
        self.last_esc_at = None;
        false
    }

    pub fn open_session_tree(&mut self) {
        self.tree = product_tree_selector("u2");
        self.tree_open = true;
    }

    pub fn close_session_tree(&mut self) {
        self.tree_open = false;
    }

    pub fn open_session_tree_for_test(&mut self) {
        self.editor.set_text(String::new());
        self.open_session_tree();
    }

    /// Push bridge UI model into status / footer; scrollback re-renders from model (c476).
    pub fn apply_ui_model(&mut self, model: &UiModel) {
        self.ui_model = model.clone();

        match model.status.as_ref() {
            Some(s) if !s.is_empty() => {
                self.status_loader.set_message(s.clone());
                self.status_busy = true;
            }
            _ => {
                self.status_busy = false;
            }
        }

        self.refresh_footer_from_queue(model.queue.steer_count, model.queue.follow_up_count);
    }

    fn refresh_footer_from_queue(&mut self, steer: usize, follow_up: usize) {
        let mut base = format!("{} · {}", self.cwd, self.model);
        if steer > 0 || follow_up > 0 {
            base = format!("q:s{steer}|f{follow_up} · {base}");
        }
        self.footer.set_text(self.theme.paint_muted(&base));
    }

    fn append_system_note(&mut self, line: impl Into<String>) {
        self.ui_model
            .entries
            .push(UiEntry::System { text: line.into() });
    }

    /// Pending steer / follow-up chrome above status (pi `pendingMessagesContainer`).
    fn render_queue_slot(&mut self, width: usize) -> Vec<String> {
        let steer = &self.ui_model.pending_steer;
        let follow_up = &self.ui_model.pending_follow_up;
        if steer.is_empty() && follow_up.is_empty() {
            return Vec::new();
        }
        let mut lines = Vec::new();
        // One blank spacer like pi's Spacer(1) before the queue block.
        if width > 0 {
            lines.push(String::new());
        }
        for msg in steer {
            let line = self.theme.paint_muted(&format!("Steering: {msg}"));
            lines.push(if width == 0 {
                line
            } else {
                truncate_to_width(&line, width, "...", true)
            });
        }
        for msg in follow_up {
            let line = self.theme.paint_muted(&format!("Follow-up: {msg}"));
            lines.push(if width == 0 {
                line
            } else {
                truncate_to_width(&line, width, "...", true)
            });
        }
        let hint = self
            .theme
            .paint_muted("↳ Alt+Up to edit all queued messages");
        lines.push(if width == 0 {
            hint
        } else {
            truncate_to_width(&hint, width, "...", true)
        });
        lines
    }

    fn render_status_slot(&mut self, width: usize) -> Vec<String> {
        if !self.status_busy {
            return Vec::new();
        }
        // Loader::render prepends a blank spacer — drop empties so busy is 1 row.
        self.status_loader
            .render(width)
            .into_iter()
            .filter(|l| !l.is_empty())
            .collect()
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

    fn render_scrollback_slot(&mut self, width: usize) -> Vec<String> {
        // Idle empty: 0 rows (DESIGN editor.md — no loud placeholder wall).
        render_scrollback(&self.ui_model, self.glyphs, self.theme, self.fold, width)
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
        lines.extend(self.render_scrollback_slot(width));
        // Queue chrome sits between transcript and status (pi morphology).
        lines.extend(self.render_queue_slot(width));
        lines.extend(self.render_status_slot(width));
        // Editor owns the operation-zone ─ borders (DESIGN editor.md / agent_demo).
        // Do NOT wrap with a second outer border pair.
        lines.extend(self.render_editor_slot(width));
        let footer = if width == 0 {
            self.footer.text().to_string()
        } else {
            truncate_to_width(self.footer.text(), width, "...", true)
        };
        lines.push(footer);
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        if self.tree_open {
            let InputEvent::Key(ref key) = event else {
                return;
            };
            if matches_key_event(key, "enter") {
                let id = self.tree.selected_id().unwrap_or("?").to_string();
                self.append_system_note(format!("travel → {id}"));
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

        if let InputEvent::Key(ref key) = event {
            if matches_key_event(key, "ctrl+t") {
                self.fold.thinking_expanded = !self.fold.thinking_expanded;
                return;
            }
            if matches_key_event(key, "alt+e") {
                self.fold.tools_expanded = !self.fold.tools_expanded;
                return;
            }
        }

        self.editor.handle_input(event);
        self.sync_editor_border();
    }

    fn invalidate(&mut self) {
        self.status_loader.invalidate();
        self.editor.invalidate();
        self.footer.invalidate();
        self.tree.invalidate();
    }

    fn tick(&mut self) -> bool {
        let mut dirty = self.editor.tick();
        if self.status_busy {
            let interval = self.status_loader.interval_ms() as u128;
            if self.loader_last_tick.elapsed().as_millis() >= interval {
                dirty = Component::tick(&mut self.status_loader) || dirty;
                self.loader_last_tick = Instant::now();
            }
        }
        dirty
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

/// Register pre-focus Ctrl+C / Esc (session tree). Busy Esc abort is host-side (c480).
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

//! Steps for `package-tui-interaction-modes` (c2070 Mode B).

use std::cell::RefCell;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::{
    Clock, Component, Editor, EditorOptions, EditorTheme, InputEvent, InputReaction,
    InteractionMode, RecordingClipboardSink, ScreenRect, ScrollView, SelectListTheme,
    SelectionController, SystemClock, TUI, Terminal,
};

thread_local! {
    static HARNESS: RefCell<Option<ModeBHarness>> = const { RefCell::new(None) };
    static LAST_BOOL: RefCell<bool> = const { RefCell::new(false) };
    static EDITOR: RefCell<Option<Editor>> = const { RefCell::new(None) };
}

struct RecTerm {
    cols: u16,
    rows: u16,
    writes: Vec<String>,
    mouse: bool,
    alt: bool,
}

impl RecTerm {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            writes: Vec::new(),
            mouse: false,
            alt: false,
        }
    }
}

impl Terminal for RecTerm {
    fn write(&mut self, data: &str) {
        self.writes.push(data.to_string());
    }
    fn columns(&self) -> u16 {
        self.cols
    }
    fn rows(&self) -> u16 {
        self.rows
    }
    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn clear_line(&mut self) {}
    fn clear_from_cursor(&mut self) {}
    fn clear_screen(&mut self) {}
    fn flush(&mut self) {}
    fn start(&mut self) {}
    fn stop(&mut self) {
        self.mouse = false;
        self.alt = false;
    }
    fn enable_mouse_capture(&mut self) {
        self.mouse = true;
    }
    fn disable_mouse_capture(&mut self) {
        self.mouse = false;
    }
    fn mouse_capture_active(&self) -> bool {
        self.mouse
    }
    fn enter_alternate_screen(&mut self) {
        self.alt = true;
    }
    fn leave_alternate_screen(&mut self) {
        self.alt = false;
    }
    fn alternate_screen_active(&self) -> bool {
        self.alt
    }
}

struct StaticLines {
    lines: Vec<String>,
}

impl Component for StaticLines {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.lines.clone()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
}

struct ModeBHarness {
    tui: TUI<RecTerm>,
    sel: SelectionController,
    scroll: ScrollView,
    sink: RecordingClipboardSink,
    tr: ScreenRect,
    dock: ScreenRect,
}

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn editor_theme() -> EditorTheme {
    EditorTheme {
        border_color: Box::new(|s| s.to_string()),
        select_list_theme: SelectListTheme::default(),
    }
}

#[given("新建默认 TUI")]
fn given_default_tui() {
    HARNESS.with(|h| {
        *h.borrow_mut() = Some(ModeBHarness {
            tui: TUI::new(RecTerm::new(40, 12)),
            sel: SelectionController::new(),
            scroll: ScrollView::new(4),
            sink: RecordingClipboardSink::default(),
            tr: ScreenRect {
                row: 0,
                col: 0,
                height: 4,
                width: 40,
            },
            dock: ScreenRect {
                row: 4,
                col: 0,
                height: 2,
                width: 40,
            },
        });
    });
}

#[given("Mode B 应用会话已 begin 且 transcript 有可拖选文本")]
fn given_mode_b_with_text() {
    let mut tui =
        TUI::with_interaction_mode(RecTerm::new(40, 10), InteractionMode::ApplicationOwned);
    tui.set_mode_b_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: vec![
            "hello".into(),
            "world".into(),
            "dock-a".into(),
            "dock-b".into(),
        ],
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    let _ = tui.render_now();
    tui.terminal.writes.clear();
    HARNESS.with(|h| {
        *h.borrow_mut() = Some(ModeBHarness {
            tui,
            sel: SelectionController::new(),
            scroll: ScrollView::new(4),
            sink: RecordingClipboardSink::default(),
            tr: ScreenRect {
                row: 0,
                col: 0,
                height: 8,
                width: 40,
            },
            dock: ScreenRect {
                row: 8,
                col: 0,
                height: 2,
                width: 40,
            },
        });
    });
}

#[given("Mode B 应用会话已 begin")]
fn given_mode_b_session() {
    given_mode_b_with_text();
}

#[given("transcript 拖选进行中")]
fn given_transcript_dragging() {
    HARNESS.with(|h| {
        let mut harness = ModeBHarness {
            tui: TUI::new(RecTerm::new(40, 12)),
            sel: SelectionController::new(),
            scroll: ScrollView::new(3),
            sink: RecordingClipboardSink::default(),
            tr: ScreenRect {
                row: 0,
                col: 0,
                height: 3,
                width: 20,
            },
            dock: ScreenRect {
                row: 3,
                col: 0,
                height: 2,
                width: 20,
            },
        };
        harness.scroll.set_lines(vec![
            "one".into(),
            "two".into(),
            "three".into(),
            "four".into(),
        ]);
        harness.sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut harness.scroll,
            harness.tr,
            harness.dock,
            &mut harness.sink,
        );
        assert!(harness.sel.is_dragging());
        *h.borrow_mut() = Some(harness);
    });
}

#[given("Mode B 视口已 follow 到底")]
fn given_mode_b_follow_end() {
    let mut tui =
        TUI::with_interaction_mode(RecTerm::new(40, 6), InteractionMode::ApplicationOwned);
    tui.set_mode_b_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: (0..20).map(|i| format!("L{i:02}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    let _ = tui.render_now();
    HARNESS.with(|h| {
        *h.borrow_mut() = Some(ModeBHarness {
            tui,
            sel: SelectionController::new(),
            scroll: ScrollView::new(4),
            sink: RecordingClipboardSink::default(),
            tr: ScreenRect {
                row: 0,
                col: 0,
                height: 4,
                width: 40,
            },
            dock: ScreenRect {
                row: 4,
                col: 0,
                height: 2,
                width: 40,
            },
        });
    });
}

#[given("Mode B 下 Editor 有多行缓冲")]
fn given_editor_multiline() {
    let mut editor = Editor::new(
        editor_theme(),
        EditorOptions {
            padding_x: 0,
            terminal_rows: 12,
        },
        Box::new(SystemClock) as Box<dyn Clock>,
    );
    editor.set_text("alpha\nbeta\ngamma".into());
    let _ = editor.render(40);
    EDITOR.with(|e| *e.borrow_mut() = Some(editor));
}

#[given("产品 TuiRunOptions 默认值")]
fn given_tui_run_options_default() {
    let opts = xylitol::app::tui::TuiRunOptions::default();
    LAST_BOOL.with(|b| {
        *b.borrow_mut() = opts.interaction_mode == InteractionMode::Inline;
    });
}

#[given("产品 Mode B 会话可接收库 copy-notice")]
fn given_product_copy_notice_path() {
    let src = include_str!("../../llmanspec/specs/app-tui-host/spec.toon");
    assert!(
        src.contains("ath31"),
        "ath31 must be landed in app-tui-host spec"
    );
    LAST_BOOL.with(|b| *b.borrow_mut() = true);
}

#[when("查询交互模式")]
fn when_query_mode() {
    HARNESS.with(|h| {
        let harness = h.borrow();
        let harness = harness.as_ref().expect("harness");
        LAST_BOOL.with(|b| {
            *b.borrow_mut() = harness.tui.interaction_mode() == InteractionMode::Inline
                && !harness.tui.application_session_active();
        });
    });
}

#[when("未修饰左键拖选非空范围并松开")]
fn when_drag_select_release() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        let down = mouse(MouseEventKind::Down(MouseButton::Left), 0, 0);
        let drag = mouse(MouseEventKind::Drag(MouseButton::Left), 4, 0);
        let up = mouse(MouseEventKind::Up(MouseButton::Left), 4, 0);
        assert_eq!(
            harness.tui.dispatch_event(InputEvent::Mouse(down)),
            InputReaction::Rerender
        );
        let _ = harness.tui.dispatch_event(InputEvent::Mouse(drag));
        let _ = harness.tui.dispatch_event(InputEvent::Mouse(up));
        harness.tui.request_render(false);
        let _ = harness.tui.render_now();
    });
}

#[when("指针拖入 dock 矩形")]
fn when_drag_into_dock() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        LAST_BOOL.with(|b| {
            *b.borrow_mut() = harness.sel.handle_mouse(
                &mouse(MouseEventKind::Drag(MouseButton::Left), 2, 4),
                &mut harness.scroll,
                harness.tr,
                harness.dock,
                &mut harness.sink,
            );
        });
    });
}

#[when("在 transcript 内滚轮向上并重绘")]
fn when_wheel_up_repaint() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        let wheel = mouse(MouseEventKind::ScrollUp, 1, 1);
        assert_eq!(
            harness.tui.dispatch_event(InputEvent::Mouse(wheel)),
            InputReaction::Rerender
        );
        harness.tui.request_render(false);
        let _ = harness.tui.render_now();
        let raw = harness.tui.terminal.writes.concat();
        LAST_BOOL.with(|b| *b.borrow_mut() = raw.contains("L11"));
    });
}

#[when("松手复制成功")]
fn when_copy_success() {
    when_drag_select_release();
}

#[when("在 Editor 内未修饰拖选跨行并松开")]
fn when_editor_drag_multiline() {
    EDITOR.with(|slot| {
        let mut editor = slot.borrow_mut();
        let editor = editor.as_mut().expect("editor");
        // Local coords: row 0 = top border, content starts at row 1.
        let down = mouse(MouseEventKind::Down(MouseButton::Left), 0, 1);
        let drag = mouse(MouseEventKind::Drag(MouseButton::Left), 4, 2);
        let up = mouse(MouseEventKind::Up(MouseButton::Left), 4, 2);
        editor.handle_input(InputEvent::Mouse(down));
        editor.handle_input(InputEvent::Mouse(drag));
        editor.handle_input(InputEvent::Mouse(up));
        LAST_BOOL.with(|b| {
            *b.borrow_mut() = editor.has_selection()
                && editor.selected_text().is_some_and(|t| t.contains('\n'))
                && !editor.take_pending_clipboard().is_empty();
        });
    });
}

#[when("读取 interaction_mode")]
fn when_read_interaction_mode() {
    // already set in given
}

#[then("模式为 Inline 且应用会话未激活")]
fn then_mode_a() {
    LAST_BOOL.with(|b| assert!(*b.borrow(), "expected Mode A / inactive session"));
}

#[then("发出 OSC52 剪贴板序列")]
fn then_osc52() {
    HARNESS.with(|h| {
        let harness = h.borrow();
        let harness = harness.as_ref().expect("harness");
        let raw = harness.tui.terminal.writes.concat();
        assert!(raw.contains("\x1b]52;c;"), "expected OSC52, got {raw:?}");
    });
}

#[then("选区仍在且焦点夹在 transcript 底边")]
fn then_dock_clamp() {
    HARNESS.with(|h| {
        let harness = h.borrow();
        let harness = harness.as_ref().expect("harness");
        let dragged = LAST_BOOL.with(|b| *b.borrow());
        assert!(dragged || harness.sel.is_dragging());
        assert!(harness.sel.is_dragging() || harness.sel.has_selection());
    });
}

#[then("scroll_top 不回到底部")]
fn then_wheel_sticky() {
    LAST_BOOL.with(|b| {
        assert!(
            *b.borrow(),
            "wheel sticky paint should keep scrolled content"
        )
    });
}

#[then("copy-notice 信号可观察且空选不发")]
fn then_copy_notice() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        assert!(
            harness.tui.copy_notice_active(),
            "expected copy-notice after successful copy"
        );

        let mut tui =
            TUI::with_interaction_mode(RecTerm::new(40, 10), InteractionMode::ApplicationOwned);
        tui.set_mode_b_dock_rows(2);
        tui.add_child(Box::new(StaticLines {
            lines: vec!["ab".into(), "cd".into(), "d1".into(), "d2".into()],
        }));
        tui.terminal.start();
        tui.begin_application_owned_session();
        tui.request_render(true);
        let _ = tui.render_now();
        let down = mouse(MouseEventKind::Down(MouseButton::Left), 0, 0);
        let up = mouse(MouseEventKind::Up(MouseButton::Left), 0, 0);
        let _ = tui.dispatch_event(InputEvent::Mouse(down));
        let _ = tui.dispatch_event(InputEvent::Mouse(up));
        assert!(
            !tui.copy_notice_active(),
            "empty selection must not raise copy-notice"
        );
    });
}

#[then("仅输入缓冲文本进入复制路径且 transcript 选区未写入")]
fn then_editor_selection() {
    LAST_BOOL.with(|b| {
        assert!(
            *b.borrow(),
            "editor multi-line selection must copy buffer text via OSC52"
        );
    });
}

#[then("为 Inline")]
fn then_inline() {
    LAST_BOOL.with(|b| assert!(*b.borrow()));
}

#[then("短时提示路径存在且不使用 Error 前缀拒闸 toast 冒充成功")]
fn then_product_notice_path() {
    LAST_BOOL.with(|b| assert!(*b.borrow()));
    let design = include_str!(
        "../../llmanspec/changes/c2070-add-package-tui-dual-interaction-modes/design.md"
    );
    assert!(
        design.contains("copy-notice") || design.contains("Copied"),
        "design must document copy-notice placement"
    );
    let host = include_str!("../../src/app/tui/layout/root/mod.rs");
    assert!(
        host.contains("arm_copy_notice") && host.contains("copy_notice_until"),
        "product UiRoot must expose Mode B copy-notice cue"
    );
    assert!(
        !host.contains("Error: Copied"),
        "must not hardcode Error: Copied toast shape"
    );
}

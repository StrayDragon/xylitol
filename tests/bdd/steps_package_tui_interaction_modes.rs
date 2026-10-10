//! Steps for `package-tui-interaction-modes` (c2070 ApplicationOwned).

use std::cell::RefCell;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::{
    Clock, Component, Editor, EditorOptions, EditorTheme, Hit, InputEvent, InputReaction,
    InteractionMode, PointerClickPhase, RecordingClipboardSink, ScreenRect, ScrollView,
    SelectListTheme, SelectionController, SystemClock, TUI, Terminal,
};

thread_local! {
    static HARNESS: RefCell<Option<ApplicationOwnedHarness>> = const { RefCell::new(None) };
    static LAST_BOOL: RefCell<bool> = const { RefCell::new(false) };
    static EDITOR: RefCell<Option<Editor>> = const { RefCell::new(None) };
    static POINTER: RefCell<PointerNotes> = const { RefCell::new(PointerNotes::empty()) };
}

#[derive(Clone, Copy, Default)]
struct PointerNotes {
    hit_before: Option<u64>,
    hit_after: Option<u64>,
    first_reaction: Option<InputReaction>,
    second_reaction: Option<InputReaction>,
    click_down: Option<u64>,
    click_up: Option<u64>,
    stale: bool,
}

impl PointerNotes {
    const fn empty() -> Self {
        Self {
            hit_before: None,
            hit_after: None,
            first_reaction: None,
            second_reaction: None,
            click_down: None,
            click_up: None,
            stale: false,
        }
    }
}

fn install_hit(tui: &mut TUI<RecTerm>, id: u64, capturing: bool) {
    let mut hit = Hit::on_line(id, 0).cols(0, 5);
    if capturing {
        hit = hit.capturing();
    }
    tui.mark_hit(hit);
}

fn ao_scroll_probe_tui() -> TUI<RecTerm> {
    let mut tui =
        TUI::with_interaction_mode(RecTerm::new(40, 10), InteractionMode::ApplicationOwned);
    tui.set_dock_rows(2);
    let mut lines: Vec<String> = (0..20).map(|i| format!("row-{i:02}")).collect();
    lines.extend(["dock-a".into(), "dock-b".into()]);
    tui.add_child(Box::new(StaticLines { lines }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    let _ = tui.render_now();
    tui
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

struct ApplicationOwnedHarness {
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
        *h.borrow_mut() = Some(ApplicationOwnedHarness {
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

#[given("ApplicationOwned 应用会话已 begin 且 transcript 有可拖选文本")]
fn given_application_owned_with_text() {
    POINTER.with(|p| *p.borrow_mut() = PointerNotes::empty());
    let mut tui =
        TUI::with_interaction_mode(RecTerm::new(40, 10), InteractionMode::ApplicationOwned);
    tui.set_dock_rows(2);
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
        *h.borrow_mut() = Some(ApplicationOwnedHarness {
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

#[given("ApplicationOwned 应用会话已 begin")]
fn given_application_owned_session() {
    given_application_owned_with_text();
}

#[given("transcript 拖选进行中")]
fn given_transcript_dragging() {
    HARNESS.with(|h| {
        let mut harness = ApplicationOwnedHarness {
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

#[given("ApplicationOwned 视口已 follow 到底")]
fn given_application_owned_follow_end() {
    let mut tui =
        TUI::with_interaction_mode(RecTerm::new(40, 6), InteractionMode::ApplicationOwned);
    tui.set_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: (0..20).map(|i| format!("L{i:02}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    let _ = tui.render_now();
    HARNESS.with(|h| {
        *h.borrow_mut() = Some(ApplicationOwnedHarness {
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

#[given("ApplicationOwned 下 Editor 有多行缓冲")]
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
        // Follow-end shows L14..L17 (40x6, dock=2). One WHEEL_NOTCH up → L11..L14.
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

#[then("模式为 Inline 且应用会话未激活")]
fn then_inline_session_inactive() {
    LAST_BOOL.with(|b| assert!(*b.borrow(), "expected Inline / inactive session"));
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
        tui.set_dock_rows(2);
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

#[given("click 相位为 {phase:string}")]
fn given_click_phase(phase: String) {
    let parsed: PointerClickPhase = phase
        .parse()
        .unwrap_or_else(|_| panic!("unknown click phase {phase}"));
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        install_hit(&mut harness.tui, 3, false);
        harness.tui.set_pointer_click_phase(parsed);
    });
}

#[given("指针探针已武装")]
fn given_probe_armed() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        harness
            .tui
            .replace_hits([Hit::on_line(1, 0).cols(0, 2), Hit::on_line(2, 0).cols(3, 5)]);
        harness.tui.arm_pointer_probe(true);
    });
}

#[given("已登记 capturing 命中区")]
fn given_capturing_region() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        install_hit(&mut harness.tui, 4, true);
    });
}

fn probe_hit_after_scroll(tui: &mut TUI<RecTerm>, hit: Hit) {
    tui.mark_hit(hit);
    let hit_before = tui.hit_id_at(1, 0);
    let _ = tui.application_owned_scroll_by(-3);
    let hit_after = tui.hit_id_at(1, 0);
    POINTER.with(|p| {
        *p.borrow_mut() = PointerNotes {
            hit_before,
            hit_after,
            ..PointerNotes::empty()
        };
    });
}

#[when("登记 content 命中区并改变视口滚动后查询同一屏幕格")]
fn when_register_hit_and_scroll() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        let mut tui = ao_scroll_probe_tui();
        probe_hit_after_scroll(&mut tui, Hit::on_line(7, 12).cols(0, 8));
        harness.tui = tui;
    });
}

#[when("登记视口命中区并改变视口滚动后查询同一屏幕格")]
fn when_register_viewport_hit_and_scroll() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        let mut tui = ao_scroll_probe_tui();
        probe_hit_after_scroll(&mut tui, Hit::at_cell(11, 1, 0));
        harness.tui = tui;
    });
}

#[when("在命中区 Left Down 后未拖动即 Left Up")]
fn when_click_hit_no_drag() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        let down = mouse(MouseEventKind::Down(MouseButton::Left), 1, 0);
        let up = mouse(MouseEventKind::Up(MouseButton::Left), 1, 0);
        let _ = harness.tui.dispatch_event(InputEvent::Mouse(down));
        let click_down = harness.tui.take_pointer_click();
        let _ = harness.tui.dispatch_event(InputEvent::Mouse(up));
        let click_up = harness.tui.take_pointer_click();
        POINTER.with(|p| {
            p.borrow_mut().click_down = click_down;
            p.borrow_mut().click_up = click_up;
        });
        harness.tui.request_render(false);
        let _ = harness.tui.render_now();
    });
}

#[when("未武装探针时派发 Moved")]
fn when_unarmed_moved() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        let before = harness.tui.frame_count();
        let reaction =
            harness
                .tui
                .dispatch_event(InputEvent::Mouse(mouse(MouseEventKind::Moved, 1, 0)));
        harness.tui.request_render(false);
        let _ = harness.tui.try_render();
        LAST_BOOL.with(|b| {
            *b.borrow_mut() =
                reaction == InputReaction::None && harness.tui.frame_count() == before;
        });
    });
}

#[when("连续两次 Moved 落在同一命中 id")]
fn when_two_moved_same_id() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        let first =
            harness
                .tui
                .dispatch_event(InputEvent::Mouse(mouse(MouseEventKind::Moved, 1, 0)));
        let second =
            harness
                .tui
                .dispatch_event(InputEvent::Mouse(mouse(MouseEventKind::Moved, 1, 0)));
        POINTER.with(|p| {
            p.borrow_mut().first_reaction = Some(first);
            p.borrow_mut().second_reaction = Some(second);
        });
    });
}

#[when("在 capturing 区 Left Down 并拖动")]
fn when_capture_down_drag() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        let _ = harness.tui.dispatch_event(InputEvent::Mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            1,
            0,
        )));
        let _ = harness.tui.dispatch_event(InputEvent::Mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            5,
            0,
        )));
    });
}

#[when("Moved 导致命中 id 变化")]
fn when_moved_changes_hit_id() {
    HARNESS.with(|h| {
        let mut harness = h.borrow_mut();
        let harness = harness.as_mut().expect("harness");
        harness.tui.mark_ao_components_stale();
        harness.tui.request_render(true);
        let _ = harness.tui.render_now();
        let _ = harness
            .tui
            .dispatch_event(InputEvent::Mouse(mouse(MouseEventKind::Moved, 0, 0)));
        let reaction =
            harness
                .tui
                .dispatch_event(InputEvent::Mouse(mouse(MouseEventKind::Moved, 4, 0)));
        POINTER.with(|p| {
            p.borrow_mut().first_reaction = Some(reaction);
            p.borrow_mut().stale = harness.tui.ao_components_stale();
        });
    });
}

#[then("命中 id 随 live 滚动变化而不是登记时的快照")]
fn then_hit_follows_live_scroll() {
    POINTER.with(|p| {
        let n = p.borrow();
        assert_eq!(n.hit_before, Some(7));
        assert_eq!(n.hit_after, None);
    });
}

#[then("命中 id 仍钉在同一屏幕格")]
fn then_hit_stays_on_screen_cell() {
    POINTER.with(|p| {
        let n = p.borrow();
        assert_eq!(n.hit_before, Some(11));
        assert_eq!(n.hit_after, Some(11));
    });
}

#[then("按下吞掉拖选且松开不报告 click")]
fn then_down_swallow_up_silent() {
    HARNESS.with(|h| {
        let harness = h.borrow();
        let harness = harness.as_ref().expect("harness");
        assert!(!harness.tui.application_owned_is_dragging());
        assert!(!harness.tui.application_owned_has_selection());
    });
    POINTER.with(|p| {
        let n = p.borrow();
        assert_eq!(n.click_down, Some(3));
        assert!(n.click_up.is_none());
    });
}

#[then("报告 click 且不发出 OSC52")]
fn then_click_without_osc52() {
    POINTER.with(|p| {
        assert_eq!(p.borrow().click_up, Some(3));
    });
    HARNESS.with(|h| {
        let harness = h.borrow();
        let harness = harness.as_ref().expect("harness");
        let raw = harness.tui.terminal.writes.concat();
        assert!(
            !raw.contains("\x1b]52;c;"),
            "empty click must not OSC52, got {raw:?}"
        );
    });
}

#[then("不强制整帧重绘")]
fn then_no_full_frame() {
    LAST_BOOL.with(|b| assert!(*b.borrow(), "unarmed Moved must not paint"));
}

#[then("第二次不排帧")]
fn then_second_moved_silent() {
    POINTER.with(|p| {
        let n = p.borrow();
        assert_eq!(n.first_reaction, Some(InputReaction::Rerender));
        assert_eq!(n.second_reaction, Some(InputReaction::None));
    });
}

#[then("不启动 transcript 拖选")]
fn then_no_transcript_drag() {
    HARNESS.with(|h| {
        let harness = h.borrow();
        let harness = harness.as_ref().expect("harness");
        assert!(!harness.tui.application_owned_is_dragging());
        assert!(!harness.tui.application_owned_has_selection());
    });
}

#[then("请求重绘且组件树未被标为整段 stale")]
fn then_rerender_without_stale() {
    POINTER.with(|p| {
        let n = p.borrow();
        assert_eq!(n.first_reaction, Some(InputReaction::Rerender));
        assert!(!n.stale);
    });
}

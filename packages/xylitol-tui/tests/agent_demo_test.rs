mod support;

#[path = "../examples/agent_demo.rs"]
mod agent_demo_example;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use agent_demo_example::FakeCodingAgentApp;
use support::TuiTestHarness;

#[test]
fn agent_demo_submit_flow_stays_within_width_budget() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.keys("\r");
    h.render_result()
        .expect("submitting edited prompt must not overflow width");
    h.assert_text_contains("Thinking");
}

#[test]
fn agent_demo_submit_flow_streams_reply_after_ticks() {
    let mut h = TuiTestHarness::new(172, 48);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.keys("\r");
    let mut saw_reply = false;
    let mut saw_rg = false;
    for _ in 0..1200 {
        h.tick();
        h.render_result()
            .expect("streaming scripted turn must stay within width budget");
        let text = h.tui.terminal.viewport().join("\n");
        if text.contains("rg -n") {
            saw_rg = true;
        }
        if text.contains("收到，我已经接住") {
            saw_reply = true;
        }
        if saw_rg && saw_reply {
            break;
        }
    }
    assert!(
        saw_rg,
        "expected tool line with rg -n in viewport during turn"
    );
    assert!(
        saw_reply,
        "expected streamed assistant reply to appear after thinking typewriter"
    );
}

#[test]
fn agent_demo_cjk_emoji_submit_flow_stays_within_width_budget() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "修复 footer 宽度预算并补一个 emoji smoke 🙂",
    )))
    .focus(Some(0));

    h.render_result()
        .expect("initial CJK render should succeed");
    h.keys("\r");
    h.render_result()
        .expect("submitting CJK/emoji prompt must not overflow width");
    h.assert_text_contains("修复 footer");
}

#[test]
fn agent_demo_idle_ticks_drive_script_without_input() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.tick();
    h.render_result()
        .expect("first idle tick should render scripted status");
    h.assert_text_contains("Running acceptance harness");

    h.tick();
    h.render_result()
        .expect("second idle tick should render scripted tool activity");
    h.assert_text_contains("cargo test -p xylitol-tui --test agent_demo_test");
}

#[test]
fn agent_demo_slash_command_popup_filters_and_completes() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));

    h.render_result()
        .expect("initial empty editor should render");
    // Open CommandPopup by typing `/` on the first line.
    h.keys("/");
    h.render_result()
        .expect("slash popup must stay within width budget");
    h.assert_text_contains("help");
    h.assert_text_contains("Show key help in transcript");
    // "palette" may be below SelectList max_visible; assert via filter instead.

    // Prefix filter: `/hel` should keep help, drop unrelated commands.
    h.keys("hel");
    h.render_result()
        .expect("filtered slash popup must stay within width");
    let filtered = h.tui.terminal.viewport().join("\n");
    assert!(
        filtered.contains("help") && filtered.contains("Show key help in transcript"),
        "filtered popup should still show /help; got:\n{filtered}"
    );
    assert!(
        !filtered.contains("Inject unified"),
        "prefix /hel should hide /diff; got:\n{filtered}"
    );

    // Tab completes into the editor (does not submit); user submits with Enter later.
    h.keys("\t");
    h.render_result()
        .expect("slash completion into editor must stay within width");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("/help"),
        "Tab should complete the slash command into the editor; got:\n{after}"
    );
    assert!(
        !after.contains("Show this help"),
        "popup should close after completion; got:\n{after}"
    );
}

#[test]
fn agent_demo_slash_command_popup_backspace_to_slash_closes() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));

    h.render_result()
        .expect("initial empty editor should render");
    h.keys("/h");
    h.render_result().expect("slash popup with filter");
    h.assert_text_contains("Show key help in transcript");
    h.keys("\x7f"); // Backspace → `/` only → close popup, keep `/`
    h.render_result()
        .expect("backspace to lone slash must stay within width");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        !text.contains("Show key help in transcript"),
        "backspacing to lone `/` should close CommandPopup; got:\n{text}"
    );
}

#[test]
fn agent_demo_slash_command_popup_esc_dismisses() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));

    h.render_result()
        .expect("initial empty editor should render");
    h.keys("/");
    h.render_result().expect("slash popup open");
    h.assert_text_contains("Switch execution model");
    h.keys("\x1b"); // Esc → Editor cancels autocomplete
    h.render_result().expect("Esc dismisses slash popup");
    let dismissed = h.tui.terminal.viewport().join("\n");
    assert!(
        !dismissed.contains("Switch execution model"),
        "Esc should close CommandPopup and leave `/` in the editor; got:\n{dismissed}"
    );
}

#[test]
fn agent_demo_at_path_popup_lists_and_completes() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("demo_note.txt"), "hi").expect("write file");
    std::fs::create_dir(dir.path().join("demo_subdir")).expect("mkdir");

    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt_at(
        Arc::new(AtomicBool::new(false)),
        "",
        dir.path().to_path_buf(),
    )))
    .focus(Some(0));

    h.render_result()
        .expect("initial empty editor should render");
    h.keys("@");
    h.render_result()
        .expect("@ path popup must stay within width budget");
    let open = h.tui.terminal.viewport().join("\n");
    assert!(
        open.contains("demo_note.txt") || open.contains("demo_subdir"),
        "@ should list cwd entries; got:\n{open}"
    );

    h.keys("demo_n");
    h.render_result()
        .expect("filtered @ path popup must stay within width");
    h.assert_text_contains("demo_note.txt");

    h.keys("\t");
    h.render_result()
        .expect("@ path completion into editor must stay within width");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("@demo_note.txt"),
        "Tab should complete @path into the editor; got:\n{after}"
    );
}

#[test]
fn agent_demo_at_path_popup_esc_dismisses() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("esc_target.rs"), "").expect("write file");

    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt_at(
        Arc::new(AtomicBool::new(false)),
        "",
        dir.path().to_path_buf(),
    )))
    .focus(Some(0));

    h.render_result()
        .expect("initial empty editor should render");
    h.keys("@");
    h.render_result().expect("@ path popup open");
    h.assert_text_contains("esc_target.rs");
    h.keys("\x1b");
    h.render_result().expect("Esc dismisses @ path popup");
    let dismissed = h.tui.terminal.viewport().join("\n");
    assert!(
        !dismissed.contains("esc_target.rs"),
        "Esc should close @ path popup (file name only lived in SelectList); got:\n{dismissed}"
    );
}

#[test]
fn agent_demo_command_palette_replaces_editor_slot() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.keys("\x10");
    h.render_result()
        .expect("command plate selector must stay within width budget");
    h.assert_text_contains("Command Plate");
    h.assert_text_contains("Markdown full grammar (stream)");
    h.assert_text_contains("Diff unified + side-by-side");
    let text = h.tui.terminal.viewport().join("\n");
    let palette_pos = text
        .find("Command Plate")
        .expect("plate title should be visible");
    let footer_pos = text
        .find("esc close")
        .expect("selector footer hint should remain below");
    assert!(
        palette_pos < footer_pos,
        "plate must sit in the editor slot above the footer; got:\n{text}"
    );
}

#[test]
fn agent_demo_settings_replaces_editor_slot() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.keys("\x13");
    h.render_result()
        .expect("settings selector must stay within width budget");
    h.assert_text_contains("Session Settings");
    h.assert_text_contains("Approval");
    let text = h.tui.terminal.viewport().join("\n");
    let settings_pos = text
        .find("Session Settings")
        .expect("settings title should be visible");
    let footer_pos = text
        .find("esc close")
        .expect("selector footer hint should remain below");
    assert!(
        settings_pos < footer_pos,
        "settings must sit in the editor slot above the footer; got:\n{text}"
    );
}

#[test]
fn agent_demo_selector_stays_visible_after_long_transcript() {
    let mut h = TuiTestHarness::new(80, 24);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    // Grow transcript past the viewport via scripted ticks, then open palette.
    h.render_result().expect("initial render should succeed");
    h.keys("\r");
    for _ in 0..1200 {
        h.tick();
        h.render_result()
            .expect("streaming must stay within width budget");
        let text = h.tui.terminal.viewport().join("\n");
        if text.contains("收到，我已经接住") {
            break;
        }
    }
    h.keys("\x10");
    h.render_result()
        .expect("palette after long transcript must stay within width");
    // Viewport is only the last 24 rows; absolute-top blit would hide the
    // palette. Editor-slot replacement keeps it in the visible bottom.
    h.assert_text_contains("Command Plate");
    h.assert_text_contains("Markdown full grammar (stream)");
}

#[test]
fn agent_demo_narrow_cjk_submit_flow_stays_within_width_budget() {
    let mut h = TuiTestHarness::new(96, 32);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "把命令面板和设置面板的窄宽 CJK 回归补齐 🙂",
    )))
    .focus(Some(0));

    h.render_result()
        .expect("initial narrow CJK render should succeed");
    h.keys("\r");
    h.render_result()
        .expect("submitting narrow CJK/emoji prompt must not overflow width");
    h.assert_text_contains("窄宽 CJK");
}

#[test]
fn agent_demo_idle_omits_status_row() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        !text.contains("Ready  |  last:"),
        "idle must not show a busy-style status chrome; got:\n{text}"
    );
    for prefix in ["- Ready", "\\ Ready", "| Ready", "/ Ready"] {
        assert!(
            !text.contains(prefix),
            "idle must hide spinner frame {prefix:?}; got:\n{text}"
        );
    }
    assert!(
        text.contains("~/xylitol") && text.contains("sonnet-4"),
        "minimal footer should remain; got:\n{text}"
    );
}

#[test]
fn agent_demo_default_hides_hardware_cursor_during_stream() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.keys("\r");
    for _ in 0..80 {
        h.tick();
        h.render_result()
            .expect("streaming frames must stay within width budget");
    }
    assert!(
        !h.tui.terminal.cursor_visible(),
        "default show_hardware_cursor=false must leave hardware cursor hidden (pi parity)"
    );
    assert_eq!(
        h.tui.terminal.show_cursor_calls(),
        0,
        "streaming redraws must not call show_cursor when hardware cursor is off"
    );
    assert!(
        h.tui.terminal.hide_cursor_calls() > 0,
        "engine should hide the hardware cursor after positioning for IME"
    );
}

#[test]
fn agent_demo_layout_is_minimal_single_column() {
    // Tall enough that seed (all blocks expanded) still shows the top of transcript.
    let mut h = TuiTestHarness::new(172, 80);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        !text.contains("dbg") && !text.contains("Workspace") && !text.contains("Conversation"),
        "no debug strip / sidebar / section title chrome; got:\n{text}"
    );
    assert!(
        text.contains("Collapse examples into one fake coding-agent demo"),
        "full transcript must render into the line-array for scrollback; got:\n{text}"
    );
    assert!(
        text.contains("❯"),
        "user messages use a short glyph prefix; got:\n{text}"
    );
    assert!(
        text.contains("Ctrl+P") && text.contains("plate") && text.contains("/md"),
        "slim seed should point at plate/md without a permanent chrome wall; got:\n{text}"
    );
    assert!(
        text.contains("thinking"),
        "compact kit seed still includes a thinking block; got:\n{text}"
    );
}

#[test]
fn agent_demo_thinking_typewriter_expands_then_collapses() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.keys("\r");

    let mut saw_streaming_body = false;
    for _ in 0..120 {
        h.tick();
        h.render_result()
            .expect("thinking typewriter frames must stay within width");
        let text = h.tui.terminal.viewport().join("\n");
        if text.contains("User asked:") || text.contains("hesitating on width") {
            saw_streaming_body = true;
            break;
        }
    }
    assert!(
        saw_streaming_body,
        "thinking should typewriter with body visible while streaming"
    );

    // Mid-stream ^T must stick even as more chunks arrive.
    h.keys("\x14"); // Ctrl+T → collapse (any_collapsed path)
    let mut stayed_collapsed = true;
    for _ in 0..40 {
        h.tick();
        h.render_result()
            .expect("collapsed thinking stream must stay within width");
        let text = h.tui.terminal.viewport().join("\n");
        // While still in Thinking status, body of the *active* stream should stay hidden.
        if text.contains("Drafting reply") || text.contains("收到，我已经接住") {
            break;
        }
        if text.contains("User asked:") || text.contains("hesitating on width") {
            stayed_collapsed = false;
            break;
        }
    }
    assert!(
        stayed_collapsed,
        "Ctrl+T during thinking stream must not be overridden by later chunks"
    );

    // Finish the turn; thinking should collapse again.
    for _ in 0..400 {
        h.tick();
        h.render_result()
            .expect("remainder of turn must stay within width");
    }
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("收到，我已经接住") || after.contains("rg -n"),
        "turn should complete; got:\n{after}"
    );
}

#[test]
fn agent_demo_ctrl_t_toggles_thinking_block() {
    let mut h = TuiTestHarness::new(172, 80);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    let before = h.tui.terminal.viewport().join("\n");
    assert!(
        before.contains("thinking"),
        "thinking starts present; got:\n{before}"
    );
    assert!(
        before.contains("Keep transcript in scrollback"),
        "seed thinking starts expanded; got:\n{before}"
    );

    h.keys("\x14"); // Ctrl+T collapses when any thinking is open
    h.render_result()
        .expect("collapse thinking must stay within width");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        !after.contains("Keep transcript in scrollback"),
        "Ctrl+T should collapse thinking body; got:\n{after}"
    );
}

#[test]
fn agent_demo_alt_e_toggles_tool_block() {
    let mut h = TuiTestHarness::new(172, 80);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    let before = h.tui.terminal.viewport().join("\n");
    assert!(
        before.contains("agent_demo.rs"),
        "seed tool block present; got:\n{before}"
    );
    assert!(
        before.contains("opened agent_demo") || before.contains("FakeCodingAgentApp"),
        "seed tool starts expanded; got:\n{before}"
    );

    h.keys("\x1be"); // Alt+E collapses all tool/diff when any are open
    h.render_result()
        .expect("collapse tools must stay within width");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        !after.contains("opened agent_demo") && !after.contains("FakeCodingAgentApp"),
        "Alt+E should collapse tool detail; got:\n{after}"
    );
}

#[test]
fn agent_demo_alt_g_cycles_glyph_set_to_ascii() {
    let mut h = TuiTestHarness::new(172, 80);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.assert_text_contains("❯");
    h.keys("\x1bg"); // Alt+G (glyphs; Ctrl+G is external editor)
    h.render_result()
        .expect("glyph cycle must stay within width");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("ascii") && text.contains("~/xylitol") && text.contains("sonnet-4"),
        "Alt+G should switch glyph set and keep slim footer metadata; got:\n{text}"
    );
    assert!(
        text.contains("glyph_set=ascii"),
        "system note should confirm the switch; got:\n{text}"
    );
}

#[test]
fn agent_demo_ctrl_c_clears_editor_then_quits() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::Ordering;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let quit = Arc::new(AtomicBool::new(false));
    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        quit.clone(),
        "keep me",
    )));
    let mut h = TuiTestHarness::new(100, 24);
    FakeCodingAgentApp::install_input_listeners(&app, &mut h.tui);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));

    h.keys("\x03"); // Ctrl+C
    assert!(
        !quit.load(Ordering::SeqCst),
        "non-empty editor must clear, not quit"
    );
    assert!(
        app.borrow().input_text_for_test().is_empty(),
        "Ctrl+C should clear editor text"
    );

    h.keys("\x03");
    assert!(
        quit.load(Ordering::SeqCst),
        "second Ctrl+C on empty editor should quit"
    );
}

#[test]
fn agent_demo_escape_aborts_active_stream() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render");
    h.keys("\r");
    h.render_result().expect("after submit");
    // Esc while scripted turn is active
    h.keys("\x1b");
    h.render_result().expect("after abort");
    h.assert_text_contains("stream aborted");
}

#[cfg(feature = "highlight")]
#[test]
fn agent_demo_seed_rust_fence_is_highlighted() {
    let mut h = TuiTestHarness::new(120, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");
    // Prefer short stream-rust plate item over full md-full typewriter.
    h.keys("\x10stream-rust\r");
    let mut saw = false;
    for i in 0..4000 {
        h.tick();
        if i % 4 == 0 {
            h.render_result().ok();
        }
        let text = h.tui.terminal.scroll_buffer().join("\n");
        if text.contains("fn accept") || text.contains("println") {
            saw = true;
            break;
        }
    }
    assert!(saw, "streamed rust highlight demo should appear");
    let raw = h.tui.terminal.all_writes();
    assert!(
        raw.contains('\u{1b}') || raw.contains("\x1b["),
        "highlighted code block should emit ANSI under feature highlight"
    );
}

#[cfg(feature = "highlight")]
#[test]
fn agent_demo_streaming_fence_emits_ansi_when_closed() {
    let mut h = TuiTestHarness::new(120, 48);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));
    h.render_result().expect("initial render");
    // Clear seed writes; submit to start scripted turn with streamed code block.
    h.tui.terminal.clear_writes();
    h.keys("\r");
    let mut saw_accept_fn = false;
    for _ in 0..2000 {
        h.tick();
        h.render_result().ok();
        let text = h.tui.terminal.viewport().join("\n");
        if text.contains("fn accept") {
            saw_accept_fn = true;
            break;
        }
    }
    assert!(saw_accept_fn, "streamed assistant code block should appear");
    let raw = h.tui.terminal.all_writes();
    assert!(
        raw.contains('\u{1b}') || raw.contains("\x1b["),
        "closed streamed code block should highlight with ANSI"
    );
}

#[test]
fn agent_demo_seed_markdown_showcase_c530() {
    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");
    // Slash popup steals Enter for selection — Tab completes, then Enter submits.
    h.keys("/md\t\r");
    let mut text = String::new();
    for i in 0..8000 {
        h.tick();
        // Render every N ticks — irregular stream is long; full render each tick is costly.
        if i % 8 == 0 {
            h.render_result().ok();
        }
        text = h.tui.terminal.scroll_buffer().join("\n");
        if text.contains("cara")
            && text.contains("docs (https://example.com/md)")
            && text.contains("（加粗）")
        {
            h.render_result().ok();
            break;
        }
    }
    assert!(
        text.contains("Markdown grammar stub"),
        "showcase title:\n{text}"
    );
    assert!(
        text.contains("docs (https://example.com/md)"),
        "links must render as text (url):\n{text}"
    );
    assert!(
        text.contains("diagram (https://example.com/a.png)"),
        "images must render as alt (url):\n{text}"
    );
    assert!(
        text.contains("（加粗）")
            && text.contains("`inline code`")
            && text.contains("~~strikethrough~~"),
        "semantic bold label + code/strike markers must be visible:\n{text}"
    );
    assert!(
        text.contains("（斜体）"),
        "semantic italic label must be visible:\n{text}"
    );
    assert!(
        !text.contains("**（加粗）") && !text.contains("*（斜体）"),
        "bold/italic must not keep star markers:\n{text}"
    );
    assert!(
        text.contains("alice") && text.contains("eng"),
        "table cells should appear:\n{text}"
    );
    assert!(
        text.contains("- [ ] 未完成") && text.contains("- [x] 已完成"),
        "task list checkboxes must stay inline:\n{text}"
    );
    assert!(
        text.contains("2. 有序二项")
            && text.contains("嵌套无序")
            && text.contains("1. 再嵌套有序")
            && text.contains("链接 (https://example.com/list)"),
        "nested lists + ordered link item must not flatten:\n{text}"
    );
    assert!(
        !text.contains("有序二项嵌套无序") && !text.lines().any(|l| l.trim() == "链接"),
        "list nesting/link must not mash or isolate label:\n{text}"
    );
    assert!(
        !text.contains('┌') && !text.lines().any(|l| l.contains("```")),
        "no box-drawing table or fence chrome:\n{text}"
    );
    assert!(
        !text
            .lines()
            .any(|l| l.trim_start().starts_with("# Markdown")),
        "rendered heading must not keep # prefix:\n{text}"
    );
}

#[test]
fn agent_demo_slash_md_injects_showcase() {
    let mut h = TuiTestHarness::new(120, 48);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.keys("/md\t\r");
    let mut text = String::new();
    let mut saw_accent = false;
    let mut saw_warning = false;
    for i in 0..8000 {
        h.tick();
        if i % 8 == 0 {
            h.render_result().ok();
        }
        text = h.tui.terminal.scroll_buffer().join("\n");
        saw_accent = scrollback_has_fg_rgb(&h, (137, 180, 250));
        saw_warning = scrollback_has_fg_rgb(&h, (249, 226, 175));
        if text.contains("Markdown grammar stub")
            && text.contains("（加粗）")
            && text.contains("（斜体）")
            && saw_accent
            && saw_warning
        {
            h.render_result().ok();
            break;
        }
    }
    assert!(
        text.contains("request Markdown showcase"),
        "/md should inject showcase request line:\n{text}"
    );
    assert!(
        text.contains("Markdown grammar stub"),
        "/md should stream showcase body:\n{text}"
    );
    // B color enhance: bold → accent RGB, italic → warning RGB (cell attrs; scroll text strips ANSI).
    assert!(
        saw_accent || scrollback_has_fg_rgb(&h, (137, 180, 250)),
        "/md bold/heading should paint DESIGN accent RGB on cells"
    );
    assert!(
        saw_warning || scrollback_has_fg_rgb(&h, (249, 226, 175)),
        "/md italic should paint DESIGN warning RGB on cells"
    );
}

#[test]
fn agent_demo_collapsible_headers_show_key_hints() {
    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.assert_text_contains("(Ctrl+T)");
    h.assert_text_contains("(Alt+E)");
}

#[test]
fn agent_demo_simulated_edit_tool_pops_expanded_diff() {
    let mut h = TuiTestHarness::new(120, 48);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.keys("\r"); // submit seed prompt → scripted turn includes Edit
    let mut saw_edit = false;
    for _ in 0..3000 {
        h.tick();
        h.render_result().ok();
        let text = h.tui.terminal.viewport().join("\n");
        if text.contains("edit src/app/tui/ui_root.rs")
            && (text.contains("footer_note") || text.contains("ctx"))
        {
            saw_edit = true;
            break;
        }
    }
    assert!(
        saw_edit,
        "simulated Edit tool should pop an expanded edit-format Diff"
    );
}

#[test]
fn agent_demo_seed_shows_unified_and_side_by_side_diffs() {
    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.assert_text_contains("unified");
    h.assert_text_contains("side-by-side");
    h.assert_text_contains("edit-format");

    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("ready") || text.contains("prompt"),
        "expanded unified diff body should be visible; got:\n{text}"
    );
    assert!(
        text.contains("Working") || text.contains("status:"),
        "expanded side-by-side diff body should be visible; got:\n{text}"
    );
}

#[test]
fn agent_demo_plate_diff_shows_c540_cjk_and_empty_half() {
    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.keys("\x10diff\r");
    h.render_result().expect("after diff-sbs plate");
    let text = h.tui.terminal.scroll_buffer().join("\n");
    assert!(
        text.contains("c540")
            || text.contains("CJK")
            || text.contains("验收")
            || text.contains("发布"),
        "plate diff-sbs should surface c540 CJK sample; got:\n{text}"
    );
    assert!(
        text.contains("only_old") || text.contains("empty half") || text.contains("orphan"),
        "plate diff-sbs should surface empty-half sample; got:\n{text}"
    );
}

#[test]
fn agent_demo_dollar_stub_source_opens_and_plate_mentions_c545() {
    let mut h = TuiTestHarness::new(100, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");

    h.keys("\x10completion\r");
    h.render_result().expect("after completion-dollar plate");
    let tip = h.tui.terminal.scroll_buffer().join("\n");
    assert!(
        tip.contains("c545") || tip.contains("inline") || tip.contains("$skill"),
        "plate completion-dollar should mention c545 inline $skill; got:\n{tip}"
    );

    // Mid-prompt `$` (like `@`), not line-leading only.
    h.keys("use $");
    h.render_result()
        .expect("inline dollar stub popup must stay within width");
    let open = h.tui.terminal.viewport().join("\n");
    assert!(
        open.contains("demo") || open.contains("c545 stub"),
        "mid-line $ should open demo dollar stub popup; got:\n{open}"
    );
    for line in h.tui.terminal.viewport() {
        assert!(
            xylitol_tui::visible_width(&line) <= 100,
            "dollar popup overflow; line={line:?}"
        );
    }

    h.keys("dem\t");
    h.render_result().expect("Tab applies inline $demo");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("use $demo"),
        "Tab should preserve leading text; got:\n{after}"
    );
}

#[test]
fn agent_demo_plate_expandable_head_shows_c550_more_hint() {
    let mut h = TuiTestHarness::new(120, 60);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.keys("\x10expandable\r");
    h.render_result().expect("after expandable-head plate");
    let text = h.tui.terminal.scroll_buffer().join("\n");
    assert!(
        text.contains("c550") && text.contains("more lines"),
        "plate expandable-head should surface c550 Head tip; got:\n{text}"
    );
    // Isolate the Read tool block (seed may still contain Tail "earlier lines").
    let read_idx = text
        .find("Read expandable_output.rs")
        .expect("Read tool header");
    let read_block = &text[read_idx..];
    assert!(
        read_block.contains("more lines") && read_block.contains("ctrl+o to expand"),
        "collapsed Head viewport must show more-lines hint below body; got:\n{read_block}"
    );
    assert!(
        read_block.contains("render_expandable_output") || read_block.contains("Head keeps"),
        "Head preview should keep file head visible; got:\n{read_block}"
    );
    let hint_at = read_block
        .find("more lines")
        .expect("more lines hint in Read block");
    let head_at = read_block
        .find("expandable_output.rs")
        .or_else(|| read_block.find("render_expandable_output"))
        .expect("head content");
    assert!(
        head_at < hint_at,
        "Head body must appear above more-lines hint; got:\n{read_block}"
    );
}

#[test]
fn agent_demo_plate_playground_sync_mentions_c555() {
    let mut h = TuiTestHarness::new(100, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.keys("\x10playground\r");
    h.render_result().expect("after playground-sync plate");
    let text = h.tui.terminal.scroll_buffer().join("\n");
    assert!(
        text.contains("c555") && text.contains("sync_tokens"),
        "plate playground-sync should mention c555 sync; got:\n{text}"
    );
    assert!(
        text.contains("playground") && (text.contains("ignore") || text.contains("忽略")),
        "tip should note Agent ignores playground; got:\n{text}"
    );
}

#[test]
fn agent_demo_plate_tree_mentions_c560_and_empty_search() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let mut h = TuiTestHarness::new(100, 40);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.keys("\x10tree\r");
    h.render_result().expect("after tree plate");
    let tip = h.tui.terminal.scroll_buffer().join("\n");
    assert!(
        tip.contains("c560") && tip.contains("no-match"),
        "plate tree should inject c560 empty/selection tip; got:\n{tip}"
    );

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();
    app.borrow_mut().open_session_tree_for_test();
    let mut h2 = TuiTestHarness::new(100, 28);
    h2.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h2.render_result().expect("tree open");
    h2.keys("zzz-nomatch-c560");
    h2.render_result().expect("after no-match search");
    let empty = h2.tui.terminal.viewport().join("\n");
    assert!(
        empty.contains("No entries found"),
        "tree search with no hits must show empty hint; got:\n{empty}"
    );
}

#[test]
fn agent_demo_plate_md_list_wrap_streams_nested_lists() {
    let mut h = TuiTestHarness::new(100, 48);
    h.mount(Box::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )))
    .focus(Some(0));
    h.render_result().expect("initial render");
    h.keys("\x10md-list\r");
    let mut text = String::new();
    for i in 0..4000 {
        h.tick();
        if i % 8 == 0 {
            h.render_result().ok();
        }
        text = h.tui.terminal.scroll_buffer().join("\n");
        if text.contains("再嵌套有序") && text.contains("example.com/list") {
            break;
        }
    }
    assert!(
        text.contains("prewrapped") || text.contains("悬挂"),
        "plate tip should explain list wrap; got:\n{text}"
    );
    assert!(
        text.contains("嵌套无序") && text.contains("再嵌套有序"),
        "streamed nested list body missing; got:\n{text}"
    );
    assert!(
        !text.contains("有序二项嵌套无序"),
        "nested list must not flatten into parent; got:\n{text}"
    );
}

#[test]
fn agent_demo_plate_narrow_clamp_opens_settings_search() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();
    let mut h = TuiTestHarness::new(100, 36);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("initial");
    h.keys("\x10narrow\r");
    h.render_result().expect("after narrow-clamp plate");
    let tip = h.tui.terminal.scroll_buffer().join("\n");
    assert!(
        tip.contains("Library reference") && tip.contains("clamp"),
        "narrow-clamp tip missing; got:\n{tip}"
    );
    let viewport = h.tui.terminal.viewport().join("\n");
    assert!(
        viewport.contains("Model") || viewport.contains("Settings") || viewport.contains("search"),
        "settings slot should open for live empty-match demo; got:\n{viewport}"
    );
    h.keys("zzz");
    h.render_result().expect("after search miss");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("No matching"),
        "settings search miss should show empty hint; got:\n{after}"
    );
}

#[test]
fn agent_demo_command_plate_echoes_filter() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();
    let mut h = TuiTestHarness::new(100, 36);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("initial");
    h.keys("\x10trunc");
    h.render_result().expect("filtered plate");
    let viewport = h.tui.terminal.viewport().join("\n");
    assert!(
        viewport.contains("Command Plate") && viewport.contains("> trunc"),
        "plate should echo typed filter; got:\n{viewport}"
    );
}

#[test]
fn agent_demo_plate_lib_atoms_open_editor_slots() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let cases = [
        ("truncated\r", "TruncatedText", None),
        ("panel\r", "Panel", None),
        ("cancellable\r", "CancellableLoader", Some("\x1b")),
    ];

    for (filter, needle, extra) in cases {
        let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
            Arc::new(AtomicBool::new(false)),
            "",
        )));
        app.borrow_mut().freeze_script_for_test();
        let mut h = TuiTestHarness::new(100, 36);
        h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
            .focus(Some(0));
        h.render_result().expect("initial");
        h.keys(&format!("\x10{filter}"));
        h.render_result().expect("after plate");
        let viewport = h.tui.terminal.viewport().join("\n");
        assert!(
            viewport.contains(needle),
            "plate filter `{filter}` should open `{needle}` slot; got:\n{viewport}"
        );
        if let Some(keys) = extra {
            h.keys(keys);
            h.render_result().expect("after atom action");
            let after = h.tui.terminal.scroll_buffer().join("\n");
            if keys == "\x1b" {
                assert!(
                    after.contains("on_abort") || after.contains("aborted"),
                    "Esc should fire CancellableLoader on_abort; got:\n{after}"
                );
            }
        }
    }
}

#[test]
fn agent_demo_plate_ask_single_opens_choice_prompt() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();
    let mut h = TuiTestHarness::new(100, 40);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("initial");
    h.keys("\x10ask-single\r");
    h.render_result().expect("ask-single");
    let viewport = h.tui.terminal.viewport().join("\n");
    assert!(
        viewport.contains("ChoicePrompt") && viewport.contains("本轮优先"),
        "ask-single should open ChoicePrompt; got:\n{viewport}"
    );
    h.keys("\x1b");
    h.render_result().expect("after esc");
    let after = h.tui.terminal.scroll_buffer().join("\n");
    assert!(
        after.contains("cancelled"),
        "Esc should cancel ChoicePrompt; got:\n{after}"
    );
}

#[test]
fn agent_demo_seed_tool_blocks_use_status_background_tints() {
    use agent_demo_example::ToolBlockStatus;
    use xylitol_tui::SemanticPalette;

    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));
    h.render_result().expect("initial render");

    let p = SemanticPalette::dark();
    let success = ToolBlockStatus::Success.rgb(&p);
    let error = ToolBlockStatus::Error.rgb(&p);
    assert!(
        viewport_has_bg_rgb(&h, success),
        "success tool/diff rows must paint DESIGN success tint {success:?}"
    );
    assert!(
        viewport_has_bg_rgb(&h, error),
        "error tool seed must paint DESIGN error tint {error:?}"
    );
}

#[test]
fn agent_demo_scripted_tool_flips_pending_to_success_bg() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::{SharedFakeCodingAgentApp, ToolBlockStatus};

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))));
    let idx = app.borrow_mut().inject_pending_tool_for_test();

    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("initial render");

    let pending = ToolBlockStatus::Pending.rgb(&xylitol_tui::SemanticPalette::dark());
    let text0 = h.tui.terminal.viewport().join("\n");
    assert!(
        text0.contains("inject-tool") && text0.contains("· running"),
        "injected pending tool visible; got:\n{text0}"
    );
    assert!(
        viewport_has_bg_rgb(&h, pending),
        "injected tool must start with pending tint {pending:?}"
    );

    app.borrow_mut().complete_tool_at_for_test(idx);
    h.render_result().expect("after complete");
    let text1 = h.tui.terminal.viewport().join("\n");
    assert!(
        text1.contains("inject-tool") && text1.contains("· ok"),
        "summary should flip to · ok; got:\n{text1}"
    );
    assert!(
        !viewport_has_bg_rgb(&h, pending),
        "pending tint must clear after success flip"
    );
}

#[test]
fn agent_demo_parallel_tools_flip_by_index_not_last() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::{SharedFakeCodingAgentApp, ToolBlockStatus};

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))));
    let (a, b) = app.borrow_mut().inject_parallel_pending_tools_for_test();

    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("initial");

    let pending = ToolBlockStatus::Pending.rgb(&xylitol_tui::SemanticPalette::dark());
    assert!(viewport_has_bg_rgb(&h, pending), "both tools start pending");

    // Complete only A — B must stay · running (regression: old code flipped "last").
    app.borrow_mut().complete_tool_at_for_test(a);
    h.render_result().ok();
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("inject-tool · ok"),
        "A should be ok; got:\n{text}"
    );
    assert!(
        text.contains("inject-tool-b · running"),
        "B must remain running when only A completes; got:\n{text}"
    );
    assert!(
        viewport_has_bg_rgb(&h, pending),
        "B pending tint must remain"
    );

    app.borrow_mut().complete_tool_at_for_test(b);
    h.render_result().ok();
    let text2 = h.tui.terminal.viewport().join("\n");
    assert!(
        text2.contains("inject-tool-b · ok"),
        "B should flip independently; got:\n{text2}"
    );
    assert!(
        !viewport_has_bg_rgb(&h, pending),
        "no pending tint after both complete"
    );
}

#[test]
fn agent_demo_compaction_and_retry_status_chrome() {
    let mut app = FakeCodingAgentApp::new(Arc::new(AtomicBool::new(false)));
    app.demo_compaction_status_for_test();
    assert_eq!(app.status_text_for_test(), "Compacting");
    for _ in 0..80 {
        let _ = app.tick_for_test();
    }
    assert_eq!(
        app.status_text_for_test(),
        "Ready",
        "compaction demo should settle on Ready"
    );

    app.demo_retry_status_for_test();
    assert_eq!(app.status_text_for_test(), "Retry 1/3");
    for _ in 0..100 {
        let _ = app.tick_for_test();
    }
    assert_eq!(
        app.status_text_for_test(),
        "Ready",
        "retry demo should settle on Ready"
    );
}

#[test]
fn agent_demo_idle_returns_to_ready_after_tool_flips() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))));
    let (a, b) = app.borrow_mut().inject_parallel_pending_tools_for_test();
    // Drop scheduled flips — complete manually to assert Ready sync.
    app.borrow_mut().clear_scheduled_actions_for_test();
    app.borrow_mut().freeze_script_for_test();

    let mut h = TuiTestHarness::new(120, 40);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().ok();
    assert!(
        h.tui.terminal.viewport().join("\n").contains("Working")
            || app.borrow().status_text_for_test() == "Working",
        "pending tools keep Working"
    );

    app.borrow_mut().complete_tool_at_for_test(a);
    app.borrow_mut().complete_tool_at_for_test(b);
    assert_eq!(
        app.borrow().status_text_for_test(),
        "Ready",
        "all tools terminal + idle script → Ready"
    );
}

#[test]
fn agent_demo_diff_body_skips_tool_status_bg() {
    use agent_demo_example::ToolBlockStatus;
    use support::Color;
    use xylitol_tui::SemanticPalette;

    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));
    h.render_result().expect("initial render");

    let p = SemanticPalette::dark();
    let success = Color::Rgb(
        ToolBlockStatus::Success.rgb(&p).0,
        ToolBlockStatus::Success.rgb(&p).1,
        ToolBlockStatus::Success.rgb(&p).2,
    );
    // DESIGN.md diff-removed-bg / diff-added-bg
    let removed_bg = Color::Rgb(0x2b, 0x1e, 0x24);
    let added_bg = Color::Rgb(0x1e, 0x2b, 0x22);
    let height = h.tui.terminal.viewport().len();
    let top = h.tui.terminal.viewport_top_pub();
    let mut found_body = false;
    let mut saw_removed_bg = false;
    let mut saw_added_bg = false;
    for row in 0..height {
        let line = h.tui.terminal.viewport()[row].clone();
        if !(line.contains("Ready") && line.contains("Working")) {
            continue;
        }
        found_body = true;
        let width = h.tui.terminal.grid_row(top + row).len();
        for col in 0..width {
            let cell = h.tui.terminal.viewport_cell(row, col);
            assert_ne!(
                cell.bg, success,
                "diff body must not use tool-success-bg; cell({row},{col})={cell:?} line={line}"
            );
            if cell.bg == removed_bg {
                saw_removed_bg = true;
            }
            if cell.bg == added_bg {
                saw_added_bg = true;
            }
        }
    }
    assert!(
        found_body,
        "expected SBS Ready|Working body row in viewport"
    );
    assert!(
        !saw_removed_bg && !saw_added_bg,
        "SBS body must not use Mocha diff row tints (c464; removed={saw_removed_bg} added={saw_added_bg})"
    );
}

#[test]
fn agent_demo_tool_detail_does_not_echo_command() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))));
    app.borrow_mut().freeze_script_for_test();
    // Simulate a completed scripted tool via apply path: push like apply_event does.
    {
        let mut a = app.borrow_mut();
        a.push_tool_for_test(
            "cargo test -p xylitol-tui --test agent_demo_test · ok",
            "(exit 0 — demo stub)",
        );
    }

    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(SharedFakeCodingAgentApp(app)))
        .focus(Some(0));
    h.render_result().ok();
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("cargo test -p xylitol-tui --test agent_demo_test"),
        "command stays on header; got:\n{text}"
    );
    assert!(
        !text.contains("$ cargo test"),
        "detail must not re-echo `$ cmd`; got:\n{text}"
    );
    assert!(
        text.contains("exit 0"),
        "detail keeps exit stub; got:\n{text}"
    );
}

/// True if any scrollback cell has truecolor foreground `rgb`.
fn scrollback_has_fg_rgb(h: &TuiTestHarness, rgb: (u8, u8, u8)) -> bool {
    use support::Color;
    let want = Color::Rgb(rgb.0, rgb.1, rgb.2);
    for row in 0..h.tui.terminal.grid_len() {
        for cell in h.tui.terminal.grid_row(row) {
            if cell.fg == want {
                return true;
            }
        }
    }
    false
}

/// True if any viewport cell has truecolor background `rgb`.
fn viewport_has_bg_rgb(h: &TuiTestHarness, rgb: (u8, u8, u8)) -> bool {
    use support::Color;
    let want = Color::Rgb(rgb.0, rgb.1, rgb.2);
    let height = h.tui.terminal.viewport().len();
    let width = h
        .tui
        .terminal
        .grid_row(h.tui.terminal.viewport_top_pub())
        .len();
    for row in 0..height {
        for col in 0..width {
            if h.tui.terminal.viewport_cell(row, col).bg == want {
                return true;
            }
        }
    }
    false
}

#[test]
fn agent_demo_session_tree_slot_replaces_editor() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();
    app.borrow_mut().open_session_tree_for_test();

    let mut h = TuiTestHarness::new(100, 28);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("tree slot render");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("Session tree"),
        "expected tree header; got:\n{text}"
    );
    assert!(
        (text.contains("user:") && text.contains("tighten footer"))
            || text.contains("├")
            || text.contains("└"),
        "expected tree rows/connectors with kind prefix; got:\n{text}"
    );
    assert!(
        text.contains("[default]"),
        "status suffix for default filter; got:\n{text}"
    );
    assert!(
        app.borrow().tree_open_for_test(),
        "tree should stay open until Esc/Enter"
    );

    // Ctrl+T → no-tools (demo filter; hides kind=tool rows)
    h.keys("\x14");
    h.render_result().expect("after no-tools filter");
    let filtered = h.tui.terminal.viewport().join("\n");
    assert!(
        filtered.contains("[no-tools]"),
        "expected [no-tools] suffix; got:\n{filtered}"
    );
    assert!(
        !(filtered.contains("tool:") && filtered.contains("rg")),
        "no-tools must hide tool rows; got:\n{filtered}"
    );

    h.keys("fork");
    h.render_result().expect("after search");
    let searched = h.tui.terminal.viewport().join("\n");
    assert!(
        searched.contains("Search: fork"),
        "expected search chrome; got:\n{searched}"
    );

    // Esc clears search first
    h.keys("\x1b");
    h.render_result().expect("clear search");
    assert!(
        app.borrow().tree_open_for_test(),
        "first Esc clears search, tree stays open"
    );

    let with_ann = h.tui.terminal.viewport().join("\n");
    assert!(
        with_ann.contains("[ship]") || with_ann.contains("[alt]"),
        "expected annotation markers; got:\n{with_ann}"
    );

    h.keys("\x1b");
    h.render_result().expect("close tree");
    assert!(
        !app.borrow().tree_open_for_test(),
        "Esc should close session tree"
    );
}

#[test]
fn agent_demo_double_esc_opens_session_tree_when_editor_empty() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();

    let mut h = TuiTestHarness::new(100, 28);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("initial");
    // Two Esc within 500ms (empty editor) → tree.
    h.keys("\x1b\x1b");
    h.render_result().expect("after double Esc");
    assert!(
        app.borrow().tree_open_for_test(),
        "double Esc on empty editor should open session tree"
    );
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("Session tree"),
        "expected session tree chrome; got:\n{text}"
    );
}

#[test]
fn agent_demo_session_tree_fold_and_label_edit() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use xylitol_tui::InputEvent;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();
    app.borrow_mut().open_session_tree_for_test();
    app.borrow_mut().tree_select_id_for_test("u1");
    app.borrow_mut().tree_fold_selected_for_test();
    assert!(
        app.borrow().tree_is_folded_for_test("u1"),
        "u1 should be folded"
    );

    let mut h = TuiTestHarness::new(100, 32);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("after fold");
    let folded = h.tui.terminal.viewport().join("\n");
    assert!(
        folded.contains('⊞') || folded.contains("⊞"),
        "expected fold marker; got:\n{folded}"
    );
    assert!(
        !(folded.contains("tool:") && folded.contains("rg")),
        "folded u1 should hide tool descendant; got:\n{folded}"
    );

    h.tui.dispatch_event(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('l'),
        KeyModifiers::SHIFT,
    )));
    h.render_result().expect("label edit");
    let editing = h.tui.terminal.viewport().join("\n");
    assert!(
        editing.contains("Label edit"),
        "expected label editor chrome; got:\n{editing}"
    );
    h.keys("ok\r");
    h.render_result().expect("after label save");
    let labeled = h.tui.terminal.viewport().join("\n");
    assert!(
        labeled.contains("[ok]"),
        "saved annotation should show; got:\n{labeled}"
    );
}

#[test]
fn agent_demo_session_tree_travel_rebuilds_history_along_path() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();
    assert_eq!(app.borrow().history_leaf_for_test(), "u2");

    app.borrow_mut().open_session_tree_for_test();
    app.borrow_mut().tree_select_id_for_test("af");

    let mut h = TuiTestHarness::new(100, 32);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("tree before travel");
    h.keys("\r");
    h.render_result().expect("after travel");

    assert!(
        !app.borrow().tree_open_for_test(),
        "Enter travel must close the session tree"
    );
    assert_eq!(app.borrow().history_leaf_for_test(), "af");
    assert_eq!(
        app.borrow().status_text_for_test(),
        "Ready",
        "travel must leave idle status (no spinner)"
    );

    let plain = app.borrow().transcript_plain_for_test();
    assert!(
        plain.contains("history @ af") && plain.contains("root → fork → af"),
        "expected history banner with path; got:\n{plain}"
    );
    assert!(
        plain.contains("alternate branch") && plain.contains("fork leaf"),
        "expected fork-path messages only; got:\n{plain}"
    );
    assert!(
        !plain.contains("tighten footer truncation"),
        "main-branch user turn must not remain after travel to fork; got:\n{plain}"
    );

    // Re-open tree: active leaf is af (• on path).
    app.borrow_mut().open_session_tree_for_test();
    h.render_result().expect("tree after travel");
    let tree = h.tui.terminal.viewport().join("\n");
    assert!(
        tree.contains("fork leaf") || tree.contains("alternate branch"),
        "tree should still show fork branch; got:\n{tree}"
    );
}

#[test]
fn agent_demo_submit_grows_session_tree() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        Arc::new(AtomicBool::new(false)),
        "",
    )));
    app.borrow_mut().freeze_script_for_test();
    app.borrow_mut()
        .submit_text_for_test("UNIQUE_TREE_GROW_PROMPT_xyz");

    assert!(
        app.borrow()
            .session_tree_contains_label_for_test("UNIQUE_TREE_GROW_PROMPT_xyz"),
        "submitted user text must appear as a session-tree node"
    );
    assert!(
        app.borrow().history_leaf_for_test().starts_with("live-u-"),
        "history leaf should advance to the new user node; got {}",
        app.borrow().history_leaf_for_test()
    );

    let mut h = TuiTestHarness::new(100, 28);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    app.borrow_mut().open_session_tree_for_test();
    h.render_result().expect("tree with live node");
    let tree = h.tui.terminal.viewport().join("\n");
    assert!(
        tree.contains("UNIQUE_TREE_GROW_PROMPT_xyz"),
        "opened tree must render the live user node; got:\n{tree}"
    );
}

#[test]
fn agent_demo_travel_to_user_prefills_editor_and_omits_reply_spine() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();
    app.travel_to_history_for_test("u1");
    let plain = app.transcript_plain_for_test();
    assert!(
        !plain.contains("tighten footer truncation"),
        "selected user body goes to editor, not transcript; got:\n{plain}"
    );
    assert!(
        !plain.contains("plan + tools") && !plain.contains("tree selector"),
        "travel to user must NOT include linear assistant reply; got:\n{plain}"
    );
    assert_eq!(
        app.history_leaf_for_test(),
        "root",
        "user travel leaf must be parent"
    );
    assert_eq!(
        app.input_text_for_test(),
        "tighten footer truncation",
        "user travel must prefill editor"
    );
}

#[test]
fn agent_demo_travel_to_assistant_does_not_prefill_user_body() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();
    app.set_editor_text_for_test("should be cleared");
    app.travel_to_history_for_test("a1");
    assert_eq!(app.history_leaf_for_test(), "a1");
    assert!(
        app.input_text_for_test().is_empty(),
        "non-user travel must not leave user body in editor; got {:?}",
        app.input_text_for_test()
    );
    let plain = app.transcript_plain_for_test();
    assert!(
        plain.contains("tighten footer truncation") && plain.contains("plan + tools"),
        "path to assistant includes prior user+assistant; got:\n{plain}"
    );
}

#[test]
fn agent_demo_steer_does_not_abort_busy_turn() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();
    // Start a turn (busy).
    app.submit_text_for_test("first turn prompt");
    assert!(
        app.status_text_for_test() == "Thinking"
            || app.status_text_for_test() == "Working"
            || app.status_text_for_test() == "Running tools"
            || app.status_text_for_test() == "Drafting reply",
        "expected busy status after submit; got {}",
        app.status_text_for_test()
    );
    let before_leaf = app.history_leaf_for_test().to_string();
    app.submit_text_for_test("steer while busy");
    assert_eq!(app.steer_queue_len_for_test(), 1);
    assert!(
        app.session_tree_contains_label_for_test("[steer] steer while busy"),
        "steer must grow a tree node without aborting"
    );
    // Still busy / first turn not wiped into Ready-only by a second queue_simulated_turn clear.
    assert!(
        app.steer_queue_len_for_test() == 1,
        "steer stays queued until turn finishes"
    );
    assert_ne!(
        app.history_leaf_for_test(),
        before_leaf,
        "steer advances leaf under the in-flight branch"
    );
}

#[test]
fn agent_demo_follow_up_queues_while_busy() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();
    app.submit_text_for_test("busy turn");
    app.enqueue_follow_up_for_test("later please");
    assert_eq!(app.follow_up_queue_len_for_test(), 1);
    assert!(
        !app.session_tree_contains_label_for_test("later please"),
        "follow-up must not grow the tree until applied"
    );
}

#[test]
fn agent_demo_bang_prefix_enables_bash_border() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();
    assert!(!app.bash_mode_for_test());
    app.set_editor_text_for_test("!echo hi");
    app.sync_editor_border_for_test();
    assert!(app.bash_mode_for_test(), "! prefix must enable bash border");
    app.set_editor_text_for_test("echo hi");
    app.sync_editor_border_for_test();
    assert!(
        !app.bash_mode_for_test(),
        "clearing ! restores default border"
    );
}

#[test]
fn agent_demo_ctrl_g_external_editor_stub() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();
    app.set_editor_text_for_test("draft body");
    assert_eq!(app.external_editor_invocations_for_test(), 0);
    app.open_external_editor_stub_for_test();
    assert_eq!(app.external_editor_invocations_for_test(), 1);
    assert!(
        app.input_text_for_test().contains("$EDITOR stub"),
        "stub should mark editor text; got {}",
        app.input_text_for_test()
    );
    let plain = app.transcript_plain_for_test();
    assert!(
        plain.contains("external editor stub"),
        "expected system banner; got:\n{plain}"
    );
}

#[test]
fn agent_demo_ctrl_g_defaults_to_stub_without_tty() {
    // Harness stdin is not a TTY → request_external_editor must use stub, not pending.
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();
    app.set_editor_text_for_test("no tty");
    app.request_external_editor_for_test();
    assert!(
        !app.take_pending_external_editor(),
        "non-TTY must not arm real-editor pending"
    );
    assert!(
        app.input_text_for_test().contains("$EDITOR stub"),
        "expected stub marker; got {}",
        app.input_text_for_test()
    );
}

#[test]
fn agent_demo_theme_defaults_dark() {
    let app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    assert!(
        !app.theme_auto_for_test(),
        "auto must be off unless env enables it"
    );
    assert_eq!(
        app.theme_mode_for_test(),
        xylitol_tui::TerminalColorScheme::Dark
    );
}

#[test]
fn agent_demo_theme_auto_osc11_light() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.set_theme_auto_for_test(true);
    app.apply_theme_detect_for_test(Some("\x1b]11;#eff1f5\x07"), Some("15;0"), None);
    assert_eq!(
        app.theme_mode_for_test(),
        xylitol_tui::TerminalColorScheme::Light,
        "OSC11 light bg must win over dark COLORFGBG"
    );
}

#[test]
fn agent_demo_feed_terminal_color_reply_light() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.set_theme_auto_for_test(true);
    app.feed_terminal_color_reply("\x1b]11;#eff1f5\x07");
    assert_eq!(
        app.theme_mode_for_test(),
        xylitol_tui::TerminalColorScheme::Light
    );
    let light = xylitol_tui::SemanticPalette::light();
    assert_eq!(app.palette().accent, light.accent);
}

#[test]
fn agent_demo_light_chrome_uses_latte_tool_bg() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::{SharedFakeCodingAgentApp, ToolBlockStatus};

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))));
    {
        let mut a = app.borrow_mut();
        a.set_theme_auto_for_test(true);
        a.apply_theme_detect_for_test(Some("\x1b]11;#eff1f5\x07"), None, None);
        a.freeze_script_for_test();
        a.inject_pending_tool_for_test();
    }

    let mut h = TuiTestHarness::new(120, 80);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("light render");

    let latte_pending = ToolBlockStatus::Pending.rgb(&xylitol_tui::SemanticPalette::light());
    let mocha_pending = ToolBlockStatus::Pending.rgb(&xylitol_tui::SemanticPalette::dark());
    assert_ne!(latte_pending, mocha_pending);
    assert!(
        viewport_has_bg_rgb(&h, latte_pending),
        "light mode tool rows must use Latte pending bg {latte_pending:?}"
    );
}

#[test]
fn agent_demo_theme_slash_light_and_toggle() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    assert_eq!(
        app.theme_mode_for_test(),
        xylitol_tui::TerminalColorScheme::Dark
    );
    app.submit_text_for_test("/theme light");
    assert_eq!(
        app.theme_mode_for_test(),
        xylitol_tui::TerminalColorScheme::Light
    );
    assert!(
        !app.theme_auto_for_test(),
        "explicit /theme must turn auto off"
    );
    app.submit_text_for_test("/theme toggle");
    assert_eq!(
        app.theme_mode_for_test(),
        xylitol_tui::TerminalColorScheme::Dark
    );
    app.submit_text_for_test("/theme");
    assert_eq!(
        app.theme_mode_for_test(),
        xylitol_tui::TerminalColorScheme::Light,
        "bare /theme toggles"
    );
}

#[test]
fn agent_demo_theme_auto_off_ignores_sources() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    assert!(!app.theme_auto_for_test());
    app.apply_theme_detect_for_test(Some("\x1b]11;#ffffff\x07"), Some("0;15"), None);
    assert_eq!(
        app.theme_mode_for_test(),
        xylitol_tui::TerminalColorScheme::Dark,
        "without auto, probes must not flip theme"
    );
}

#[test]
fn agent_demo_session_tree_fork_stays_on_node_and_branches() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();

    // u1 already has one child (a1) in the sample tree.
    let before = app.session_tree_child_count_for_test("u1");
    assert!(before >= 1, "sample tree u1 should have children");

    app.open_session_tree_for_test();
    app.tree_select_id_for_test("u1");
    app.fork_from_selected_for_test();

    assert!(!app.tree_open_for_test(), "fork closes the tree");
    assert_eq!(
        app.history_leaf_for_test(),
        "u1",
        "fork leaf must stay on selected user (not reply spine)"
    );
    assert_eq!(
        app.input_text_for_test(),
        "tighten footer truncation",
        "fork prefills the user prompt"
    );
    let plain = app.transcript_plain_for_test();
    assert!(
        plain.contains("forked @ u1") && plain.contains("tighten footer truncation"),
        "expected fork banner + user turn; got:\n{plain}"
    );
    // u1's path is root→u1 only; child assistant must not appear (unlike travel spine).
    assert!(
        !plain.contains("plan + tools"),
        "fork must not include child assistant reply; got:\n{plain}"
    );

    app.submit_text_for_test("forked alternate reply prompt");
    let after = app.session_tree_child_count_for_test("u1");
    assert_eq!(
        after,
        before + 1,
        "submit after fork must add a sibling under u1"
    );
    assert!(
        app.session_tree_contains_label_for_test("forked alternate reply prompt"),
        "new branch user node must appear in the live tree"
    );
}

#[test]
fn agent_demo_tool_event_grows_session_tree() {
    let mut app = FakeCodingAgentApp::new_with_prompt(Arc::new(AtomicBool::new(false)), "");
    app.freeze_script_for_test();
    app.submit_text_for_test("need tools in tree");
    // Drive script until a tool lands.
    let mut saw_tool = false;
    for _ in 0..400 {
        let _ = app.tick_for_test();
        if app.session_tree_contains_label_for_test("tool: rg -n")
            || app.session_tree_contains_label_for_test("rg -n")
        {
            saw_tool = true;
            break;
        }
    }
    assert!(
        saw_tool,
        "scripted Tool events must grow tool: nodes in the session tree"
    );
}

#[test]
fn agent_demo_collapsed_tool_viewport_shows_earlier_hint_and_tail() {
    let mut h = TuiTestHarness::new(120, 100);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));
    h.render_result().expect("initial");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("earlier lines") && text.contains("ctrl+o to expand"),
        "collapsed long tool detail must show pi-style hint; got:\n{text}"
    );
    assert!(
        text.contains("Took 9.3s"),
        "collapsed viewport keeps the tail; got:\n{text}"
    );
    assert!(
        !text.contains("suite-01"),
        "early lines stay hidden while collapsed; got:\n{text}"
    );
}

#[test]
fn agent_demo_ctrl_o_expands_tool_output_viewport() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))));
    let mut h = TuiTestHarness::new(120, 120);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("initial");
    assert!(!app.borrow().tools_output_expanded_for_test());
    h.keys("\x0f"); // Ctrl+O
    h.render_result().expect("after Ctrl+O");
    assert!(app.borrow().tools_output_expanded_for_test());
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("suite-01") && text.contains("Took 9.3s"),
        "Ctrl+O expands full tool detail; got:\n{text}"
    );
    assert!(
        !text.contains("ctrl+o to expand"),
        "hint disappears when expanded; got:\n{text}"
    );
}

#[test]
fn agent_demo_streaming_tool_detail_sticks_to_tail_while_collapsed() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use agent_demo_example::SharedFakeCodingAgentApp;

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))));
    app.borrow_mut().freeze_script_for_test();
    let idx = {
        let mut a = app.borrow_mut();
        a.clear_scheduled_actions_for_test();
        let i = a.transcript_len_for_test();
        a.push_tool_for_test("$ stream-demo · running", "");
        for n in 1..=12 {
            a.append_tool_detail_for_test(i, format!("row-{n:02}\n"));
        }
        i
    };
    let _ = idx;

    let mut h = TuiTestHarness::new(100, 60);
    h.mount(Box::new(SharedFakeCodingAgentApp(app.clone())))
        .focus(Some(0));
    h.render_result().expect("render");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("earlier lines") && text.contains("row-12"),
        "streaming collapsed viewport sticks to last lines; got:\n{text}"
    );
    assert!(
        !text.contains("row-01"),
        "early streamed lines hidden while collapsed; got:\n{text}"
    );

    app.borrow_mut().set_tools_output_expanded_for_test(true);
    h.render_result().expect("expanded");
    let full = h.tui.terminal.viewport().join("\n");
    assert!(
        full.contains("row-01") && full.contains("row-12"),
        "expanded shows full streamed detail; got:\n{full}"
    );
}

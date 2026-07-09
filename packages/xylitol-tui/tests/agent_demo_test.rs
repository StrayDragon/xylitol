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
    h.assert_text_contains("Show this help");
    h.assert_text_contains("palette");

    // Prefix filter: `/hel` should keep help, drop unrelated commands.
    h.keys("hel");
    h.render_result()
        .expect("filtered slash popup must stay within width");
    let filtered = h.tui.terminal.viewport().join("\n");
    assert!(
        filtered.contains("help") && filtered.contains("Show this help"),
        "filtered popup should still show /help; got:\n{filtered}"
    );
    assert!(
        !filtered.contains("Show workspace diff"),
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
    h.assert_text_contains("Show this help");
    h.keys("\x7f"); // Backspace → `/` only → close popup, keep `/`
    h.render_result()
        .expect("backspace to lone slash must stay within width");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        !text.contains("Show this help"),
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
        .expect("command palette selector must stay within width budget");
    h.assert_text_contains("Command Palette");
    h.assert_text_contains("Run regression tests");
    let text = h.tui.terminal.viewport().join("\n");
    let palette_pos = text
        .find("Command Palette")
        .expect("palette title should be visible");
    let footer_pos = text
        .find("esc close")
        .expect("selector footer hint should remain below");
    assert!(
        palette_pos < footer_pos,
        "palette must sit in the editor slot above the footer; got:\n{text}"
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
    h.assert_text_contains("Command Palette");
    h.assert_text_contains("Run regression tests");
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
    let mut h = TuiTestHarness::new(172, 40);
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
        text.contains("thinking"),
        "seed transcript should include a collapsed thinking block; got:\n{text}"
    );
    assert!(
        text.contains("^P") && text.contains("^S") && text.contains("keys:"),
        "seed should teach palette/settings keys without a permanent chrome wall; got:\n{text}"
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
fn agent_demo_ctrl_t_expands_thinking_block() {
    let mut h = TuiTestHarness::new(172, 40);
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
        !before.contains("Keep transcript in scrollback"),
        "collapsed thinking must hide body; got:\n{before}"
    );

    h.keys("\x14"); // Ctrl+T
    h.render_result()
        .expect("expand thinking must stay within width");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("Keep transcript in scrollback"),
        "Ctrl+T should expand thinking body; got:\n{after}"
    );
}

#[test]
fn agent_demo_alt_e_expands_tool_block() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    let before = h.tui.terminal.viewport().join("\n");
    assert!(
        before.contains("agent_demo.rs"),
        "seed tool block starts collapsed; got:\n{before}"
    );
    assert!(
        !before.contains("demo stub") && !before.contains("opened agent_demo"),
        "collapsed tool must hide detail; got:\n{before}"
    );

    h.keys("\x1be"); // Alt+E (not Ctrl+E — reserved for editor cursorLineEnd)
    h.render_result()
        .expect("expand tools must stay within width");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("opened agent_demo") || after.contains("FakeCodingAgentApp"),
        "Alt+E should expand tool detail; got:\n{after}"
    );
}

#[test]
fn agent_demo_alt_g_cycles_glyph_set_to_ascii() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.assert_text_contains("❯");
    h.keys("\x1bg"); // Alt+G (not Ctrl+G — reserved for future external editor)
    h.render_result()
        .expect("glyph cycle must stay within width");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("ascii") && text.contains("^P/^S"),
        "Alt+G should switch glyph set and footer should keep palette/settings cues; got:\n{text}"
    );
    assert!(
        text.contains("glyph_set=ascii"),
        "system note should confirm the switch; got:\n{text}"
    );
}

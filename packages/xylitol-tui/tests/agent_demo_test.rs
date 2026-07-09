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
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.keys("\r");
    for _ in 0..200 {
        h.tick();
        h.render_result()
            .expect("streaming scripted turn must stay within width budget");
    }
    h.assert_text_contains("rg -n");
    h.assert_text_contains("收到，我已经接住");
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
    for _ in 0..200 {
        h.tick();
        h.render_result()
            .expect("streaming must stay within width budget");
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
        text.contains("feat/tui-dev"),
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
fn agent_demo_ctrl_e_expands_tool_block() {
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

    h.keys("\x05"); // Ctrl+E
    h.render_result()
        .expect("expand tools must stay within width");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("opened agent_demo") || after.contains("FakeCodingAgentApp"),
        "Ctrl+E should expand tool detail; got:\n{after}"
    );
}

#[test]
fn agent_demo_ctrl_g_cycles_glyph_set_to_ascii() {
    let mut h = TuiTestHarness::new(172, 40);
    h.mount(Box::new(FakeCodingAgentApp::new(Arc::new(
        AtomicBool::new(false),
    ))))
    .focus(Some(0));

    h.render_result().expect("initial render should succeed");
    h.assert_text_contains("❯");
    h.keys("\x07"); // Ctrl+G
    h.render_result()
        .expect("glyph cycle must stay within width");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        text.contains("ascii") && text.contains("^P/^S"),
        "Ctrl+G should switch glyph set and footer should keep palette/settings cues; got:\n{text}"
    );
    assert!(
        text.contains("glyph_set=ascii"),
        "system note should confirm the switch; got:\n{text}"
    );
}

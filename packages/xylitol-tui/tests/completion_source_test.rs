//! CompletionSource registry wiring through Editor (no slash/`@` hardcoding).

mod support;

use support::TuiTestHarness;
use xylitol_tui::Focusable;
use xylitol_tui::autocomplete::SlashCommand;
use xylitol_tui::clock::SystemClock;
use xylitol_tui::completion::{AtPathSource, SlashCommandSource};
use xylitol_tui::components::editor::{Editor, EditorOptions, EditorTheme};

fn editor_with(sources: Vec<Box<dyn xylitol_tui::CompletionSource>>) -> Editor {
    let mut e = Editor::new(
        EditorTheme::default(),
        EditorOptions {
            padding_x: 0,
            terminal_rows: 12,
        },
        Box::new(SystemClock),
    );
    e.set_focused(true);
    e.set_completion_sources(sources);
    e.set_text(String::new());
    e
}

#[test]
fn slash_only_source_does_not_open_on_at() {
    let mut h = TuiTestHarness::new(80, 20);
    h.mount(Box::new(editor_with(vec![Box::new(
        SlashCommandSource::new(vec![SlashCommand {
            name: "help".into(),
            description: Some("Show help".into()),
            argument_hint: None,
            get_argument_completions: None,
        }]),
    )])))
    .focus(Some(0));

    h.render_result().expect("render");
    h.keys("@");
    h.render_result().expect("@ with slash-only sources");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        !text.contains("Show help"),
        "slash-only registry must not open on @; got:\n{text}"
    );
}

#[test]
fn at_source_opens_on_at() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("only_me.rs"), "").unwrap();

    let mut h = TuiTestHarness::new(80, 20);
    h.mount(Box::new(editor_with(vec![Box::new(AtPathSource::new(
        dir.path().to_path_buf(),
    ))])))
    .focus(Some(0));

    h.render_result().expect("render");
    h.keys("@");
    h.render_result().expect("@ with AtPathSource");
    h.assert_text_contains("only_me.rs");
}

#[test]
fn empty_registry_ignores_slash() {
    let mut h = TuiTestHarness::new(80, 20);
    h.mount(Box::new(editor_with(vec![]))).focus(Some(0));

    h.render_result().expect("render");
    h.keys("/help");
    h.render_result().expect("slash with empty registry");
    let text = h.tui.terminal.viewport().join("\n");
    // Editor still contains typed text; no SelectList description chrome.
    assert!(
        text.contains("/help"),
        "typed slash text should remain; got:\n{text}"
    );
}

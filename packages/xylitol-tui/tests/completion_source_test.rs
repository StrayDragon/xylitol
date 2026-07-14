//! CompletionSource registry wiring through Editor (no slash/`@` hardcoding).

mod support;

use support::TuiTestHarness;
use xylitol_tui::Focusable;
use xylitol_tui::autocomplete::{AutocompleteItem, AutocompleteSuggestions, SlashCommand};
use xylitol_tui::clock::SystemClock;
use xylitol_tui::completion::{
    AtPathSource, CompletionContext, CompletionMatch, CompletionSource, SlashArgCompletionSource,
    SlashCommandSource,
};
use xylitol_tui::components::editor::{Editor, EditorOptions, EditorTheme};
use xylitol_tui::extract_dollar_prefix;
use xylitol_tui::utils::visible_width;

/// Test-only `$` trigger — proves a third source can register without product semantics.
/// Probe/apply mirror `@`: inline `$skill` anywhere before the cursor.
struct DollarStubSource {
    skills: Vec<(&'static str, &'static str)>,
}

impl CompletionSource for DollarStubSource {
    fn id(&self) -> &'static str {
        "dollar-stub"
    }

    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch> {
        extract_dollar_prefix(ctx.before_cursor()).map(|prefix| CompletionMatch { prefix })
    }

    fn should_dismiss(&self, ctx: &CompletionContext<'_>, _m: &CompletionMatch) -> bool {
        extract_dollar_prefix(ctx.before_cursor()).is_none()
    }

    fn suggestions(
        &self,
        _ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions> {
        let needle = m.prefix.strip_prefix('$').unwrap_or("");
        let items: Vec<AutocompleteItem> = self
            .skills
            .iter()
            .filter(|(name, _)| name.starts_with(needle))
            .map(|(name, desc)| AutocompleteItem {
                value: format!("${name}"),
                label: (*name).to_string(),
                description: Some((*desc).to_string()),
            })
            .collect();
        if items.is_empty() {
            None
        } else {
            Some(AutocompleteSuggestions {
                items,
                prefix: m.prefix.clone(),
            })
        }
    }

    fn apply(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> (Vec<String>, usize, usize) {
        let current = lines[cursor_line].clone();
        let before = &current[..cursor_col.saturating_sub(prefix.len())];
        let after = &current[cursor_col..];
        let suffix = " ";
        let new_line = format!("{}{}{}{}", before, item.value, suffix, after);
        let mut new_lines = lines.to_vec();
        new_lines[cursor_line] = new_line;
        (
            new_lines,
            cursor_line,
            before.len() + item.value.len() + suffix.len(),
        )
    }
}

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

#[test]
fn third_dollar_stub_source_registers_and_opens() {
    let mut h = TuiTestHarness::new(80, 20);
    h.mount(Box::new(editor_with(vec![
        Box::new(SlashCommandSource::new(vec![SlashCommand {
            name: "help".into(),
            description: Some("Show help".into()),
            argument_hint: None,
            get_argument_completions: None,
        }])),
        Box::new(AtPathSource::new(std::env::temp_dir())),
        Box::new(DollarStubSource {
            skills: vec![
                ("demo", "c545 stub skill"),
                (
                    "very-long-skill-name-that-should-truncate-in-narrow-terminals",
                    "long description that must not blow the width budget either",
                ),
            ],
        }),
    ])))
    .focus(Some(0));

    h.render_result().expect("render");
    // Inline like `@`: mid-line `$` must open the stub source.
    h.keys("use $");
    h.render_result().expect("inline $ stub popup");
    h.assert_text_contains("demo");
    h.assert_text_contains("c545 stub skill");

    h.keys("dem\t");
    h.render_result().expect("Tab applies $demo inline");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("use $demo"),
        "Tab should keep leading text and insert $demo; got:\n{after}"
    );
}

#[test]
fn dollar_stub_opens_mid_line_without_leading_dollar() {
    let mut h = TuiTestHarness::new(80, 16);
    h.mount(Box::new(editor_with(vec![Box::new(DollarStubSource {
        skills: vec![("search", "find things")],
    })])))
    .focus(Some(0));

    h.render_result().expect("render");
    h.keys("please $se");
    h.render_result().expect("mid-line $se");
    h.assert_text_contains("search");
    h.assert_text_contains("find things");
}

#[test]
fn slash_still_works_with_dollar_source_registered() {
    let mut h = TuiTestHarness::new(80, 16);
    h.mount(Box::new(editor_with(vec![
        Box::new(SlashCommandSource::new(vec![SlashCommand {
            name: "help".into(),
            description: Some("Show help".into()),
            argument_hint: None,
            get_argument_completions: None,
        }])),
        Box::new(DollarStubSource {
            skills: vec![("demo", "stub")],
        }),
    ])))
    .focus(Some(0));

    h.render_result().expect("render");
    h.keys("/");
    h.render_result().expect("slash with dollar registered");
    h.assert_text_contains("Show help");
}

#[test]
fn narrow_editor_popup_does_not_overflow_width() {
    let width: u16 = 28;
    let mut h = TuiTestHarness::new(width, 16);
    h.mount(Box::new(editor_with(vec![
        Box::new(SlashCommandSource::new(vec![SlashCommand {
            name: "supercalifragilisticexpialidocious".into(),
            description: Some(
                "an extraordinarily long description that would overflow a narrow terminal".into(),
            ),
            argument_hint: None,
            get_argument_completions: None,
        }])),
        Box::new(DollarStubSource {
            skills: vec![(
                "another-extremely-long-skill-identifier-for-c545",
                "narrow popup clamp",
            )],
        }),
    ])))
    .focus(Some(0));

    h.render_result().expect("render");
    h.keys("/");
    h.render_result()
        .expect("narrow slash popup must stay within width");
    let budget = width as usize;
    for line in h.tui.terminal.viewport() {
        assert!(
            visible_width(&line) <= budget,
            "slash popup overflow: visible {} > {budget}; line={line:?}",
            visible_width(&line)
        );
    }

    h.keys("\x1b\x7f$");
    h.render_result()
        .expect("narrow dollar popup must stay within width");
    for line in h.tui.terminal.viewport() {
        assert!(
            visible_width(&line) <= budget,
            "dollar popup overflow: visible {} > {budget}; line={line:?}",
            visible_width(&line)
        );
    }
    h.assert_text_contains("another-extremely");
}

#[test]
fn slash_arg_model_opens_after_space_not_bare() {
    let catalog = vec![
        ("deepseek-v4-flash".into(), "opencode-go".into()),
        ("grok-4.5:slow".into(), "cursor".into()),
    ];
    let mut h = TuiTestHarness::new(80, 20);
    h.mount(Box::new(editor_with(vec![
        Box::new(SlashArgCompletionSource::new("model", catalog).with_id("model-id")),
        Box::new(SlashCommandSource::new(vec![SlashCommand {
            name: "model".into(),
            description: Some("Switch model: /model <id>".into()),
            argument_hint: None,
            get_argument_completions: None,
        }])),
    ])))
    .focus(Some(0));

    h.render_result().expect("render");
    h.keys("/model");
    h.render_result().expect("bare /model");
    let bare = h.tui.terminal.viewport().join("\n");
    assert!(
        bare.contains("Switch model: /model <id>"),
        "bare /model must stay on slash list; got:\n{bare}"
    );
    assert!(
        !bare.contains("opencode-go"),
        "bare /model must not open model-id catalog; got:\n{bare}"
    );

    h.keys(" dee");
    h.render_result().expect("/model dee arg popup");
    h.assert_text_contains("deepseek-v4-flash");
    h.assert_text_contains("opencode-go");
    let arg = h.tui.terminal.viewport().join("\n");
    assert!(
        !arg.contains("Switch model: /model <id>"),
        "arg popup must own the slot; got:\n{arg}"
    );

    h.keys("\t");
    h.render_result().expect("Tab applies model id");
    let after = h.tui.terminal.viewport().join("\n");
    assert!(
        after.contains("/model deepseek-v4-flash"),
        "Tab should write id onto the line; got:\n{after}"
    );
}

#[test]
fn slash_arg_model_esc_closes_without_apply() {
    let catalog = vec![("deepseek-v4-flash".into(), "opencode-go".into())];
    let mut h = TuiTestHarness::new(80, 16);
    h.mount(Box::new(editor_with(vec![Box::new(
        SlashArgCompletionSource::new("model", catalog).with_id("model-id"),
    )])))
    .focus(Some(0));

    h.render_result().expect("render");
    h.keys("/model dee");
    h.render_result().expect("arg popup");
    h.assert_text_contains("deepseek-v4-flash");
    h.keys("\x1b");
    h.render_result().expect("Esc dismisses");
    let text = h.tui.terminal.viewport().join("\n");
    assert!(
        !text.contains("opencode-go"),
        "Esc should close popup; got:\n{text}"
    );
    assert!(
        text.contains("/model dee"),
        "Esc must not rewrite the typed prefix; got:\n{text}"
    );
}

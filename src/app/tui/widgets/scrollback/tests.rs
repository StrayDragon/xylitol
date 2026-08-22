use super::super::fold_hit::FoldTarget;
use super::live::inflight_short_label;
use super::*;
use xylitol_tui::terminal_colors::RgbColor;
use xylitol_tui::{bold, fg_rgb, mix_rgb};

use crate::app::tui::bridge::{AskPhase, BashBlockStatus, CompactionBlockStatus, UiEntry};

#[test]
fn ask_header_paints_accent_ask_and_keeps_full_body() {
    let mut model = UiModel::default();
    model.entries.push(UiEntry::Ask {
        id: "a1".into(),
        summary: "Ask · demo_choice → aaa · demo_multi → 📊 看诊断, 📝 查符号".into(),
        detail_lines: vec![
            "demo_choice → aaa".into(),
            "demo_multi → 📊 看诊断, 📝 查符号, 🔍 搜代码, 🚀 跑命令".into(),
        ],
        phase: AskPhase::Answered,
        expanded: true,
    });
    let theme = LayoutTheme::product_dark();
    let fold = ScrollbackFold {
        tools_expanded: true,
        ..ScrollbackFold::default()
    };
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &fold,
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let joined = lines.join("\n");
    let accent_ask = theme.paint_tool_name("Ask");
    assert!(
        joined.contains(&accent_ask),
        "Ask brand must use accent paint; got:\n{joined}"
    );
    let plain = strip_ansi_local(&joined);
    assert!(
        plain.contains("🔍 搜代码") && plain.contains("🚀 跑命令"),
        "expanded body must stay full; got:\n{plain}"
    );
}

#[test]
fn todo_checklist_defaults_to_summary_line() {
    let mut model = UiModel::default();
    model.entries.push(UiEntry::Todo {
        summary: "Todo · 2/5".into(),
        detail_lines: vec!["[x] done".into(), "[ ] next".into()],
    });
    let theme = LayoutTheme::product_dark();
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold::default(),
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let plain = strip_ansi_local(&lines.join("\n"));
    assert!(plain.contains("Todo · 2/5"), "missing summary: {plain}");
    assert!(
        plain.contains("(Alt+E)") && !plain.contains("((Alt+E))"),
        "folded hint MUST be a single pair of parens: {plain}"
    );
    assert!(
        !plain.contains("[x] done"),
        "detail must stay folded: {plain}"
    );
    assert!(
        !plain.to_lowercase().contains("plan"),
        "must not render Plan side chrome: {plain}"
    );
}

#[test]
fn todo_checklist_expands_with_fold() {
    let mut model = UiModel::default();
    model.entries.push(UiEntry::Todo {
        summary: "Todo · 1/2".into(),
        detail_lines: vec!["[x] done".into(), "[~] wip".into()],
    });
    let theme = LayoutTheme::product_dark();
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold {
            todo_expanded: true,
            ..ScrollbackFold::default()
        },
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let plain = strip_ansi_local(&lines.join("\n"));
    assert!(
        plain.contains("[x] done") && plain.contains("[~] wip"),
        "{plain}"
    );
}

#[test]
fn compaction_block_defaults_collapsed() {
    let mut model = UiModel::default();
    model.entries.push(UiEntry::Compaction {
        status: CompactionBlockStatus::Complete,
        summary: "long summary body that should stay hidden".into(),
        tokens_before: 186_842,
        detail: None,
    });
    let theme = LayoutTheme::product_dark();
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold::default(),
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        100,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let plain = strip_ansi_local(&lines.join("\n"));
    let header = plain
        .lines()
        .find(|l| l.contains("Compacted from"))
        .unwrap_or("");
    assert!(
        header.contains("[compaction]")
            && header.contains("Compacted from 186,842 tokens (Alt+E to expand)")
            && (header.contains('▸') || header.contains('>')),
        "fold triangle MUST sit on the Compacted header line: {plain}"
    );
    assert!(
        !plain.contains("long summary body"),
        "summary must stay hidden when collapsed: {plain}"
    );
}

#[test]
fn compaction_block_expands_with_fold() {
    let mut model = UiModel::default();
    model.entries.push(UiEntry::Compaction {
        status: CompactionBlockStatus::Complete,
        summary: "visible summary body".into(),
        tokens_before: 1_000,
        detail: None,
    });
    let theme = LayoutTheme::product_dark();
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold {
            compaction_expanded: true,
            ..ScrollbackFold::default()
        },
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        100,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let plain = strip_ansi_local(&lines.join("\n"));
    let header = plain
        .lines()
        .find(|l| l.contains("Compacted from"))
        .unwrap_or("");
    assert!(
        header.contains("[compaction]")
            && header.contains("Compacted from 1,000 tokens")
            && (header.contains('▾') || header.contains('v')),
        "expanded triangle MUST sit on the Compacted header line: {plain}"
    );
    assert!(
        plain.contains("visible summary body"),
        "expanded must show summary: {plain}"
    );
    assert!(
        !plain.contains("to expand"),
        "expanded must not show expand hint: {plain}"
    );
}

#[test]
fn inflight_write_placeholder_is_editing_not_dots() {
    let entry = UiEntry::Tool {
        timeout_secs: None,
        id: "w1".into(),
        name: "write".into(),
        args_preview: "...".into(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: String::new(),
        is_error: false,
        done: false,
    };
    assert_eq!(inflight_short_label(&entry).as_deref(), Some("Editing"));
    let with_path = UiEntry::Tool {
        timeout_secs: None,
        id: "w2".into(),
        name: "write".into(),
        args_preview: "a.py".into(),
        tool_path: Some("a.py".into()),
        write_content: None,
        display_diff: None,
        output: String::new(),
        is_error: false,
        done: false,
    };
    assert_eq!(
        inflight_short_label(&with_path).as_deref(),
        Some("Editing a.py")
    );
}

fn inflight_tool(name: &str, path: Option<&str>) -> UiEntry {
    UiEntry::Tool {
        timeout_secs: None,
        id: name.into(),
        name: name.into(),
        args_preview: path.unwrap_or("").into(),
        tool_path: path.map(str::to_string),
        write_content: None,
        display_diff: None,
        output: String::new(),
        is_error: false,
        done: false,
    }
}

#[test]
fn inflight_labels_use_tool_activity_role_not_searchish_substring() {
    assert_eq!(
        inflight_short_label(&inflight_tool("grep", Some("a.rs"))).as_deref(),
        Some("Searching a.rs")
    );
    assert_eq!(
        inflight_short_label(&inflight_tool("my_custom_search", None)).as_deref(),
        Some("Running my_custom_search")
    );
    assert_eq!(
        inflight_short_label(&inflight_tool("read", Some("a.rs"))).as_deref(),
        Some("Reading a.rs")
    );
    assert_eq!(
        inflight_short_label(&inflight_tool("bash", None)).as_deref(),
        Some("Running")
    );
}

#[test]
fn user_row_highlights_dollar_skill_ref() {
    let mut model = UiModel::default();
    model.entries.push(UiEntry::User {
        text: "please run $demo now".into(),
    });
    let theme = LayoutTheme::product_dark();
    let skill = theme.palette().skill_ref;
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold::default(),
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let joined = lines.join("\n");
    let expect = bold(&fg_rgb(skill, "$demo"));
    assert!(
        joined.contains(&expect),
        "user row must paint skill_ref on $demo; got {joined:?}"
    );
}

fn assert_write_header_body_share_rail(done: bool, is_error: bool, expect: RgbColor) {
    let mut model = UiModel::default();
    model.entries.push(UiEntry::Tool {
        timeout_secs: None,
        id: "w1".into(),
        name: "write".into(),
        args_preview: "a.py".into(),
        tool_path: Some("a.py".into()),
        write_content: Some("line-a\nline-b\nline-c\n".into()),
        display_diff: None,
        output: String::new(),
        is_error,
        done,
    });
    let theme = LayoutTheme::product_dark();
    let rail = format!("\x1b[48;2;{};{};{}m", expect.r, expect.g, expect.b);
    let wash = {
        let p = theme.palette();
        let bg = if !done {
            p.tool_pending_bg
        } else if is_error {
            p.tool_error_bg
        } else {
            p.tool_success_bg
        };
        format!("\x1b[48;2;{};{};{}m", bg.r, bg.g, bg.b)
    };
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold::default(),
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let header = lines
        .iter()
        .find(|l| l.contains("Write") && l.contains("a.py"))
        .expect("header");
    let body = lines.iter().find(|l| l.contains("line-a")).expect("body");
    assert!(
        header.contains(&rail),
        "write header must share status rail"
    );
    assert!(
        !header.contains('⚙'),
        "tool header MUST NOT use gear glyph: {header}"
    );
    assert!(
        body.contains(&rail),
        "write body must share the same status rail"
    );
    assert!(
        !header.contains(&wash) || wash == rail,
        "write header MUST NOT use full tool-*-bg wash: {header}"
    );
}

#[test]
fn write_block_rails_header_and_body_together() {
    let p = LayoutTheme::product_dark().palette();
    assert_write_header_body_share_rail(false, false, mix_rgb(p.surface, p.accent, 0.72));
    assert_write_header_body_share_rail(true, false, mix_rgb(p.surface, p.success, 0.72));
    assert_write_header_body_share_rail(true, true, mix_rgb(p.surface, p.error, 0.72));
}

#[test]
fn edit_block_rails_header_and_diff_without_wash() {
    let mut model = UiModel::default();
    model.entries.push(UiEntry::Tool {
        timeout_secs: None,
        id: "e1".into(),
        name: "edit".into(),
        args_preview: "tmp/flow_test.py".into(),
        tool_path: Some("tmp/flow_test.py".into()),
        write_content: None,
        display_diff: Some(
            "@@ -4,3 +4,4 @@\n context-a\n-old line\n+new line\n context-b\n".into(),
        ),
        output: String::new(),
        is_error: false,
        done: true,
    });
    let theme = LayoutTheme::product_dark();
    let p = theme.palette();
    let rail = mix_rgb(p.surface, p.success, 0.72);
    let rail_bg = format!("\x1b[48;2;{};{};{}m", rail.r, rail.g, rail.b);
    let success_wash = format!(
        "\x1b[48;2;{};{};{}m",
        p.tool_success_bg.r, p.tool_success_bg.g, p.tool_success_bg.b
    );
    let added_row_bg = format!(
        "\x1b[48;2;{};{};{}m",
        p.diff_added_bg.r, p.diff_added_bg.g, p.diff_added_bg.b
    );
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold::default(),
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        100,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let header = lines
        .iter()
        .find(|l| l.contains("Edit") && l.contains("tmp/flow_test.py"))
        .expect("header");
    let body = lines
        .iter()
        .find(|l| l.contains("new line") || l.contains("+new"))
        .expect("diff body");
    assert!(
        header.contains(&rail_bg),
        "edit header must use success rail"
    );
    assert!(
        body.contains(&rail_bg),
        "edit diff body must share success rail (no naked split)"
    );
    assert!(
        !header.contains(&success_wash) || success_wash == rail_bg,
        "edit MUST NOT use tool-success-bg wash envelope"
    );
    assert!(
        !body.contains(&added_row_bg),
        "embedded edit MUST NOT stack diff-added-bg row tint; got {body:?}"
    );
}

#[test]
fn bash_full_output_footer_uses_warning_fg() {
    let mut model = UiModel::default();
    let mut output = String::new();
    for i in 0..20 {
        output.push_str(&format!("line-{i}\n"));
    }
    output.push_str("[Full output: /tmp/x.log. Truncated: 20 lines shown (50.0KB limit)]");
    model.entries.push(UiEntry::Bash {
        command: "big".into(),
        status: BashBlockStatus::Success,
        output,
        exclude_from_context: false,
    });
    let theme = LayoutTheme::product_dark();
    let warning = theme.palette().warning;
    let expect = bold(&fg_rgb(
        warning,
        "[Full output: /tmp/x.log. Truncated: 20 lines shown (50.0KB limit)]",
    ));
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold {
            tools_output_expanded: true,
            ..ScrollbackFold::default()
        },
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        120,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let joined = lines.join("\n");
    assert!(
        joined.contains(&expect),
        "Full output footer must use warning fg; got {joined:?}"
    );
    let plain = strip_ansi_local(&joined);
    assert!(
        plain.contains("expand disabled"),
        "hard-truncated must not offer ctrl+o expand; got {plain:?}"
    );
    assert!(
        !plain.contains("ctrl+o to expand"),
        "hard-truncated must not show expand hint"
    );
    assert!(
        !plain.contains("line-0"),
        "even with Ctrl+O fold on, hard-truncated must stay on tail; got {plain:?}"
    );
}

#[test]
fn write_viewport_defaults_to_tail_earlier() {
    let mut model = UiModel::default();
    let body: String = (0..18).map(|i| format!("line-{i}\n")).collect();
    model.entries.push(UiEntry::Tool {
        timeout_secs: None,
        id: "w1".into(),
        name: "write".into(),
        args_preview: "a.py".into(),
        tool_path: Some("a.py".into()),
        write_content: Some(body),
        display_diff: None,
        output: String::new(),
        is_error: false,
        done: false,
    });
    let theme = LayoutTheme::product_dark();
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold::default(),
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        100,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let plain = strip_ansi_local(&lines.join("\n"));
    assert!(
        plain.contains("earlier lines"),
        "write must use Tail earlier hint; got {plain:?}"
    );
    assert!(
        plain.contains("line-17"),
        "write viewport must show stream end; got {plain:?}"
    );
    let body_at = plain.find("line-17").expect("line-17 present");
    let hint_at = plain.find("earlier lines").expect("earlier hint present");
    assert!(
        hint_at > body_at,
        "earlier hint must be block footer after body; got {plain:?}"
    );
}

#[test]
fn huge_edit_diff_is_capped_in_scrollback() {
    let mut diff = String::new();
    for i in 0..300 {
        diff.push_str(&format!("     {i:>4} | +line-{i}\n"));
    }
    let mut model = UiModel::default();
    model.entries.push(UiEntry::Tool {
        timeout_secs: None,
        id: "e1".into(),
        name: "edit".into(),
        args_preview: "edit big.txt".into(),
        tool_path: Some("big.txt".into()),
        write_content: None,
        display_diff: Some(diff),
        output: String::new(),
        is_error: false,
        done: true,
    });
    let theme = LayoutTheme::product_dark();
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        theme,
        &ScrollbackFold::default(),
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        100,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let plain = strip_ansi_local(&lines.join("\n"));
    assert!(
        plain.contains("omitted") || plain.contains("capped"),
        "huge diff must show cap notice; got {plain:?}"
    );
    assert!(
        lines.len() < 120,
        "scrollback must not paint hundreds of diff rows; got {}",
        lines.len()
    );
}

#[test]
fn live_window_omits_planning_placeholder() {
    let mut model = UiModel::default();
    model.phase = UiPhase::Busy;
    model.entries.push(UiEntry::User { text: "go".into() });
    let mut hits = FoldHitTable::default();
    let lines = render_scrollback(
        &model,
        GlyphSet::from_env(),
        LayoutTheme::product_dark(),
        &ScrollbackFold::default(),
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut ScrollbackPaintCache::default(),
        &mut hits,
    );
    let plain = strip_ansi_local(&lines.join("\n"));
    assert!(
        !plain.contains("Planning next moves"),
        "busy chrome is status spinner, not a Planning placeholder: {plain}"
    );
    assert!(
        !plain.contains("Worked for"),
        "live window must not wrap current turn in Worked for: {plain}"
    );
    assert!(
        !hits
            .regions
            .iter()
            .any(|r| matches!(r.target, FoldTarget::LiveTail)),
        "no Planning live-tail hit: {:?}",
        hits.regions
    );
}

/// c1505 evidence: warm flatten (cache hit + extend) and upper-cache clone
/// scale with entry count — B-scroll flames miss this because `upper_gen` is
/// stable while scrolling (no `render_scrollback` re-entry).
#[test]
fn scrollback_warm_flatten_grows_with_entry_count() {
    use std::time::Instant;

    fn model_with_n(n: usize) -> UiModel {
        let mut model = UiModel::default();
        for i in 0..n {
            model.entries.push(UiEntry::Assistant {
                text: format!(
                    "entry-{i}: {}",
                    "这是一段用于 flatten 压力的中文与 `code` 混合正文。".repeat(6)
                ),
            });
        }
        model
    }

    fn warm_flatten_ns(n: usize, iters: u32) -> (usize, u128) {
        let model = model_with_n(n);
        let theme = LayoutTheme::product_dark();
        let fold = ScrollbackFold::default();
        let glyphs = GlyphSet::from_env();
        let mut cache = ScrollbackPaintCache::default();
        // Cold fill cache.
        let _cold = render_scrollback(
            &model,
            glyphs,
            theme,
            &fold,
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            100,
            &mut cache,
            &mut FoldHitTable::default(),
        );
        let t0 = Instant::now();
        let mut last_len = 0usize;
        for _ in 0..iters {
            let lines = render_scrollback(
                &model,
                glyphs,
                theme,
                &fold,
                &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
                100,
                &mut cache,
                &mut FoldHitTable::default(),
            );
            last_len = lines.len();
            // Simulate UiRoot upper_cache hit: clone full upper each frame.
            let _ = lines.clone();
        }
        (last_len, t0.elapsed().as_nanos() / u128::from(iters))
    }

    let iters = 40;
    let (len50, ns50) = warm_flatten_ns(50, iters);
    let (len200, ns200) = warm_flatten_ns(200, iters);
    let (len400, ns400) = warm_flatten_ns(400, iters);
    let report = format!(
        "c1505 flatten evidence: n=50 lines={len50} ~{ns50}ns/iter; \
         n=200 lines={len200} ~{ns200}ns; n=400 lines={len400} ~{ns400}ns\n"
    );
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/profile/c1505-flatten-microbench.txt");
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&out, &report);
    assert!(
        len200 > len50 * 2,
        "line count should track entries: {report}"
    );
    assert!(len400 > len200, "line count should keep growing: {report}");
    // Allow noise but require clear growth 50 → 400 (warm path).
    assert!(
        ns400 > ns50.saturating_mul(2),
        "warm flatten+clone should grow with history: {report}"
    );
}

fn strip_ansi_local(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for x in chars.by_ref() {
                    if x.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn tool_header_shows_timeout_only_when_requested() {
    let theme = LayoutTheme::product_dark();

    // Explicit model request → muted budget note before the key hint.
    let with = paint::paint_tool_header_line(theme, "▎", "Bash", "$ sleep 600", Some(600));
    let plain = strip_ansi_local(&with);
    let t = plain.find("(timeout 600s)").expect("timeout note present");
    let e = plain.find("(Alt+E)").expect("hint present");
    assert!(t < e, "timeout note must precede Alt+E hint: {plain:?}");

    // Tool default (omitted) → no chrome.
    let without = paint::paint_tool_header_line(theme, "▎", "Bash", "$ sleep 2", None);
    let plain2 = strip_ansi_local(&without);
    assert!(!plain2.contains("timeout"), "no default chrome: {plain2:?}");
}

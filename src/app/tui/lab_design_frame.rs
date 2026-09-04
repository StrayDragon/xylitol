//! Lab: export real product TUI frames for the designing shell view.
//!
//! Not a qa gate (`#[ignore]`). Run:
//! ```bash
//! cargo test -p xylitol --lib lab_design_frame_export -- --ignored --nocapture
//! # or: just export-design-frame
//! ```
//!
//! Writes `designing/generated/shell-frame.json` (span rows per frame) consumed
//! by `designing/app` — the shell view renders product truth, so refreshing the
//! view after UI changes means re-running this lab, not hand-editing mockups.
//!
//! Reuses the engine test `VirtualTerminal` (vte cell grid) via `#[path]`
//! include instead of copying it; if this lab outgrows lab status, promote that
//! module into `xylitol-tui` proper instead of duplicating it here.

#[cfg(test)]
#[path = "../../../packages/xylitol-tui/tests/support/mod.rs"]
mod vt_support;

#[cfg(test)]
mod lab_design_frame {
    use super::vt_support::{Color, VirtualTerminal};
    use crate::app::debug_fixtures::FixedZoneOp;
    use crate::app::debug_fixtures::seed_scene;
    use crate::app::tui::harness::enter_event;
    use crate::app::tui::host::{HostEvent, HostSession};
    use crate::infra::session::SessionManager;
    use crate::protocol::ports::XySessionStore;
    use xylitol_tui::InputEvent;
    use xylitol_tui::Palette;

    const COLS: u16 = 80;
    const FRAME_ROWS: u16 = 24;
    const SHORT_ROWS: u16 = 16;
    const MODEL_DISPLAY: &str = "Ornith-1.5-35B";
    const OUTPUT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/designing/generated/shell-frame.json"
    );

    type Vt = VirtualTerminal;

    fn product_session(vt: Vt) -> HostSession<Vt> {
        HostSession::new_product_ui_with_meta(vt, "~/x".into(), MODEL_DISPLAY.into())
    }

    /// Restored session with tool rows + answered Ask + activity envelopes.
    async fn seeded_session() -> Vec<crate::protocol::session::SessionEntry> {
        let mgr = SessionManager::in_memory();
        mgr.create("design-shell", Some("."), None)
            .await
            .expect("create");
        seed_scene(&mgr, "design-shell", "activity-fold-resume")
            .await
            .expect("seed activity-fold-resume");
        mgr.load_entries("design-shell").await.expect("load seed")
    }

    #[derive(serde::Serialize, Clone, PartialEq, Eq)]
    struct SpanOut {
        text: String,
        token: Option<&'static str>,
        fg: String,
        bg: Option<String>,
        bg_token: Option<&'static str>,
        rev: bool,
    }

    #[derive(serde::Serialize)]
    struct FrameOut {
        id: &'static str,
        rows: usize,
        lines: Vec<Vec<SpanOut>>,
    }

    #[derive(serde::Serialize)]
    struct ShellFrameDoc {
        cols: u16,
        generated_by: String,
        themes: serde_json::Value,
        frames: Vec<FrameOut>,
    }

    fn hex(rgb: (u8, u8, u8)) -> String {
        format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
    }

    /// RGB → DESIGN token, canonical word first: the palette has 5 same-color
    /// token pairs (a palette property, not a mapping bug), so the reverse map
    /// picks the discussion word and the raw hex rides along anyway.
    fn token_for(rgb: (u8, u8, u8)) -> Option<&'static str> {
        const TABLE: &[((u8, u8, u8), &str)] = &[
            ((0xcd, 0xd6, 0xf4), "on-surface"),
            ((0x6c, 0x70, 0x86), "muted"),
            ((0x89, 0xb4, 0xfa), "accent"),
            ((0xcb, 0xa6, 0xf7), "user"),
            ((0xa6, 0xe3, 0xa1), "success"),
            ((0xf3, 0x8b, 0xa8), "error"),
            ((0xf9, 0xe2, 0xaf), "warning"),
            ((0x1e, 0x2b, 0x22), "diff-added-bg"),
            ((0x2b, 0x1e, 0x24), "diff-removed-bg"),
            ((0x2d, 0x4a, 0x35), "diff-added-word-bg"),
            ((0x4a, 0x2d, 0x38), "diff-removed-word-bg"),
            ((0x1e, 0x1e, 0x2e), "surface"),
            // same-color collisions resolved above (palette facts):
            // assistant==on_surface, tool==diff_context==muted,
            // diff_added==success, diff_removed==error, user==skill_ref
        ];
        TABLE
            .iter()
            .find(|(candidate, _)| *candidate == rgb)
            .map(|(_, token)| *token)
    }

    fn color_rgb(color: Color) -> Option<(u8, u8, u8)> {
        match color {
            Color::Rgb(r, g, b) => Some((r, g, b)),
            _ => None,
        }
    }

    /// Loader glyph advances with wall-clock ticks; pin it so consecutive
    /// exports of the same fixture produce identical JSON.
    fn normalize_spinner(mut spans: Vec<SpanOut>) -> Vec<SpanOut> {
        for span in &mut spans {
            if let Some(first) = span.text.chars().next() {
                if ('\u{2800}'..='\u{28ff}').contains(&first) && span.text.contains("Working") {
                    span.text = format!("⠋{}", &span.text[first.len_utf8()..]);
                }
            }
        }
        spans
    }

    fn trim_trailing_blanks(mut spans: Vec<SpanOut>) -> Vec<SpanOut> {
        while let Some(last) = spans.last_mut() {
            let trimmed = last.text.trim_end_matches(' ').to_string();
            if trimmed.is_empty() {
                spans.pop();
            } else {
                last.text = trimmed;
                break;
            }
        }
        spans
    }

    fn row_spans(vt: &Vt, abs_row: usize) -> Vec<SpanOut> {
        let mut spans: Vec<SpanOut> = Vec::new();
        for col in 0..COLS {
            let mut cell = vt.cell(abs_row, col as usize);
            // vte 空单元格默认 '\0'；终端语义上是空格。
            if cell.ch == '\0' {
                cell.ch = ' ';
            }
            let fg = color_rgb(cell.fg).unwrap_or((0xcd, 0xd6, 0xf4));
            let bg = color_rgb(cell.bg);
            let rev = cell.reverse;
            let token = if rev { None } else { token_for(fg) };
            let fg_hex = hex(fg);
            let bg_hex = bg.map(hex);
            let bg_token = bg.and_then(token_for);
            match spans.last_mut() {
                Some(last)
                    if last.token == token
                        && last.fg == fg_hex
                        && last.bg == bg_hex
                        && last.bg_token == bg_token
                        && last.rev == rev =>
                {
                    last.text.push(cell.ch);
                }
                _ => spans.push(SpanOut {
                    text: cell.ch.to_string(),
                    token,
                    fg: fg_hex,
                    bg: bg_hex,
                    bg_token,
                    rev,
                }),
            }
        }
        normalize_spinner(trim_trailing_blanks(spans))
    }

    /// 规范 token（与 scripts/sync_tui_tokens.py TOKEN_TO_FIELD 同集）→ Palette 字段。
    const CANONICAL_TOKENS: &[(&str, &str)] = &[
        ("on-surface", "on_surface"),
        ("muted", "muted"),
        ("accent", "accent"),
        ("user", "user"),
        ("assistant", "assistant"),
        ("tool", "tool"),
        ("error", "error"),
        ("warning", "warning"),
        ("success", "success"),
        ("diff-added", "diff_added"),
        ("diff-removed", "diff_removed"),
        ("diff-context", "diff_context"),
        ("diff-added-bg", "diff_added_bg"),
        ("diff-removed-bg", "diff_removed_bg"),
        ("diff-added-word-bg", "diff_added_word_bg"),
        ("diff-removed-word-bg", "diff_removed_word_bg"),
        ("surface", "surface"),
        ("tool-pending-bg", "tool_pending_bg"),
        ("tool-success-bg", "tool_success_bg"),
        ("tool-error-bg", "tool_error_bg"),
        ("user-message-bg", "user_message_bg"),
        ("skill-ref", "skill_ref"),
    ];

    fn field_rgb(p: &Palette, field: &str) -> (u8, u8, u8) {
        let c = match field {
            "on_surface" => p.on_surface,
            "muted" => p.muted,
            "accent" => p.accent,
            "user" => p.user,
            "assistant" => p.assistant,
            "tool" => p.tool,
            "error" => p.error,
            "warning" => p.warning,
            "success" => p.success,
            "diff_added" => p.diff_added,
            "diff_removed" => p.diff_removed,
            "diff_context" => p.diff_context,
            "diff_added_bg" => p.diff_added_bg,
            "diff_removed_bg" => p.diff_removed_bg,
            "diff_added_word_bg" => p.diff_added_word_bg,
            "diff_removed_word_bg" => p.diff_removed_word_bg,
            "surface" => p.surface,
            "tool_pending_bg" => p.tool_pending_bg,
            "tool_success_bg" => p.tool_success_bg,
            "tool_error_bg" => p.tool_error_bg,
            "user_message_bg" => p.user_message_bg,
            "skill_ref" => p.skill_ref,
            _ => unreachable!("unknown token field"),
        };
        (c.r, c.g, c.b)
    }

    /// 双主题：dark 帧内已是 dark hex；light 由 span 的 token 反查亮色，
    /// 无 token 的 span（syntect / wash 混合色）保留原 hex 并按不同实现视角渐进同步。
    fn themes_doc() -> serde_json::Value {
        let dark = Palette::dark();
        let light = Palette::light();
        let mut tokens = serde_json::Map::new();
        for (token, field) in CANONICAL_TOKENS {
            tokens.insert(
                (*token).to_string(),
                serde_json::Value::String(hex(field_rgb(&light, field))),
            );
        }
        serde_json::json!({
            "dark": { "surface": hex(field_rgb(&dark, "surface")) },
            "light": {
                "surface": hex(field_rgb(&light, "surface")),
                "tokens": serde_json::Value::Object(tokens),
            },
        })
    }

    fn capture_frame(vt: &Vt, id: &'static str, rows: usize) -> FrameOut {
        let top = vt.scroll_buffer().len().saturating_sub(rows);
        let lines = (top..top + rows).map(|abs| row_spans(vt, abs)).collect();
        FrameOut { id, rows, lines }
    }

    #[tokio::test]
    #[ignore = "lab: export product frames for the designing shell view"]
    async fn lab_design_frame_export() {
        let skills: Vec<(String, String)> = vec![
            ("xylitol".into(), "本仓库开发手册".into()),
            ("rust-build-tune".into(), "构建调优手册".into()),
        ];

        // --- busy: restored transcript + Working + steering queue + 通知条 ---
        let entries = seeded_session().await;
        let vt = Vt::new(COLS, FRAME_ROWS);
        let mut session = product_session(vt);
        session.apply_cli_restored_session("design-shell", entries);
        session.on_run_started("帮我分析这段报错的原因，并给出修复方案");
        session
            .step(HostEvent::Input(InputEvent::Paste(
                "顺手把相关的测试也补上".into(),
            )))
            .expect("paste steering");
        session
            .step(HostEvent::Input(enter_event()))
            .expect("steer enter");
        session.apply_fixed_zone_op(FixedZoneOp::Toast);
        session.set_dollar_skill_catalog(skills.clone());
        session.tui.render_now().expect("busy render");
        let busy = capture_frame(&session.tui.terminal, "busy", FRAME_ROWS as usize);

        // --- idle: 同一会话，静止无 busy ---
        let entries = seeded_session().await;
        let vt = Vt::new(COLS, FRAME_ROWS);
        let mut session = product_session(vt);
        session.apply_cli_restored_session("design-shell", entries);
        session.set_dollar_skill_catalog(skills);
        session.tui.render_now().expect("idle render");
        let idle = capture_frame(&session.tui.terminal, "idle", FRAME_ROWS as usize);

        // --- short: 短终端 busy + models 槽（固定区预算可见） ---
        let entries = seeded_session().await;
        let vt = Vt::new(COLS, SHORT_ROWS);
        let mut session = product_session(vt);
        session.apply_cli_restored_session("design-shell", entries);
        session.on_run_started("切换一下模型对比输出风格");
        session.apply_fixed_zone_op(FixedZoneOp::SlotModels);
        session.tui.render_now().expect("short render");
        let short = capture_frame(&session.tui.terminal, "short", SHORT_ROWS as usize);

        let doc = ShellFrameDoc {
            cols: COLS,
            generated_by: "cargo test -p xylitol --lib lab_design_frame_export -- --ignored".into(),
            themes: themes_doc(),
            frames: vec![busy, idle, short],
        };
        let json = serde_json::to_string_pretty(&doc).expect("serialize");
        std::fs::write(OUTPUT, json).expect("write shell-frame.json");
        println!("wrote {OUTPUT}");
    }

    // --- qa-run contract probes for the pure helpers above ---

    #[test]
    fn lab_design_frame_token_reverse_prefers_canonical_word() {
        // 同色碰撞：assistant/on_surface 同 hex 时取规范讨论词 on-surface。
        assert_eq!(token_for((0xcd, 0xd6, 0xf4)), Some("on-surface"));
        assert_eq!(token_for((0x6c, 0x70, 0x86)), Some("muted"));
        assert_eq!(token_for((0xa6, 0xe3, 0xa1)), Some("success"));
        assert_eq!(token_for((0xde, 0xad, 0xbe)), None);
    }

    #[test]
    fn lab_design_frame_spinner_normalized_to_pinned_glyph() {
        let spans = vec![SpanOut {
            text: "⠹ Working".into(),
            token: Some("accent"),
            fg: "#89b4fa".into(),
            bg: None,
            bg_token: None,
            rev: false,
        }];
        let normalized = normalize_spinner(spans);
        assert_eq!(normalized[0].text, "⠋ Working");
    }
}

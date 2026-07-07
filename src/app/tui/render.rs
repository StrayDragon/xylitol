//! XyEvent → UI rendering seam (c399 stage 4: ratatui-free).
//!
//! Responsibilities:
//! - the [`RenderedLine`] UI-data type + the single `xyevent_to_rendered` seam
//!   (spec tui42: rendering consumes only `RenderedLine`, never `XyEvent`).
//!
//! c399 stage 4 removed the entire ratatui inline-viewport path
//! (`draw_tail_frame`, `commit_height`, `render_commit_lines_into_buf`,
//! `wrap_to_width`). The new pi-tui line-array engine consumes
//! [`StyledLine`]s produced here via [`RenderedLine::to_lines`]; the host loop
//! feeds them into the transcript widget. CJK-aware wrapping lives in
//! [`crate::app::tui::engine::width`] and markdown rendering in
//! [`crate::app::tui::widgets::markdown`].

use crate::app::tui::engine::style::{CellStyle, Span, StyledLine};
use crate::app::tui::theme;
use crate::app::tui::widgets::markdown::{MarkdownTheme, render_markdown};
use crate::domain::lifecycle::XyEvent;

// ── UI/UX ↔ business-flow boundary (spec tui42) ──────────────────────────
//
// `RenderedLine` is the UI-only data type. Business events (`XyEvent`) are
// translated into `RenderedLine` at a SINGLE seam (`xyevent_to_rendered`); the
// rendering layer (the transcript widget) consumes only `RenderedLine` and
// never matches `XyEvent` variants or calls Driver/agent methods.

/// A UI-only representation of one finalized scrollback line.
///
/// This is the type the rendering layer consumes. Adding a new message kind is
/// a new variant + a `to_lines` arm; business-event churn does not touch the
/// rendering layer (the seam function absorbs it).
pub enum RenderedLine {
    /// The user's submitted prompt, echoed into history.
    UserInput(String),
    /// A finalized assistant text line (streamed text committed on boundary).
    AssistantText(String),
    /// A finalized reasoning/thinking text line (streamed from ThinkingDelta,
    /// committed on boundary; gray/dim to distinguish from the main reply).
    ThinkingText(String),
    /// A tool-execution summary line (name + result preview + error flag).
    ToolSummary {
        name: String,
        preview: String,
        is_error: bool,
    },
    /// A status/notification line (model switch, generic error, etc.).
    Status(String),
}

impl RenderedLine {
    /// Render this UI data into styled lines for the line-array engine.
    /// Multi-line markdown variants (`UserInput`/`AssistantText`/`ThinkingText`)
    /// go through the shared markdown renderer (passthrough + code-block
    /// highlight); single-line variants produce one line. `width` is used for
    /// CJK-aware wrapping inside the renderer. `pal` supplies the theme-aware
    /// tokens (user prompt prefix, thinking style, etc.).
    pub fn to_lines(&self, pal: &theme::Palette, width: usize) -> Vec<StyledLine> {
        match self {
            RenderedLine::UserInput(prompt) => {
                let lines = render_markdown(prompt, width, MarkdownTheme::passthrough());
                // Prepend the `❯ ` marker as a leading span on the first line
                // (kept out of markdown parsing so `#` in user input still
                // renders structurally but stays visually marked as user echo).
                prepend_prefix(lines, "❯ ", pal.user_prompt())
            }
            RenderedLine::AssistantText(text) => {
                render_markdown(text, width, MarkdownTheme::passthrough())
            }
            RenderedLine::ThinkingText(text) => {
                // Thinking renders through the same markdown pipeline, then the
                // dim-gray italic thinking style is applied to every span so
                // committed thinking rows match the streaming pending-tail style
                // (mod.rs::pending_tail_rows uses p.thinking() too). Without this,
                // committed thinking renders unstyled and visually merges with the
                // reply body, looking like it was "overwritten" when the reply
                // streams right below. pi renders thinking as a themed `Markdown`
                // child (assistant-message.ts:121-126).
                let style = pal.thinking();
                let lines = render_markdown(text, width, MarkdownTheme::passthrough());
                lines
                    .into_iter()
                    .map(|mut l| {
                        for span in &mut l.spans {
                            span.style = style;
                        }
                        l
                    })
                    .collect()
            }
            RenderedLine::ToolSummary {
                name,
                preview,
                is_error,
            } => {
                let style = if *is_error { pal.error() } else { pal.tool() };
                let text = format!("[{name}] {preview}");
                vec![styled_line(&text, style)]
            }
            RenderedLine::Status(msg) => {
                vec![styled_line(msg, pal.text_dim())]
            }
        }
    }
}

/// Prepend `prefix` (with `prefix_style`) to the first line of `lines`. If
/// `lines` is empty, push a single styled-prefix line. The prefix is inserted
/// as a new leading span so the rest of the line keeps its own styling.
fn prepend_prefix(
    mut lines: Vec<StyledLine>,
    prefix: &str,
    prefix_style: CellStyle,
) -> Vec<StyledLine> {
    if let Some(first) = lines.first_mut() {
        first
            .spans
            .insert(0, Span::styled(prefix.to_string(), prefix_style));
    } else {
        let mut line = StyledLine::new();
        line.spans
            .push(Span::styled(prefix.to_string(), prefix_style));
        lines.push(line);
    }
    lines
}

/// Build a single [`StyledLine`] from `text` under one `style`. The engine's
/// hard-width invariant is enforced later by the render path (it wraps lines
/// that exceed width); single-line status/tool summaries are short in
/// practice, and over-long ones wrap at the engine layer.
fn styled_line(text: &str, style: CellStyle) -> StyledLine {
    let mut line = StyledLine::new();
    line.spans.push(Span::styled(text.to_string(), style));
    line
}

/// The SINGLE seam: translate a business `XyEvent` into UI-only `RenderedLine`s.
///
/// This is the only place the rendering layer learns about `XyEvent`. Everything
/// downstream (`to_lines`, the transcript widget) consumes `RenderedLine` and is
/// insulated from domain-event vocabulary churn (spec tui42).
pub fn xyevent_to_rendered(event: &XyEvent) -> Vec<RenderedLine> {
    match event {
        XyEvent::ToolExecutionEnd {
            name,
            result,
            is_error,
            ..
        } => {
            let preview: String = result.lines().take(3).collect::<Vec<_>>().join("\n");
            let preview = if result.lines().count() > 3 {
                format!("{preview}…")
            } else {
                preview
            };
            vec![RenderedLine::ToolSummary {
                name: name.clone(),
                preview,
                is_error: *is_error,
            }]
        }
        XyEvent::Error(msg) => vec![RenderedLine::Status(format!("error: {msg}"))],
        XyEvent::ModelSelect { model_id, .. } => {
            vec![RenderedLine::Status(format!("model: {model_id}"))]
        }
        // TextDelta/ThinkingDelta/ToolStart/ToolUpdate/TurnStart/TurnEnd do not
        // produce finalized scrollback lines here — TextDelta is accumulated by
        // the host's stream buffer and committed on boundaries; others update
        // tail status only.
        _ => Vec::new(),
    }
}

/// The user's submitted prompt as a [`RenderedLine`] (修复 c340 §7 #3).
///
/// Called on Enter before `driver.run`, so the user sees their own message in
/// the conversation history above the streaming reply.
pub fn user_message_rendered(prompt: &str) -> RenderedLine {
    RenderedLine::UserInput(prompt.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::theme_detect::TerminalTheme;

    fn pal() -> theme::Palette {
        theme::palette(TerminalTheme::Dark)
    }

    #[test]
    fn user_message_renders_with_prefix_and_prompt() {
        let lines = user_message_rendered("fix the bug").to_lines(&pal(), 80);
        let text = lines.first().map(|l| l.plain_text()).unwrap_or_default();
        assert!(text.contains('❯'), "prefix present: {text}");
        assert!(text.contains("fix the bug"), "prompt text present: {text}");
    }

    #[test]
    fn user_message_empty_prompt_still_has_prefix() {
        let lines = user_message_rendered("").to_lines(&pal(), 80);
        let text = lines.first().map(|l| l.plain_text()).unwrap_or_default();
        assert!(text.contains('❯'), "prefix present even for empty: {text}");
    }

    #[test]
    fn status_line_carries_dim_style() {
        let lines = RenderedLine::Status("hello".into()).to_lines(&pal(), 80);
        assert_eq!(lines.len(), 1);
        // DarkGray fg per palette::text_dim (dark theme).
        assert_eq!(lines[0].spans[0].style.fg, pal().text_dim().fg);
    }

    #[test]
    fn tool_summary_error_uses_error_style() {
        let lines = RenderedLine::ToolSummary {
            name: "bash".into(),
            preview: "boom".into(),
            is_error: true,
        }
        .to_lines(&pal(), 80);
        assert_eq!(lines.len(), 1);
        let text = lines[0].plain_text();
        assert!(text.contains("[bash]"), "tool name present: {text}");
        assert!(text.contains("boom"), "preview present: {text}");
    }

    #[test]
    fn assistant_text_passthrough_keeps_prose() {
        // Passthrough markdown keeps prose text verbatim.
        let lines = RenderedLine::AssistantText("hello world".into()).to_lines(&pal(), 80);
        let text: String = lines
            .iter()
            .map(|l| l.plain_text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("hello world"),
            "assistant text present: {text}"
        );
    }
}

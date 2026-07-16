//! Codex-style startup card: minimal meta card (c1135 polish).

use xylitol_tui::{fg_rgb, truncate_to_width, visible_width};

use crate::app::core::driver::LoadedResourcesSnapshot;
use crate::app::tui::layout::LayoutTheme;

/// Accent for `>_` (logo stroke `#38BDF8`).
const ACCENT_RGB: (u8, u8, u8) = (0x38, 0xbd, 0xf8);

/// Render product startup header as a minimal Codex-like bordered card.
///
/// ```text
/// ╭──────────────────────────────────────────────╮
/// │ >_ xylitol (v…)                              │
/// │                                              │
/// │ model:     ornith                            │
/// │ directory: ~/…                               │
/// │ skills(N): a · b · c                         │
/// │ mcp:       2 connected · …                   │
/// ╰──────────────────────────────────────────────╯
/// ```
///
/// - Skills / MCP wrap inside the card — **no `...` truncation**.
/// - Every line is padded to exact `width` (helps differential resize).
/// - MUST NOT list prompt templates / 「木糖醇」文案 / ASCII logo art.
pub fn render_loaded_resources(
    theme: LayoutTheme,
    snap: &LoadedResourcesSnapshot,
    cwd: &str,
    model: &str,
    width: usize,
) -> Vec<String> {
    let w = width.max(1);
    if w < 8 {
        return vec![truncate_to_width("xylitol", w, "", true)];
    }

    let inner = w.saturating_sub(2);
    let version = env!("CARGO_PKG_VERSION");
    let mut meta: Vec<String> = Vec::new();
    meta.push(title_line(theme, version, inner));
    meta.push(String::new());
    meta.extend(field_lines(
        theme,
        "model",
        model,
        theme.palette().on_surface,
        inner,
    ));
    meta.extend(field_lines(
        theme,
        "directory",
        cwd,
        theme.palette().muted,
        inner,
    ));

    if !snap.skill_names.is_empty() {
        let body = snap.skill_names.join(" · ");
        let label = format!("skills({})", snap.skill_names.len());
        meta.extend(field_lines(
            theme,
            &label,
            &body,
            theme.palette().skill_ref,
            inner,
        ));
    }

    let show_mcp = snap.mcp_configured > 0
        || !snap.mcp_connected.is_empty()
        || !snap.mcp_diag_short.is_empty();
    if show_mcp {
        let ids: Vec<String> = snap
            .mcp_connected
            .iter()
            .map(|(id, n)| format!("{id}({n})"))
            .collect();
        let mut body = if ids.is_empty() {
            format!("{} configured · 0 connected", snap.mcp_configured)
        } else {
            format!(
                "{} connected · {}",
                snap.mcp_connected.len(),
                ids.join(" · ")
            )
        };
        if !snap.mcp_diag_short.is_empty() {
            body.push_str(" · ");
            body.push_str(&snap.mcp_diag_short.join("; "));
        }
        meta.extend(field_lines(
            theme,
            "mcp",
            &body,
            theme.palette().success,
            inner,
        ));
    }

    let border = |s: &str| theme.paint_muted(s);
    let mut out = Vec::with_capacity(meta.len() + 2);
    out.push(fit_exact(
        &format!(
            "{}{}{}",
            border("╭"),
            border(&"─".repeat(inner)),
            border("╮")
        ),
        w,
    ));
    for row in meta {
        let inner_line = truncate_to_width(&row, inner, "", true);
        out.push(fit_exact(
            &format!("{}{}{}", border("│"), inner_line, border("│")),
            w,
        ));
    }
    out.push(fit_exact(
        &format!(
            "{}{}{}",
            border("╰"),
            border(&"─".repeat(inner)),
            border("╯")
        ),
        w,
    ));
    out
}

fn title_line(theme: LayoutTheme, version: &str, inner_w: usize) -> String {
    let (r, g, b) = ACCENT_RGB;
    let prompt = fg_rgb(xylitol_tui::terminal_colors::RgbColor { r, g, b }, ">_");
    let name = fg_rgb(theme.palette().on_surface, " xylitol");
    let ver = theme.paint_muted(&format!(" ({version})"));
    truncate_to_width(&format!("{prompt}{name}{ver}"), inner_w, "", true)
}

fn field_lines(
    theme: LayoutTheme,
    label: &str,
    value: &str,
    value_rgb: xylitol_tui::terminal_colors::RgbColor,
    inner_w: usize,
) -> Vec<String> {
    let label_col = 11usize;
    let label_txt = format!("{label}:");
    let label_pad = format!(
        "{label_txt}{pad}",
        pad = " ".repeat(label_col.saturating_sub(visible_width(&label_txt)))
    );
    let label_s = theme.paint_muted(&label_pad);
    let body_w = inner_w.saturating_sub(label_col).max(4);
    let wrapped = wrap_plain(value, body_w);
    let indent = " ".repeat(label_col);
    let mut out = Vec::with_capacity(wrapped.len().max(1));
    for (i, chunk) in wrapped.into_iter().enumerate() {
        let line = if i == 0 {
            format!("{label_s}{}", fg_rgb(value_rgb, &chunk))
        } else {
            format!("{indent}{}", fg_rgb(value_rgb, &chunk))
        };
        out.push(truncate_to_width(&line, inner_w, "", true));
    }
    out
}

fn fit_exact(line: &str, width: usize) -> String {
    truncate_to_width(line, width, "", true)
}

fn wrap_plain(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for token in text.split_whitespace() {
        let tw = visible_width(token);
        let gap = usize::from(!cur.is_empty());
        if !cur.is_empty() && cur_w + gap + tw > max_width {
            lines.push(std::mem::take(&mut cur));
            cur_w = 0;
        }
        if !cur.is_empty() {
            cur.push(' ');
            cur_w += 1;
        }
        if tw > max_width {
            let mut rest = token;
            while visible_width(rest) > max_width {
                let mut end = rest.len();
                while visible_width(&rest[..end]) > max_width {
                    end = rest.char_indices().nth_back(1).map(|(i, _)| i).unwrap_or(0);
                    if end == 0 {
                        break;
                    }
                }
                if end == 0 {
                    break;
                }
                lines.push(rest[..end].to_string());
                rest = &rest[end..];
            }
            if !rest.is_empty() {
                cur.push_str(rest);
                cur_w = visible_width(rest);
            }
        } else {
            cur.push_str(token);
            cur_w += tw;
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::layout::LayoutTheme;

    #[test]
    fn card_has_box_and_meta() {
        let lines = render_loaded_resources(
            LayoutTheme::product_dark(),
            &LoadedResourcesSnapshot::default(),
            "~/proj",
            "ornith",
            72,
        );
        let joined = lines.join("\n");
        assert!(joined.contains("╭") && joined.contains("╰"), "{joined}");
        assert!(joined.contains("xylitol"), "{joined}");
        assert!(!joined.contains("木糖醇"), "{joined}");
        assert!(
            joined.contains("model") && joined.contains("ornith"),
            "{joined}"
        );
        assert!(
            joined.contains("directory") && joined.contains("~/proj"),
            "{joined}"
        );
        assert!(
            !joined.contains("✦") && !joined.contains("◕"),
            "no ascii logo"
        );
        assert!(!joined.contains("C5H12O5"), "{joined}");
        for line in &lines {
            assert_eq!(visible_width(line), 72, "{line:?}");
        }
    }

    #[test]
    fn skills_wrap_without_ellipsis_shows_all() {
        let snap = LoadedResourcesSnapshot {
            skill_names: (0..16).map(|i| format!("skill-name-{i:02}")).collect(),
            ..Default::default()
        };
        let lines = render_loaded_resources(LayoutTheme::product_dark(), &snap, "~/x", "m", 72);
        let joined = lines.join("\n");
        assert!(joined.contains("skills"));
        assert!(joined.contains("skill-name-00"));
        assert!(joined.contains("skill-name-15"));
        assert!(!joined.contains("..."));
    }

    #[test]
    fn mcp_inside_card() {
        let snap = LoadedResourcesSnapshot {
            mcp_connected: vec![("fs".into(), 3), ("git".into(), 1)],
            mcp_configured: 2,
            ..Default::default()
        };
        let lines = render_loaded_resources(LayoutTheme::product_dark(), &snap, "~/x", "m", 80);
        let joined = lines.join("\n");
        assert!(joined.contains("mcp"));
        assert!(joined.contains("fs(3)"));
        assert!(joined.contains("git(1)"));
        assert!(!joined.contains("..."));
    }
}

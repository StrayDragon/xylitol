//! Product-only composites assembled from `xylitol_tui` atoms.
//!
//! Not a second component library — no differential engine, no generic Editor.
//! Keep atoms in the package; put product scrollback / queue strip / glyphs here.

mod glyphs;
mod loaded_resources;
mod queue;
mod scrollback;

pub use glyphs::GlyphSet;
pub use loaded_resources::render_loaded_resources;
pub use queue::render_queue_strip;
#[cfg(test)]
pub use scrollback::find_stable_markdown_prefix_end;
pub use scrollback::{ScrollbackFold, ScrollbackPaintCache, render_scrollback};

use crate::protocol::model::{ThinkingLevel, TokenProvenance};

/// Compact token/window counts for footer (pi `formatTokens`).
pub fn format_compact_tokens(count: u64) -> String {
    if count < 1_000 {
        count.to_string()
    } else if count < 10_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else if count < 1_000_000 {
        format!("{}k", (count as f64 / 1_000.0).round() as u64)
    } else if count < 10_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else {
        format!("{}M", (count as f64 / 1_000_000.0).round() as u64)
    }
}

/// Provenance-honest footer fragment (`used C tokens` / `~C` / `?`), plus derived
/// `p%/window` when `context_window > 0` (c1680). Count `C` reuses [`format_compact_tokens`]
/// (c1820); Unknown stays `?` without compact.
pub fn footer_token_label(provenance: TokenProvenance, tokens: u64, context_window: u64) -> String {
    let base = match provenance {
        TokenProvenance::Api | TokenProvenance::RemoteCount | TokenProvenance::LocalTokenizer => {
            format!("used {} tokens", format_compact_tokens(tokens))
        }
        TokenProvenance::Heuristic => {
            format!("used ~{} tokens", format_compact_tokens(tokens))
        }
        TokenProvenance::Unknown => "used ? tokens".into(),
    };
    if context_window == 0 {
        return base;
    }
    let win = format_compact_tokens(context_window);
    match provenance {
        TokenProvenance::Unknown => format!("{base} · ?%/{win}"),
        TokenProvenance::Heuristic => {
            let pct = tokens as f64 / context_window as f64 * 100.0;
            format!("{base} · ~{pct:.1}%/{win}")
        }
        TokenProvenance::Api | TokenProvenance::RemoteCount | TokenProvenance::LocalTokenizer => {
            let pct = tokens as f64 / context_window as f64 * 100.0;
            format!("{base} · {pct:.1}%/{win}")
        }
    }
}

/// Footer thinking-level label (`thinking off` for Off, else `as_str`).
pub fn footer_thinking_label(level: ThinkingLevel) -> String {
    if level == ThinkingLevel::Off {
        "thinking off".into()
    } else {
        level.as_str().to_string()
    }
}

/// Footer identity line (`cwd · model · {thinking}`, optional queue / tokens).
/// Empty `thinking_label` omits the thinking segment (no-thinking models).
pub fn format_footer_text(
    cwd: &str,
    model: &str,
    thinking_label: &str,
    steer: usize,
    follow_up: usize,
    token_label: Option<&str>,
) -> String {
    let mut base = if thinking_label.is_empty() {
        format!("{cwd} · {model}")
    } else {
        format!("{cwd} · {model} · {thinking_label}")
    };
    if let Some(tok) = token_label.filter(|s| !s.is_empty()) {
        base = format!("{base} · {tok}");
    }
    if steer > 0 || follow_up > 0 {
        base = format!("q:s{steer}|f{follow_up} · {base}");
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::model::TokenProvenance;

    #[test]
    fn format_compact_tokens_boundaries() {
        assert_eq!(format_compact_tokens(999), "999");
        assert_eq!(format_compact_tokens(1_000), "1.0k");
        assert_eq!(format_compact_tokens(8_000), "8.0k");
        assert_eq!(format_compact_tokens(128_000), "128k");
        assert_eq!(format_compact_tokens(1_000_000), "1.0M");
    }

    #[test]
    fn footer_token_label_provenance() {
        assert_eq!(
            footer_token_label(TokenProvenance::Api, 42, 0),
            "used 42 tokens"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::RemoteCount, 7, 0),
            "used 7 tokens"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::LocalTokenizer, 9, 0),
            "used 9 tokens"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::Heuristic, 100, 0),
            "used ~100 tokens"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::Unknown, 0, 0),
            "used ? tokens"
        );
    }

    #[test]
    fn footer_token_label_derived_percent() {
        assert_eq!(
            footer_token_label(TokenProvenance::Api, 42_000, 128_000),
            "used 42k tokens · 32.8%/128k"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::Heuristic, 100, 128_000),
            "used ~100 tokens · ~0.1%/128k"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::Heuristic, 42_000, 128_000),
            "used ~42k tokens · ~32.8%/128k"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::Unknown, 0, 128_000),
            "used ? tokens · ?%/128k"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::Api, 100, 0),
            "used 100 tokens"
        );
    }

    #[test]
    fn footer_thinking_label_off_and_levels() {
        assert_eq!(footer_thinking_label(ThinkingLevel::Off), "thinking off");
        assert_eq!(footer_thinking_label(ThinkingLevel::Medium), "medium");
        assert_eq!(footer_thinking_label(ThinkingLevel::Xhigh), "xhigh");
    }

    #[test]
    fn format_footer_text_field_order() {
        assert_eq!(
            format_footer_text("~/x", "m", "thinking off", 0, 0, None),
            "~/x · m · thinking off"
        );
        assert_eq!(
            format_footer_text("~/x", "m", "medium", 0, 0, Some("used 3 tokens")),
            "~/x · m · medium · used 3 tokens"
        );
        assert_eq!(
            format_footer_text("~/x", "m", "high", 1, 2, Some("used ~4 tokens")),
            "q:s1|f2 · ~/x · m · high · used ~4 tokens"
        );
    }
}

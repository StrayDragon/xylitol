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
pub use scrollback::{ScrollbackFold, render_scrollback};

use crate::protocol::types::{ThinkingLevel, TokenProvenance};

/// Provenance-honest footer fragment (`used N tokens` / `~N` / `?`).
pub fn footer_token_label(provenance: TokenProvenance, tokens: u64) -> String {
    match provenance {
        TokenProvenance::Api | TokenProvenance::RemoteCount | TokenProvenance::LocalTokenizer => {
            format!("used {tokens} tokens")
        }
        TokenProvenance::Heuristic => format!("used ~{tokens} tokens"),
        TokenProvenance::Unknown => "used ? tokens".into(),
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

/// Footer identity line (`cwd · model · • {thinking}`, optional queue / tokens).
pub fn format_footer_text(
    cwd: &str,
    model: &str,
    thinking_label: &str,
    steer: usize,
    follow_up: usize,
    token_label: Option<&str>,
) -> String {
    let mut base = format!("{cwd} · {model} · • {thinking_label}");
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
    use crate::protocol::types::TokenProvenance;

    #[test]
    fn footer_token_label_provenance() {
        assert_eq!(
            footer_token_label(TokenProvenance::Api, 42),
            "used 42 tokens"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::RemoteCount, 7),
            "used 7 tokens"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::LocalTokenizer, 9),
            "used 9 tokens"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::Heuristic, 100),
            "used ~100 tokens"
        );
        assert_eq!(
            footer_token_label(TokenProvenance::Unknown, 0),
            "used ? tokens"
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
            "~/x · m · • thinking off"
        );
        assert_eq!(
            format_footer_text("~/x", "m", "medium", 0, 0, Some("used 3 tokens")),
            "~/x · m · • medium · used 3 tokens"
        );
        assert_eq!(
            format_footer_text("~/x", "m", "high", 1, 2, Some("used ~4 tokens")),
            "q:s1|f2 · ~/x · m · • high · used ~4 tokens"
        );
    }
}

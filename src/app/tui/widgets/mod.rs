//! Product-only composites assembled from `xylitol_tui` atoms.
//!
//! Not a second component library — no differential engine, no generic Editor.
//! Keep atoms in the package; put product scrollback / queue strip / glyphs here.

mod glyphs;
mod queue;
mod scrollback;

pub use glyphs::GlyphSet;
pub use queue::render_queue_strip;
pub use scrollback::{ScrollbackFold, render_scrollback};

/// Footer identity line (`cwd · model`, optional queue badge prefix).
pub fn format_footer_text(cwd: &str, model: &str, steer: usize, follow_up: usize) -> String {
    let mut base = format!("{cwd} · {model}");
    if steer > 0 || follow_up > 0 {
        base = format!("q:s{steer}|f{follow_up} · {base}");
    }
    base
}

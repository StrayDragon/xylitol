//! Line-based differential renderer.
//!
//! Compares old and new line arrays and produces minimal ANSI output to update
//! the terminal.  Equivalent to pi's differential render in `tui.ts` (doRender).

use crate::interface::tui::engine::ansi;

/// Result of comparing old and new line buffers.
#[derive(Debug, Clone)]
pub(crate) struct DiffResult {
    /// Index of the first changed line (inclusive).
    pub(crate) first_changed: usize,
    /// Index of the last changed line (inclusive).
    pub(crate) last_changed: usize,
    /// Whether the content grew (new lines appended).
    pub(crate) appended: bool,
}

/// Compute the diff between old and new line arrays.
///
/// Returns the range of changed lines.  If nothing changed, returns
/// `DiffResult` where `first_changed > last_changed`.
pub(crate) fn compute_diff(old: &[String], new: &[String]) -> DiffResult {
    let max_len = old.len().max(new.len());
    let mut first = max_len;
    let mut last = 0;
    let mut any_change = false;

    for i in 0..max_len {
        let a = old.get(i).map(|s| s.as_str()).unwrap_or("");
        let b = new.get(i).map(|s| s.as_str()).unwrap_or("");
        if a != b {
            if !any_change {
                first = i;
                any_change = true;
            }
            last = i;
        }
    }

    let appended = new.len() > old.len() && first == old.len();

    DiffResult {
        first_changed: if any_change { first } else { max_len },
        last_changed: if any_change { last } else { 0 },
        appended,
    }
}

/// Build the ANSI escape sequence output to update the terminal from `old` to `new`.
///
/// `prev_viewport_top` is the number of lines scrolled above the visible area
/// (used for cursor arithmetic).  For now, we assume viewport_top = 0 (all content visible).
pub(crate) fn build_diff_output(
    old: &[String],
    new: &[String],
    cursor_row: u16,
    _viewport_top: u16,
) -> Vec<u8> {
    let diff = compute_diff(old, new);

    if diff.first_changed > diff.last_changed {
        // No changes — return empty
        return Vec::new();
    }

    let mut output = String::new();

    // Begin synchronized output (avoid flicker)
    output.push_str(ansi::begin_sync());

    let target_row = diff.first_changed as u16;

    // Move cursor to the first changed line
    if target_row != cursor_row {
        if target_row > cursor_row {
            output.push_str(&ansi::cursor_down(target_row - cursor_row));
        } else if cursor_row > target_row {
            output.push_str(&ansi::cursor_up(cursor_row - target_row));
        }
    }

    // Write each changed line
    for i in diff.first_changed..=diff.last_changed {
        let line_idx = (i - diff.first_changed) as u16;
        if line_idx > 0 {
            output.push_str("\r\n");
        }
        output.push_str(ansi::erase_line());

        if let Some(line) = new.get(i) {
            output.push_str(line);
        }
    }

    // If content shrank, clear remaining lines
    if new.len() < old.len() {
        let extra = old.len() - new.len();
        for _ in 0..extra {
            output.push_str("\r\n");
            output.push_str(ansi::erase_line());
        }
        // Move cursor back
        if extra > 0 {
            output.push_str(&ansi::cursor_up(extra as u16));
        }
    }

    // End synchronized output
    output.push_str(ansi::end_sync());

    output.into_bytes()
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_no_change() {
        let old = vec!["a".into(), "b".into()];
        let new = vec!["a".into(), "b".into()];
        let d = compute_diff(&old, &new);
        assert!(d.first_changed > d.last_changed);
    }

    #[test]
    fn test_diff_one_line_changed() {
        let old = vec!["a".into(), "b".into(), "c".into()];
        let new = vec!["a".into(), "X".into(), "c".into()];
        let d = compute_diff(&old, &new);
        assert_eq!(d.first_changed, 1);
        assert_eq!(d.last_changed, 1);
    }

    #[test]
    fn test_diff_appended() {
        let old = vec!["a".into(), "b".into()];
        let new = vec!["a".into(), "b".into(), "c".into()];
        let d = compute_diff(&old, &new);
        assert_eq!(d.first_changed, 2);
        assert_eq!(d.last_changed, 2);
        assert!(d.appended);
    }

    #[test]
    fn test_diff_all_changed() {
        let old = vec!["a".into()];
        let new = vec!["b".into()];
        let d = compute_diff(&old, &new);
        assert_eq!(d.first_changed, 0);
        assert_eq!(d.last_changed, 0);
    }

    #[test]
    fn test_diff_shrunk() {
        let old = vec!["a".into(), "b".into(), "c".into()];
        let new = vec!["a".into()];
        let d = compute_diff(&old, &new);
        assert_eq!(d.first_changed, 1);
        assert_eq!(d.last_changed, 2);
    }

    #[test]
    fn test_output_no_change_empty() {
        let old = vec!["a".into()];
        let new = vec!["a".into()];
        let out = build_diff_output(&old, &new, 0, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn test_output_one_change() {
        let old = vec!["hello".into()];
        let new = vec!["world".into()];
        let out = build_diff_output(&old, &new, 0, 0);
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("world"));
        assert!(s.contains(ansi::begin_sync()));
        assert!(s.contains(ansi::end_sync()));
    }
}

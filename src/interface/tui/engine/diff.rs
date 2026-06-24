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
    _cursor_row: u16,
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

    // Write each changed line using ABSOLUTE cursor positioning (cursor_goto).
    // We deliberately do NOT use relative cursor_up/down + \r\n here: relative
    // positioning drifts when the hardware cursor and the assumed baseline
    // (the old `cursor_row` argument) disagree, which produced ghosting where
    // a new composer line was written over the wrong row and stale text from
    // the previous frame lingered. Absolute positioning per row is robust to
    // any prior cursor state — the same approach pi's doRender uses.
    for i in diff.first_changed..=diff.last_changed {
        // Terminal rows are 1-based in cursor_goto.
        output.push_str(&ansi::cursor_goto((i as u16) + 1, 1));
        output.push_str(ansi::erase_line());
        if let Some(line) = new.get(i) {
            output.push_str(line);
        }
    }

    // If content shrank, clear the now-empty trailing lines absolutely too.
    if new.len() < old.len() {
        for i in new.len()..old.len() {
            output.push_str(&ansi::cursor_goto((i as u16) + 1, 1));
            output.push_str(ansi::erase_line());
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

    /// r5 no-leftover (regression): the diff writer MUST position each changed
    /// line ABSOLUTELY (cursor_goto) so a stale/wrong `cursor_row` baseline
    /// cannot move writes to the wrong row. The old relative-positioning
    /// implementation drifted when the baseline disagreed with the real
    /// hardware cursor, ghosting composer lines. Passing a deliberately wrong
    /// cursor_row here must not affect WHERE lines are written.
    #[test]
    fn test_diff_ignores_stale_cursor_row_baseline() {
        let old = vec!["keep".into(), "old composer".into()];
        let new = vec!["keep".into(), "new composer".into()];
        // Pass a wildly wrong cursor_row baseline (5 vs the changed row 1).
        let out = build_diff_output(&old, &new, 5, 0);
        let s = String::from_utf8(out).unwrap();
        // Must address row 2 absolutely (cursor_goto uses 1-based rows).
        assert!(
            s.contains(&ansi::cursor_goto(2, 1)),
            "changed line 1 must be written with absolute cursor_goto(2,1), \
             regardless of the cursor_row baseline; got: {s:?}"
        );
        assert!(s.contains("new composer"));
        // Must not contain any relative up/down movement sequences.
        assert!(!s.contains("\u{1b}[5A") && !s.contains("\u{1b}[5B"));
    }
}

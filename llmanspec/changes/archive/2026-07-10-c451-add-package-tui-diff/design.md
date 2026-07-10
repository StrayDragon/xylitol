# Design — c451-add-package-tui-diff

## Approach

Mirror `Markdown` + theme closures. Core free function `render_diff_lines` + `Component` wrapper with width cache. Diff engine: `similar` in-package (same crate as main `patch.rs`, but **no** dependency on main crate).

UX SSOT: `src/app/tui/design/diff-block.md`（c449）。

## API sketch

```rust
pub enum DiffInput {
    DisplayText(String),
    UnifiedText(String),
    LinePair { old: String, new: String, path: Option<String> },
}

pub struct DiffTheme { /* added/removed/context/gutter/meta/word_change closures */ }
pub struct DiffOptions {
    pub word_level: bool,
    pub side_by_side_min_width: Option<usize>, // None = never; Some(100) = DESIGN default
}
```

## Word-level

Only for exactly one adjacent Delete+Insert pair; use `TextDiff::from_words` (or chars for CJK-heavy lines). Multi-line hunks stay line-colored only.

## Side-by-side

When `width >= side_by_side_min_width`, split into two columns with a single space gutter (no box drawing). Otherwise unified.

## Demo

`TranscriptEntry::Diff { expanded, summary, display_diff }` — collapsed one line; expanded renders `Diff` component.

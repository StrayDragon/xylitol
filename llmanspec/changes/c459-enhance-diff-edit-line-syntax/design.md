# Design — c459-enhance-diff-edit-line-syntax

## Decisions

1. **No new diff crate** — keep `similar` + optional `syntect`/`two-face` via existing `highlight` feature. Reject `imara-diff`/`diffy` (overlap) and `deff`/`asd`/`git-ui` (ratatui apps, not embeddable Component APIs).

2. **`DiffInput::EditText`** — parse pi `generateDiffString` lines with `^([+-\s])(\s*\d*)\s(.*)$`. Meta/`...` ellipsis rows stay as `LineKind::Meta` or Equal with empty content as appropriate.

3. **Compact edit gutter (unified)** — for EditText (and when `DiffOptions::compact_line_numbers` is true), render `±{pad}{num} {content}` instead of dual old/new gutter. LinePair/UnifiedText keep dual gutter unless option set.

4. **SBS line numbers** — each cell: `{sign}{pad}{num} {content}` using that side's `old_no`/`new_no`. Pad width = max decimal width of all line numbers in the normalized set. Empty half-column = spaces only.

5. **`DiffTheme::highlight_line`** — `Box<dyn Fn(&str) -> String>`, default identity. Applied to raw content **before** kind coloring when set; callers under `highlight` feature may inject syntect. Package MUST NOT require syntect for Diff.

6. **Demo** — seed one collapsed EditText block; SBS/unified LinePair samples remain. Expand via Alt+E.

## Alternatives rejected

- Replace Diff with an external TUI diff binary/crate.
- Force syntect inside Diff default path.
- Fake line numbers on empty SBS half.

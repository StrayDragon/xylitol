# Design — c462-demo-tool-status-bg

## Decisions

1. **Application-layer composition** — do not add a package `ToolBlock` component. Demo (and later `src/app/tui` transcript) wraps already-rendered lines with `apply_background_to_line` + a status→bg closure. Matches `expandable.md` / write-tui seam rules.

2. **`ToolBlockStatus`** — `Pending | Success | Error` on demo `TranscriptEntry::Tool` and `::Diff` (edit tools share the same shell). Default for completed seed/script tools: `Success`.

3. **ANSI contract** — truecolor `\x1b[48;2;R;G;Bm{line}\x1b[49m` only. MUST NOT use `\x1b[0m` to clear bg (would wipe content fg). Hex SSOT remains `DESIGN.md`; demo keeps matching RGB constants until product theme wiring (c458/c470).

4. **Pad then paint** — `fit`/`wrap` to width first, then `apply_background_to_line` so tint spans the full terminal row.

5. **Scripted pending** — optional short `Pending` → `Success` flip for Edit/tool events so the tint is visible in interactive demo; harness may assert success RGB on seed without waiting for the flip.

## Alternatives rejected

- Hard-coding Mocha into `DiffTheme` / package defaults (product tokens belong in DESIGN.md).
- Full-row bg via `\x1b[0m` reset or fg-only ok/error labels (fails pi parity and readability).
- New package capability solely for three RGB constants.

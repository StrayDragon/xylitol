# Porting Checklist

Step-by-step guide to port pi's TUI rendering mode to a new project or language, plus a
smoke-test matrix. Assumes you have read all three module specs
(`rendering-engine.md`, `ui-components.md`, `ux.md`) and `architecture.md`. The canonical
implementation to port from is `packages/tui/src/`; this file is the order of operations
and the validation gates.

## 0. Decide scope

Pick the smallest correct subset first; add modules only when the previous one passes
every smoke test.

- **Minimal**: Terminal abstraction, width utilities, `Component`/`Container`, the diff
  engine with synchronized output, render scheduling, single-focus input, one demo widget
  (`Text`). Covers `rendering-engine.md` Steps 1–10 + `ui-components.md` Steps 0–2.
- **Useful**: add `Input`/`Editor` (`Focusable` + cursor marker), `SelectList`, `Loader`,
  `matchesKey` over Kitty/modifyOtherKeys/legacy VT, `KeybindingsManager`, bracketed paste,
  shared editing primitives (word-nav, kill-ring, undo). Covers `ux.md` Steps 1–5.
- **Full**: add overlays (with the focus-restore state machine), `Markdown`, `Image`,
  autocomplete, status/widget/footer affordances.

Don't port overlays, Markdown, images, or the full overlay state machine until the minimal
set passes every smoke test. The engine is worthless if the diff is wrong.

## 1. Confirm the terminal protocol baseline

The target must support (or you can emulate): ANSI SGR (`\x1b[...m`), cursor positioning
(`\x1b[<n>A/B/G`), line clear (`\x1b[2K`), clear screen + scrollback (`\x1b[2J\x1b[H\x1b[3J`),
raw mode (no line buffering, no signal generation), resize events. Optional but recommended:
synchronized output (`\x1b[?2026h/l`), bracketed paste (`\x1b[?2004h/l`), Kitty keyboard
protocol, OSC 11 background query, OSC 9;4 progress.

For non-Node targets (Rust, Go, Python, Bun): use `crossterm`/`termios`, `tcell`, `tty`, or
the platform's raw-mode API. The `Terminal` interface is small; implement it directly.

## 2. Build module 1 — rendering engine (`rendering-engine.md`)

In order:
- [ ] Width utilities (`ui-components.md` Step 0): `visibleWidth`, `truncateToWidth`,
      `wrapTextWithAnsi`, `sliceByColumn`, `applyBackgroundToLine`. Unit-test in isolation.
- [ ] `Terminal` interface + `ProcessTerminal` (`rendering-engine.md` Step 1): raw mode,
      bracketed paste, resize hook, Kitty keyboard negotiation (or document unenhanced
      keys), Windows VT input, clean `stop`/`drainInput`. Provide a `VirtualTerminal` for
      tests (headless xterm or in-memory cell array) — you cannot test a diff engine
      deterministically against a real terminal.
- [ ] `Component`/`Container`/`Focusable` + `CURSOR_MARKER` (`ui-components.md` Step 1/4).
- [ ] `TUI` engine state (`rendering-engine.md` Step 2): all fields, with `cursorRow` and
      `hardwareCursorRow` tracked **separately**.
- [ ] Render scheduling (`rendering-engine.md` Step 3): coalesce + 16ms cap + `force`.
- [ ] `doRender` pipeline (`rendering-engine.md` Step 4): render → composite overlays (stub
      for now) → extract cursor → apply line resets → strategy → position cursor → save.
- [ ] Three strategies (`rendering-engine.md` Step 5): first render (no clear), full redraw
      (clear), normal diff (move + `\x1b[2K` + line + `\r\n`, clear extras on shrink).
- [ ] Hard width error (`rendering-engine.md` Step 6) with a crash log.
- [ ] Synchronized output wrapping (`rendering-engine.md` Step 7).
- [ ] Viewport/scrollback math (`rendering-engine.md` Step 8): `maxLinesRendered`,
      `previousViewportTop`, width/height-change full redraw, Termux exception.
- [ ] Virtual IME cursor (`rendering-engine.md` Step 9).
- [ ] `stop()` clean exit (`rendering-engine.md` Step 10) + signal handlers.

Canonical reference: `packages/tui/src/tui.ts`, `terminal.ts`, `utils.ts`,
`stdin-buffer.ts`.

## 3. Build module 2 — widgets (`ui-components.md`)

In order of dependency:
- [ ] `Text` (wrap + padding + optional bg + cache).
- [ ] `TruncatedText` (single-line truncate).
- [ ] `Spacer(n)`.
- [ ] `Box` (padded container + bg).
- [ ] `Input` (`Focusable`, horizontal scroll, grapheme cursor, kill-ring, undo, paste,
      Kitty CSI-u decode). Reads `tui.editor.*`/`tui.input.*` keybindings.
- [ ] `Editor` (multi-line, word wrap, autocomplete, paste markers, height-aware scroll,
      jump-to-char). The big one — port incrementally.
- [ ] `SelectList` (filter, wrap navigation, scroll indicator, two/single-column layout).
- [ ] `SettingsList` (cycle + submenus).
- [ ] `Loader` / `CancellableLoader` (spinner + abort signal). The sanctioned self-render-
      scheduling widget.
- [ ] `Markdown` (optional; needs a markdown parser + `MarkdownTheme` + optional
      `highlightCode`).
- [ ] `Image` (optional; needs Kitty/iTerm2 graphics protocol).

Canonical reference: `packages/tui/src/components/*`, `autocomplete.ts`, `fuzzy.ts`.

For each widget, enforce the composable contract (`ui-components.md` Step 6): width ≤
`width`, styles per line, no direct terminal writes, no render scheduling from `render()`,
`invalidate()` clears caches, keys via `KeybindingsManager`, `CURSOR_MARKER` only when focused.

## 4. Build module 3 — UX (`ux.md`)

In order:
- [ ] stdin splitter (`ux.md` Step 1): split batched input into single key sequences
      before routing.
- [ ] Key model (`ux.md` Step 2): `KeyId` + `Key` + `matchesKey` over legacy/modifyOtherKeys/
      Kitty; `decodeKittyPrintable`; `isKeyRelease`/`isKeyRepeat`; `setKittyProtocolActive`.
- [ ] Keybindings registry (`ux.md` Step 3): `TUI_KEYBINDINGS` defaults +
      `KeybindingsManager` with conflict detection + user overrides + global accessor.
- [ ] Single-focus routing (`ux.md` Step 4): `setFocus` toggles `Focusable.focused`;
      `handleInput` runs listeners → focus → `focused.handleInput` → `requestRender`.
      Filter key releases unless `wantsKeyRelease`.
- [ ] Shared editing primitives (`ux.md` Step 5): `word-navigation`, `kill-ring`,
      `undo-stack`, `autocomplete`, bracketed-paste buffering.
- [ ] Overlays — minimal port first (`ux.md` Step 6): stack, `showOverlay`/`hide`, restore
      `preFocus`, composite in stack order. Then upgrade to the full focus-restore state
      machine if you need stacked overlays with temporary focus handoff.
- [ ] Status/widget/footer affordances (`ux.md` Step 7) — only if you are building the host
      app integration surface.

Canonical reference: `packages/tui/src/keys.ts`, `keybindings.ts`, `stdin-buffer.ts`,
`word-navigation.ts`, `kill-ring.ts`, `undo-stack.ts`, `autocomplete.ts`, and the overlay
code in `tui.ts`.

## 5. Smoke-test matrix

Run each and watch the diff engine (enable a `PI_DEBUG_REDRAW`-equivalent log):

1. **Append-only growth** — add a line each second. Expect: only the new line written each
   frame; no scrollback pollution; terminal stays on the last line.
2. **In-place edit** — a single line changes mid-screen (spinner/counter). Expect: only
   that line rewritten; everything above/below untouched; no flicker.
3. **Width change** — resize narrower then wider. Expect: full redraw on each change;
   re-wrapped lines; no horizontal artifacts; no orphaned characters at line ends.
4. **Shrink** — content 20→5 lines. With `clearOnShrink`: full redraw clears the 15
   orphaned rows. Without: rows stay (document this).
5. **CJK width** — render `"你好世界"` and truncate to width 5. Expect: `"你好"` (4 cols) +
   ellipsis, never a half char.
6. **ANSI truncation** — `truncateToWidth(red("Hello") + " " + blue("World"), 8)`.
   Expect: red "Hello" + " W…" with a single closing reset; no leaked color after.
7. **Wrap with color** — a 3-line red paragraph wrapped to 10 cols. Expect: all 3 lines
   red, no line reverting to default.
8. **IME cursor** — a `Focusable` input with the cursor mid-line. Expect: hardware cursor
   (if shown) at the marker; CJK candidate window at the cursor.
9. **Key model across protocols** — Up arrow works under legacy VT, modifyOtherKeys, and
   Kitty (if you can test each). Ctrl+C works. Shift+Tab works.
10. **Keybinding override** — rebind `tui.select.up` to `k`; verify the list navigates on
    `k` and no longer on Up.
11. **Overlay compositing** — centered dialog over base content. Expect: base visible
    around the dialog; dialog bg doesn't bleed into base on either side; closing restores
    base exactly.
12. **Overlay focus** — open a dialog, Escape, confirm focus returns to the editor and the
    editor's cursor marker works.
13. **Stacked overlays** (full state machine) — dialog A, then B over it. Close B → A
    re-takes focus. Close A → editor re-takes focus.
14. **Ctrl+C cleanup** — press Ctrl+C. Expect: terminal restored to cooked mode, cursor
    visible, prompt on a fresh line, no raw-mode shell. Also test `SIGTERM`/`SIGHUP`.
15. **Suspend/resume** — Ctrl+Z then `fg`. Expect: dims re-read, full redraw, no garbage.
16. **Slow SSH** — pipe through a 100ms-delayed pty. Expect: synchronized output keeps
    frames atomic; no partial paints visible.

## Common porting bugs (and which module they belong to)

- **Lines wrap unexpectedly** (engine): a widget returned a line > `width`. The hard error
  should catch it; if not, `visibleWidth` is wrong (counting ANSI or mis-measuring wide chars).
- **Flicker on every frame** (engine): not wrapping writes in synchronized output, or
  calling `terminal.write` multiple times per frame instead of building one buffer.
- **Scrollback fills up** (engine): using `\n` instead of `\r\n`, or clearing every frame
  instead of diffing. First render must not clear; normal updates must not scroll.
- **Stale characters at line ends** (engine): missing `\x1b[2K` before a shorter line.
- **Colors leak across lines** (engine/widgets): missing per-line reset in `applyLineResets`,
  or a widget didn't close SGR within a line.
- **Cursor jumps on exit** (engine): `stop()` didn't move to end of content before showing
  the cursor.
- **Focus stuck after overlay close** (ux): `hide()` didn't restore `preFocus` or retarget
  dangling `preFocus` chains.
- **IME candidate window in wrong place** (widgets): a container with an embedded `Input`
  didn't implement `Focusable` + propagate `focused`.
- **Three Up arrows do nothing** (ux): didn't split batched stdin; `matchesKey` got
  `"\x1b[A\x1b[A\x1b[A"` instead of three `"\x1b[A"` events.
- **Rebinding a key has no effect** (ux): widget hardcoded the key instead of using
  `KeybindingsManager.matches(data, "tui.*")`.
- **Diff writes to the wrong row after an in-place edit** (engine): merged `cursorRow` and
  `hardwareCursorRow`; the IME move over- or under-shoots.

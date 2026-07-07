---
name: terminal-tui-differential-rendering
description: "Build a flicker-free terminal TUI with pi's rendering mode. Three modules to implement from scratch as a consistent system: (1) a differential render engine that diffs previous vs new lines and writes only changed lines wrapped in synchronized output (CSI 2026), with a Terminal abstraction, coalesced rate-limited scheduling, a hard width invariant, and a virtual IME cursor; (2) a UI widget system on a Component contract (render(width) -> string[], handleInput, invalidate) with ANSI-aware width utilities, render caching, theming-on-invalidate, and built-ins (Text, Box, Editor, Input, SelectList, SettingsList, Markdown, Loader, Image); (3) a UX layer with single-focus input routing, a protocol-agnostic key model (Kitty + modifyOtherKeys + legacy VT), configurable keybindings, bracketed paste, and an overlay stack for modals. Use when porting pi's TUI (@earendil-works/pi-tui) to another project or language, or designing a consistent terminal UI subsystem from scratch."
license: MIT
compatibility: Language-agnostic. Canonical implementation is TypeScript on Node.js (raw stdin/stdout). Ports need a terminal with ANSI/SGR, raw mode, and resize events; Kitty keyboard protocol and synchronized output are progressive enhancements.
metadata:
  source: "@earendil-works/pi-tui (packages/tui in earendil-works/pi-mono)"
  version: "2.0"
---

# Terminal TUI with Differential Rendering

A spec for building pi's terminal UI rendering mode as a **consistent system from scratch**,
split into three modules that must agree on shared invariants. The canonical implementation
lives in this repo under `packages/tui/src/`; this skill is an index + build spec into it.

## Canonical source index

When you need the authoritative answer, read the repo file, not a paraphrase. All paths
are relative to the repo root.

**Render engine + Terminal layer**
- `packages/tui/src/tui.ts` — `TUI` class, `Component`/`Container`/`Focusable` interfaces,
  `CURSOR_MARKER`, differential `doRender`, overlay compositing, render scheduling, focus.
- `packages/tui/src/terminal.ts` — `Terminal` interface, `ProcessTerminal` (raw mode,
  bracketed paste, Kitty keyboard negotiation, Windows VT input, clean stop/drain).
- `packages/tui/src/utils.ts` — `visibleWidth`, `truncateToWidth`, `wrapTextWithAnsi`,
  `sliceByColumn`, `sliceWithWidth`, `extractSegments`, `applyBackgroundToLine`,
  `getGraphemeSegmenter`/`getWordSegmenter`.
- `packages/tui/src/terminal-image.ts` — Kitty/iTerm2 inline image protocol (optional).
- `packages/tui/src/terminal-colors.ts` — OSC 11 background + color-scheme queries.
- `packages/tui/src/stdin-buffer.ts` — splits batched stdin into single key sequences.
- `packages/tui/README.md` — the maintainers' own architecture overview.

**UI components/widgets**
- `packages/tui/src/components/text.ts` — `Text` (wrap + padding + optional bg + cache).
- `packages/tui/src/components/truncated-text.ts` — single-line truncate.
- `packages/tui/src/components/box.ts` — padded container with background.
- `packages/tui/src/components/spacer.ts` — empty rows.
- `packages/tui/src/components/input.ts` — single-line `Focusable` input, horizontal scroll,
  kill ring, undo, grapheme-aware cursor.
- `packages/tui/src/components/editor.ts` — multi-line `Focusable` editor, word wrap,
  autocomplete, paste markers, height-aware scrolling, jump-to-char.
- `packages/tui/src/components/select-list.ts` — keyboard navigable list, filter, scroll.
- `packages/tui/src/components/settings-list.ts` — toggle/cycle + submenus.
- `packages/tui/src/components/markdown.ts` — markdown render + syntax-highlight hook.
- `packages/tui/src/components/loader.ts` / `cancellable-loader.ts` — spinner + abort signal.
- `packages/tui/src/components/image.ts` — inline image with fallback.
- `packages/tui/src/autocomplete.ts` — slash commands + file/path autocomplete.
- `packages/tui/src/fuzzy.ts` — fuzzy match/filter for lists and autocomplete.

**UX / interaction layer**
- `packages/tui/src/keys.ts` — `KeyId` model, `Key` helper, `matchesKey`, `parseKey`,
  Kitty CSI-u parsing, `decodeKittyPrintable`/`decodePrintableKey`, key-release/repeat.
- `packages/tui/src/keybindings.ts` — `TUI_KEYBINDINGS` defaults, `KeybindingsManager`,
  conflict detection, user overrides, `getKeybindings()`/`setKeybindings()`.
- `packages/tui/src/word-navigation.ts` — `findWordBackward`/`findWordForward`.
- `packages/tui/src/kill-ring.ts` / `undo-stack.ts` — Emacs kill/yank + undo coalescing.
- `packages/tui/src/native-modifiers.ts` — platform-native modifier query (Apple Terminal).
- `packages/tui/src/tui.ts` — overlay stack + focus-restore state machine (in the engine,
  but it is the UX of modals).
- `packages/coding-agent/docs/tui.md` — the user-facing TUI authoring guide (patterns).
- `packages/coding-agent/examples/extensions/` — working overlays, custom editors,
  status lines, widgets (see `overlay-qa-tests.ts`, `modal-editor.ts`, `plan-mode/`).

## The three modules (build each as a spec, then make them agree)

The system is only correct when the three modules share these invariants. Read each
reference as "how to build this module from scratch so it composes with the other two."

1. **Rendering engine** → `references/rendering-engine.md`
   The Terminal abstraction, the `TUI` engine state, the per-frame pipeline, the diff
   algorithm and three strategies, synchronized output, render scheduling, the hard width
   invariant, per-line style resets, viewport/scrollback math, and the virtual IME cursor.
   *Build target:* a `Terminal` + `TUI` that can render an array of lines in place without
   flicker or scrollback pollution.

2. **UI components/widgets** → `references/ui-components.md`
   The `Component`/`Focusable` contract, the mandatory ANSI-aware width utilities, the
   render-cache + invalidate protocol, the theming-on-invalidate rebuild pattern, and the
   catalog of built-in widgets with their responsibilities and invariants. *Build target:*
   a widget set where every widget honors the width contract and re-themes on invalidate.

3. **UX / interaction** → `references/ux.md`
   Single-focus input routing, the protocol-agnostic key model (`matchesKey` over Kitty /
   modifyOtherKeys / legacy VT), configurable keybindings with conflict detection,
   bracketed paste, stdin batching/splitting, the overlay stack as modal UX with its
   focus-restore state machine, and the status/widget/footer affordances. *Build target:*
   an interaction layer that routes one key to one focused component and makes modals
   composable without focus stranding.

**Shared invariants (all three modules depend on these)**

1. **Width contract**: every `render(width)` line has display width ≤ `width`. The engine
   hard-errors on overflow. Width utilities must be ANSI-aware and wide-char-aware.
2. **Styles do not cross lines**: the engine appends `\x1b[0m\x1b]8;;\x07` per non-image
   line. Widgets re-open styles per line or use `wrapTextWithAnsi`.
3. **One focused component** receives input. Input is routed, not broadcast.
4. **Render is coalesced and rate-limited** (~16ms cap). Never write to the terminal from
   inside `render()`.
5. **Cursor is virtual by default**: hardware cursor hidden; a `Focusable` widget emits
   `CURSOR_MARKER`; the engine positions the hardware cursor there for IME only.
6. **`invalidate()` is the theme/force-redraw signal**: widgets clear caches; widgets that
   pre-bake themed strings rebuild on invalidate.

## How to use this skill

- **Porting pi's TUI elsewhere**: read all three module specs in order (engine → widgets →
  UX), then `references/porting-checklist.md` for the step-by-step and smoke-test matrix.
- **Building one module from scratch**: read that module's spec + the shared invariants;
  the spec tells you which repo files to open as the canonical example.
- **Using `@earendil-works/pi-tui` as a dependency inside this repo**: stop, you don't need
  this skill — import from `@earendil-works/pi-tui` and read `packages/coding-agent/docs/tui.md`.

## Gotchas (cross-cutting)

- **Never `console.log`/print from a widget.** All output goes through the engine's
  `terminal.write`. Direct writes desync the cursor and corrupt the diff.
- **stdin batches characters.** A typed word arrives as one `data` event; an arrow key
  arrives as a multi-byte escape; rapid keys arrive concatenated. Split into single key
  sequences before routing (`packages/tui/src/stdin-buffer.ts`).
- **Never compare `data === "\x1b[A"` directly.** Use `matchesKey`/`KeybindingsManager` so
  the same logical key works under Kitty, modifyOtherKeys, and legacy VT.
- **Never hardcode key checks in a widget.** Bind to a `tui.*` keybinding id so it stays
  configurable via `TUI_KEYBINDINGS` / user overrides.
- **Width change always forces full redraw.** Wrapping changes every line.
- **First changed line above the viewport → full redraw.** Diff can only touch visible rows.
- **`cursorRow` ≠ `hardwareCursorRow`.** The first is logical end-of-content; the second
  is where the cursor physically stopped. Track separately or IME moves corrupt the diff.
- **Disable protocols on exit.** Restore raw mode, disable bracketed paste, Kitty keyboard,
  `modifyOtherKeys`; show cursor; move to end of content; drain stdin over slow SSH.
- **`fullRender` doesn't width-check in pi-tui today** (see `audit.md` Finding 2). When
  porting, put the hard width error in **both** the diff path and `fullRender` so a widget
  that overflows during a full-redraw frame crashes loudly instead of silently desyncing.

## Reference files

- `references/rendering-engine.md` — module 1: build the render engine from scratch.
- `references/ui-components.md` — module 2: build the widget system from scratch.
- `references/ux.md` — module 3: build the interaction/UX layer from scratch.
- `references/terminal-foundations.md` — terminal emulator model, the protocols a TUI
  depends on, the immediate-vs-retained and cell-grid-vs-line-array design axes (where
  ratatui and pi-tui sit), low-level libraries by language, the cost case for a Rust port
  on crossterm (which absorbs pi-tui's hardest module — the cross-protocol key model),
  what's worth hybridizing, and a decision table for when a retained-on-raw-I/O engine
  beats just using ratatui.
- `references/architecture.md` — how the three modules compose at runtime (the per-frame
  pipeline and the state they share).
- `references/audit.md` — reverse check: where pi-tui's code diverges from these specs
  (including one real pi-tui issue: `fullRender` doesn't width-check), so a port doesn't
  inherit the gaps.
- `references/porting-checklist.md` — step-by-step port + 16-case smoke-test matrix.

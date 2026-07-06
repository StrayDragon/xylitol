# Module 3 — UX / Interaction Layer

How to build pi's interaction layer from scratch so it composes with the render engine
(`rendering-engine.md`) and the widget system (`ui-components.md`). Canonical
implementations: `packages/tui/src/keys.ts`, `keybindings.ts`, `stdin-buffer.ts`,
`word-navigation.ts`, `kill-ring.ts`, `undo-stack.ts`, and the overlay stack + focus-restore
state machine in `tui.ts`. Open them alongside this spec.

## What this module is responsible for

Be the single bridge between raw stdin and the widget tree: turn batched, multi-protocol
terminal input into logical key events; route exactly one key to exactly one focused
component; let app-wide keys be intercepted; make key bindings configurable without
touching widget code; handle paste, undo, kill/yank, and word navigation as shared
primitives; and provide a modal UX (overlays) that composes without stranding focus. This
module does **not** render and does **not** own widget state; it only routes and interprets.

## Build target

```
TUI.handleInput(data) → routes one key to one focused component (then requestRender)
TUI.addInputListener(fn) → app-wide interceptors (consume / rewrite)
TUI.setFocus(component) → single focus; toggles Focusable.focused
matchesKey(data, keyId) / KeybindingsManager.matches(data, "tui.*") → protocol-agnostic key identity
TUI.showOverlay(component, options) → modal UX with focus restore
```

## Step 1 — stdin batching and splitting

stdin delivers **batched** input: a typed word arrives as one `"hi"` event; an arrow key
arrives as a multi-byte `"\x1b[A"`; rapid keys arrive concatenated as
`"\x1b[A\x1b[A\x1b[A"`. You must split into individual key sequences **before** routing, or
`matchesKey` against a single sequence fails and three Up arrows become one unmatched blob.

**Where splitting lives in pi-tui**: it is a **Terminal-layer** responsibility, not the
engine's. `StdinBuffer` (`packages/tui/src/stdin-buffer.ts`), owned by `ProcessTerminal`,
accumulates raw stdin chunks and emits one `data` event per **complete escape sequence**
(CSI/OSC/DCS/APC/SS3/meta, via `isCompleteSequence`), plus a `paste` event for bracketed
paste (which `terminal.ts` re-wraps as `\x1b[200~<content>\x1b[201~`).
`forwardInputSequence` calls `this.inputHandler(sequence)` — `TUI.handleInput` — **one
sequence at a time**. The engine's `handleInput` does not loop over a split; it receives
already-split sequences.

**Nuance**: `StdinBuffer` splits *escape sequences*; a printable run like `"hi"` is **not**
split into `"h"` and `"i"` (it's not an escape sequence). Widgets handle printable runs by
inserting the whole `data` after rejecting control chars (`Input.handleInput`'s
`insertCharacter`). So "three Up arrows as one blob" is solved by splitting the three CSI
sequences; "a typed word as one event" is solved by the widget accepting multi-char `data`.

`StdinBuffer` also disambiguates a lone Escape from the start of a CSI sequence via a
small timeout (`\x1b` alone is incomplete until either more bytes arrive or the timeout
fires). This is why splitting belongs in the Terminal layer: it needs timing/accumulation
the engine shouldn't own.

The engine's `handleInput` thus processes one already-split sequence: run input listeners,
route to the focused component, call `requestRender()`. See `rendering-engine.md` agreement
section and `audit.md` Finding 1.

**Port implication**: put the splitter in your `Terminal` implementation (or use a lib that
yields one event at a time, e.g. crossterm's `event::read`). Don't loop-split in the
engine. See `terminal-foundations.md` "On a raw I/O lib".

## Step 2 — The protocol-agnostic key model

Terminals encode the same logical key three different ways depending on protocol:

| Protocol | Up arrow | Ctrl+C | Shift+Tab |
|---|---|---|---|
| Legacy VT | `\x1b[A` | `\x03` | `\x1b[Z` (often lost without Windows VT input) |
| modifyOtherKeys | `\x1b[1;2A` etc. | `\x03` | `\x1b[1;2Z` |
| Kitty keyboard | `\x1b[<code>;<mods>u` | `\x1b[99;5u` | `\x1b[9;2u` |

Never compare `data === "\x1b[A"` in a widget. Build a `KeyId` model + `matchesKey` that
normalizes across all three. Canonical: `packages/tui/src/keys.ts`.

`KeyId` is a typed union: `BaseKey` (letters, digits, symbol keys, special keys like
`enter`/`escape`/`up`/`f1`) plus modifier combinations (`ctrl+c`, `shift+tab`, `alt+left`,
`ctrl+shift+p`). The `Key` helper gives autocomplete: `Key.enter`, `Key.ctrl("c")`,
`Key.shift("tab")`, `Key.ctrlShift("p")`.

`matchesKey(data, keyId)`:
1. Parse `keyId` into `{ key, ctrl, shift, alt, super }` and a modifier bitfield.
2. For each protocol, try the matching decoder:
   - Legacy: direct byte match (e.g. `\x03` for Ctrl+C, `\x1b[A` for Up).
   - Kitty: `matchesKittySequence(data, codepoint, modifier)` against `\x1b[<cp>;<mods>u`.
   - modifyOtherKeys: `matchesModifyOtherKeys(data, codepoint, modifier)` against
     `\x1b[1;<n><final>`.
3. Return true if any form matches.

Also implement:
- `parseKey(data): string | undefined` — return the `KeyId` for a given `data` (for debug).
- `decodeKittyPrintable(data): string | undefined` — Kitty flag 1 (disambiguate) sends
  plain letters as CSI-u (`\x1b[97u` for 'a'); decode so text widgets can insert them.
- `decodePrintableKey(data)` — extract the printable glyph from any protocol form.
- `isKeyRelease(data)` / `isKeyRepeat(data)` — Kitty flag 2 (event types); the engine
  filters releases unless `component.wantsKeyRelease === true`.
- `setKittyProtocolActive(active)` — global flag set by `ProcessTerminal` after
  negotiation; `matchesKey` consults it to pick the right decoder path.

## Step 3 — Configurable keybindings (the `tui.*` registry)

Widgets should not hardcode keys; they should bind to a **keybinding id** so users can
rebind without forking widget code. Canonical: `packages/tui/src/keybindings.ts`.

Define a registry of ids with default keys and descriptions:
```
"tui.editor.cursorUp"      { defaultKeys: "up" }
"tui.editor.cursorLeft"    { defaultKeys: ["left", "ctrl+b"] }   // multiple defaults OK
"tui.editor.deleteWordBackward" { defaultKeys: ["ctrl+w", "alt+backspace"] }
"tui.editor.deleteToLineEnd"    { defaultKeys: "ctrl+k" }
"tui.editor.undo"          { defaultKeys: "ctrl+-" }
"tui.input.submit"         { defaultKeys: "enter" }
"tui.input.newLine"        { defaultKeys: ["shift+enter", "ctrl+j"] }
"tui.select.up"            { defaultKeys: "up" }
"tui.select.confirm"       { defaultKeys: "enter" }
"tui.select.cancel"        { defaultKeys: ["escape", "ctrl+c"] }
... (see TUI_KEYBINDINGS for the full set)
```

`KeybindingsManager`:
- Holds `definitions` (the defaults) + `userBindings` (overrides).
- On `rebuild()`: compute `keysById` (resolved keys per id) and `conflicts` (a key claimed
  by >1 id in user bindings — don't silently let one shadow another; surface them).
- `matches(data, id)` — true if `data` matches any resolved key for `id`.
- `getKeys(id)` / `getResolvedBindings()` / `getConflicts()`.
- `setUserBindings(config)` — apply overrides + rebuild.
- `getKeybindings()` / `setKeybindings(manager)` — process-global accessor widgets use.

Downstream packages extend the registry by declaration merging on the `Keybindings`
interface and adding to the definitions object. **Never hardcode key checks** in a widget
(e.g. `if (data === "\x1b[A")`); always `kb.matches(data, "tui.select.up")`. This is also
a project rule in `AGENTS.md`: add defaults to the registry so they stay configurable.

## Step 4 — Single-focus input routing

The engine owns `focusedComponent` and routes input. Canonical: `tui.ts` `handleInput` +
`setFocus`.

```
handleInput(data):
  1. consume protocol replies (OSC 11 background, color scheme, CSI 16 t cell size)
  2. for each inputListener: {consume: true} stops; {data: rewritten} transforms
  3. global debug key (Shift+Ctrl+D) → onDebug()
  4. if focused component is an overlay that became invisible, redirect focus
  5. if focus is not on an overlay and a visible overlay is eligible, restore overlay focus
  6. if focusedComponent.handleInput:
       if isKeyRelease(data) && !wantsKeyRelease: return
       focusedComponent.handleInput(data)
       requestRender()
```

`setFocus(component)`:
1. If old focus is `Focusable`, `old.focused = false`.
2. `focusedComponent = next`.
3. If new focus is `Focusable`, `next.focused = true`.

(Overlay focus restore complicates this — see Step 6 — but the base rule is one focused
component. The engine never interprets keys itself except the debug key; key semantics
live in widgets via `matchesKey`/`KeybindingsManager`.)

**Input listeners** (`addInputListener`): run *before* focus routing. Return `{consume:
true}` to stop, `{data: rewritten}` to transform. Use for app-wide keys: Ctrl+C to exit
(raw mode disables SIGINT), Ctrl+L to force redraw, custom key remaps. **Important**: when
a listener returns `{consume: true}`, the engine returns before the focused-widget path
that calls `requestRender()`. A consuming listener that changes state must call
`tui.requestRender()` itself.

## Step 5 — Shared editing primitives (so every text widget behaves the same)

Factor these out so `Input`, `Editor`, and any custom text widget share behavior. All under
`packages/tui/src/`.

- **`word-navigation.ts`** — `findWordBackward(text, pos)` / `findWordForward(text, pos)`.
  Word boundaries via `getWordSegmenter()` (`Intl.Segmenter` granularity "word"). Used by
  Ctrl+Left/Right, Alt+B/F, Ctrl+W, Alt+D.
- **`kill-ring.ts`** — Emacs kill/yank. `push(text, {prepend, accumulate})` accumulates
  consecutive kills so `Ctrl+W Ctrl+W` yanks as one block. `peek`/`rotate` for
  `yank`/`yankPop` (`Ctrl+Y` / `Alt+Y`). `lastAction` tracking (`"kill"|"yank"|"type-word"`)
  decides accumulation and yank-pop eligibility.
- **`undo-stack.ts`** — `UndoStack<T>` with push/pop. Widgets coalesce: consecutive word
  chars coalesce into one undo unit (`if (whitespace || lastAction !== "type-word") push`).
- **`autocomplete.ts`** — `AutocompleteProvider` interface + `CombinedAutocompleteProvider`
  for slash commands + file paths. File completion shells out to `fd`/`find`, honors
  `~/`/`./`/`../`/`@` prefixes, filters attachable files for `@`. `fuzzy.ts` for ranking.
- **Bracketed paste** — widgets buffer `\x1b[200~`…`\x1b[201~` content and insert it as one
  undo unit. `Editor` goes further: large pastes (>10 lines) become `[paste #N +M lines]`
  markers treated as atomic graphemes (`segmentWithMarkers`) so cursor movement skips them.

These are UX because they define the *feel* of text editing across the whole app; building
them per-widget would diverge.

## Step 6 — Overlays as modal UX

Overlays render components on top of base content without clearing the screen. Use for
dialogs, menus, side panels. The engine composites them (`rendering-engine.md` Step 4);
this step specifies the UX contract. Canonical: `tui.ts` `showOverlay`/`hideOverlay`/
`OverlayHandle` + the overlay stack + focus-restore state machine.

```typescript
type OverlayAnchor = "center" | "top-left" | "top-right" | "bottom-left" | "bottom-right"
  | "top-center" | "bottom-center" | "left-center" | "right-center";
type SizeValue = number | `${number}%`;

interface OverlayOptions {
  width?: SizeValue; minWidth?: number; maxHeight?: SizeValue;
  anchor?: OverlayAnchor;           // default "center"
  offsetX?: number; offsetY?: number;
  row?: SizeValue; col?: SizeValue; // absolute or percentage (0%=top/left, 100%=bottom/right)
  margin?: OverlayMargin | number;
  visible?: (w: number, h: number) => boolean;  // hide on narrow/short terminals
  nonCapturing?: boolean;           // don't auto-focus when shown
}

interface OverlayHandle {
  hide(): void; setHidden(h: boolean): void; isHidden(): boolean;
  focus(): void; unfocus(opts?: { target: Component | null }): void; isFocused(): boolean;
}
```

**Stack + z-order**: `overlayStack: { component, options, preFocus, hidden, focusOrder }[]`.
`focusOrder` is a monotonic counter; higher = visual front. `showOverlay` pushes with
`preFocus = focusedComponent`, focuses the component (unless `nonCapturing`), requests
render. `hide`/`handle.hide()` splices out, retargets any other overlay whose `preFocus`
pointed at this one (relink to this entry's `preFocus`), restores focus to the topmost
visible capturing overlay or `preFocus`. **Lifecycle**: overlay components are disposed on
hide; re-call `showOverlay` with a fresh instance to re-show (don't reuse references).

**Positioning resolution** (`resolveOverlayLayout`): parse margin → `availWidth/Height`;
width = `parseSizeValue(width) ?? min(80, availWidth)`, apply `minWidth`, clamp; maxHeight
similarly; effective height = `min(renderedHeight, maxHeight)`; row/col by absolute >
percentage > anchor (anchors map to top/bottom/center of the available area); apply offsets;
clamp to margins. `visible(w,h)` is called each frame; when a focused overlay becomes
invisible (terminal shrunk), `handleInput` redirects focus to the topmost visible overlay
or `preFocus` — side panels disappear gracefully without stranding focus.

**Compositing** (`compositeOverlays`): render each visible overlay at its width, truncate
to `maxHeight`, position by row/col, splice each overlay line into the base line at `col`
(`compositeLineAt`): extract base segments before/after, insert the overlay segment, pad
each part, append a segment reset between parts so the overlay's background doesn't bleed
into base content on either side, and a final `visibleWidth > totalWidth` truncation
safeguard. Never splice into an image line.

**`workingHeight` invariant** (see `audit.md` Finding 3): pad the result to
`workingHeight = max(result.length, termHeight, max(row + len))` — and **do not include
`maxLinesRendered`**. pi-tui has an explicit comment that including the historical
high-water mark caused self-reinforcing inflation that pushed content into scrollback on
terminal widen. Inherit the fix, not the bug.

### Minimal overlay port (skip the state machine)

If you only need one modal at a time: stack without `focusOrder`, focus the component on
show, restore `preFocus` on hide, composite in stack order. Covers dialogs, menus,
confirm prompts. Upgrade to the full system when you need stacked overlays with temporary
focus handoff.

### Full focus-restore state machine

The subtle part: a focused visible overlay should reclaim input after a *temporary*
replacement UI closes, but also let a non-overlay component receive input while the overlay
stays visible (e.g. a dialog overlay + the editor below it both visible).

State:
```
{ status: "inactive" }
| { status: "eligible"; overlay }                          // overlay wants focus back when possible
| { status: "blocked"; overlay; blockedBy; resume: { restore-overlay } | { focus-target, target } }
```

Transitions (simplified): `setFocus(overlayComponent)` while visible → `eligible`.
`setFocus(nonOverlay)` while `eligible` and not an ancestor of the overlay's focus chain →
`blocked` by the new focus, `resume: restore-overlay`. `setFocus(null)` with
`overlayFocusRestore: "clear"` → `inactive`. On the next `handleInput`, if focus is not on
an overlay and state is `eligible`, restore the overlay; if `blocked` and `blockedBy` no
longer has focus, resume (restore overlay or focus the explicit `target`).
`unfocus({ target })` sets `resume: { focus-target, target }`. `isOverlayFocusAncestor`
walks the `preFocus` chain to decide whether a focus target is "inside" the overlay's
lineage. Canonical: `tui.ts` `setFocusInternal`/`getVisibleOverlayFocusRestore`/
`isOverlayFocusAncestor`/`retargetOverlayPreFocus`.

This is the most complex part of the system. For a port, skip it first (use the minimal
port above) and add it only when you need stacked overlays with temporary focus handoff.

## Step 7 — Status / widget / footer affordances

These are UX surfaces the host app (coding-agent) exposes; the TUI package itself provides
the primitives. See `packages/coding-agent/docs/tui.md` for the full pattern list.

- **Status line**: `ctx.ui.setStatus("id", coloredString)` / `undefined` to clear —
  persists across renders in the footer. Good for mode indicators.
- **Working indicator**: `ctx.ui.setWorkingIndicator({ frames, intervalMs })` — customize
  the inline streaming spinner; `frames: []` hides it; call with no args to restore default.
- **Widgets above/below editor**: `ctx.ui.setWidget("id", lines | (tui, theme) => { render, invalidate }, { placement })` —
  persistent content like todo lists.
- **Custom footer**: `ctx.ui.setFooter((tui, theme, footerData) => ({ render, invalidate, dispose }))` —
  `footerData` exposes git branch + extension statuses + reactive `onBranchChange`.
- **Custom editor**: `ctx.ui.setEditorComponent((tui, theme, keybindings) => new CustomEditor(...))` —
  modal editing (vim), different keybindings. Extend `CustomEditor` (not base `Editor`) to
  keep app keybindings (escape to abort, ctrl+d to exit, model switching).

These are not part of the minimal engine port; they are the integration surface a host app
builds on top of the engine + widgets.

## Step 8 — Accessibility and IME

- **IME candidate window positioning** is the reason for `CURSOR_MARKER` (`ui-components.md`
  Step 4). The hardware cursor is hidden but positioned at the marker so CJK candidate
  windows anchor correctly. Some terminals require a *visible* hardware cursor for IME;
  enable `showHardwareCursor` / `PI_HARDWARE_CURSOR=1`.
- **Containers with embedded inputs must propagate `focused`** to the child `Input`/`Editor`,
  or the candidate window appears in the wrong place.
- **High contrast**: theme provides `fg`/`bg` categories (text, accent, muted, dim, success,
  error, warning, borders, diffs, syntax). The engine queries the terminal color scheme
  (`queryTerminalColorScheme`, OSC 996/997) and broadcasts changes so widgets can re-theme.
- **Keyboard-only operation**: every built-in widget is keyboard navigable; overlays close
  on Escape; `CancellableLoader` aborts on Escape. Never build a widget that requires a mouse.

## Step 9 — Clean exit (shared with the engine)

`stop()` disables bracketed paste, Kitty keyboard, `modifyOtherKeys`; restores raw mode;
shows cursor; moves to end of content. `drainInput()` over slow SSH so late Kitty
key-release events don't leak to the parent shell. Handle `SIGINT`/`SIGTERM`/`SIGHUP` →
`stop()` + exit. See `rendering-engine.md` Step 10 and `terminal.ts` `stop`/`drainInput`.

## Agreement with the other modules

- **Engine** (`rendering-engine.md`): owns `handleInput` (the routing point) and
  `requestRender()`; composites overlays in the per-frame pipeline; calls `invalidate()` on
  theme/force-redraw.
- **Widgets** (`ui-components.md`): read `KeybindingsManager` to interpret keys; emit
  `CURSOR_MARKER` when focused; `Loader` is the only widget that self-schedules renders.
- **This module** owns: the key model, the keybinding registry, the input listeners, the
  focus model, the overlay stack + focus-restore state machine, and the shared editing
  primitives. It owns no widget state and no terminal writes.

## Canonical examples to read

- `packages/coding-agent/examples/extensions/overlay-qa-tests.ts` — anchors, margins,
  stacking, responsive visibility, animation.
- `packages/coding-agent/examples/extensions/modal-editor.ts` — custom editor with modes.
- `packages/coding-agent/examples/extensions/plan-mode/` — status + widget + overlay together.
- `packages/coding-agent/examples/extensions/snake.ts` — full game: keyboard, game loop.
- `packages/tui/test/chat-simple.ts` — minimal chat with markdown + editor + autocomplete.

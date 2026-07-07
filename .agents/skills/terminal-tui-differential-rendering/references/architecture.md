# Architecture — How the Three Modules Compose

This file is the runtime view of how `rendering-engine.md`, `ui-components.md`, and
`ux.md` fit together. It is the map; each module spec is the build target. Canonical
source: `packages/tui/src/tui.ts` (the `TUI` class is the composition root).

## The three layers and what crosses them

```
┌──────────────────────────────────────────────────────────────────────┐
│ UX / interaction  (ux.md)                                              │
│   key model (matchesKey, Key, KeybindingsManager)                      │
│   input listeners, single-focus routing, bracketed paste               │
│   overlay stack + focus-restore state machine                          │
│   shared editing primitives (word-nav, kill-ring, undo, autocomplete)  │
└────────────────────────────▲─────────────────────────────────────────┘
                              │ handleInput(data) → focused.handleInput
                              │ KeybindingsManager.matches(data, "tui.*")
┌────────────────────────────┴─────────────────────────────────────────┐
│ UI components / widgets  (ui-components.md)                             │
│   Component: render(width) => string[]   handleInput(data)             │
│   Focusable: focused + CURSOR_MARKER                                    │
│   Container (vertical stack), Text, Input, Editor, SelectList, …       │
└────────────────────────────▲─────────────────────────────────────────┘
                              │ render(width)
┌────────────────────────────┴─────────────────────────────────────────┐
│ Rendering engine  (rendering-engine.md)                                 │
│   TUI: owns previousLines, focus, overlay stack, render scheduling     │
│   diff + synchronized output (CSI 2026) + three strategies             │
│   hard width invariant, per-line style resets, virtual IME cursor      │
│   compositeOverlays (called by the engine, specified by ux.md)         │
└────────────────────────────▲─────────────────────────────────────────┘
                              │ terminal.write(buf) / start / stop
┌────────────────────────────┴─────────────────────────────────────────┐
│ Terminal abstraction  (rendering-engine.md Step 1)                      │
│   ProcessTerminal (raw mode, paste, Kitty negotiation, clean stop)     │
│   VirtualTerminal for tests                                             │
└────────────────────────────────────────────────────────────────────────┘
```

What crosses each boundary (and what must not):

- **UX → widgets**: only `handleInput(data)` and `setFocus` toggling `focused`. UX never
  reads widget state, never writes to widgets except by calling their public methods.
- **Widgets → engine**: only the `render(width)` return value and `CURSOR_MARKER` in the
  output. Widgets never call `terminal.write`, never schedule renders (except `Loader`'s
  animation timer), never touch `previousLines`.
- **Engine → terminal**: only `terminal.write(buf)` and the lifecycle methods. The engine
  never reads `process.stdin`/`stdout` directly.

## The per-frame pipeline (who does what, in order)

1. **Engine** reads `terminal.columns`/`rows`.
2. **Engine** calls `this.render(width)` → walks the **widget** tree (`Container`
   concatenates children's line arrays) → flat `string[]`.
3. **Engine** runs `compositeOverlays` (overlay stack owned by **UX**, compositing logic in
   the engine) → splices each visible overlay's lines into the base at row/col.
4. **Engine** runs `extractCursorPosition` → finds + strips `CURSOR_MARKER` emitted by a
   focused **`Focusable` widget**.
5. **Engine** runs `applyLineResets` → appends `\x1b[0m\x1b]8;;\x07` per line (the
   "styles do not cross lines" invariant).
6. **Engine** picks a strategy (first render / full redraw / normal diff) and writes the
   buffer wrapped in `\x1b[?2026h`…`\x1b[?2026l`. The **hard width invariant** fires here.
7. **Engine** runs `positionHardwareCursor` for IME.
8. **Engine** saves `previousLines`/`previousWidth`/`previousHeight`/`previousViewportTop`/
   `previousKittyImageIds`.

Input is the reverse path: **Terminal** → `TUI.handleInput` (**UX** routing: listeners →
focus → `focusedComponent.handleInput(data)`) → **widget** mutates state + `invalidate()` →
**engine** calls `requestRender()` → next frame.

## Shared state (lives in the engine, read/written by all three)

| State | Owner | Readers | Writers |
|---|---|---|---|
| `previousLines`, `previousWidth/Height`, `previousViewportTop`, `maxLinesRendered` | engine | engine (diff) | engine |
| `focusedComponent` | engine | UX (routing), widgets (`Focusable.focused`) | `setFocus` (UX + overlay handles) |
| `overlayStack`, `focusOrderCounter`, `overlayFocusRestore` | engine | UX (overlay handles), engine (compositing) | `showOverlay`/`hide`/`unfocus` |
| `renderRequested`, `renderTimer`, `lastRenderAt` | engine | engine (scheduling) | `requestRender` (widgets via `Loader`, UX listeners, engine) |
| `KeybindingsManager` (global) | UX (`keybindings.ts`) | widgets | `setKeybindings` (host app) |
| `kittyProtocolActive` (global) | UX (`keys.ts`) | `matchesKey` decoders | `ProcessTerminal` negotiation |

## Shared invariants (all three modules depend on these)

1. **Width contract**: every `render(width)` line has display width ≤ `width`. Engine
   hard-errors. Width utilities (`ui-components.md` Step 0) must be ANSI-aware + wide-char-aware.
2. **Styles do not cross lines**: engine appends `\x1b[0m\x1b]8;;\x07` per non-image line.
   Widgets re-open styles per line or use `wrapTextWithAnsi`.
3. **One focused component** receives input. UX routes, not broadcasts.
4. **Render is coalesced and rate-limited** (~16ms cap). No terminal writes from `render()`.
5. **Cursor is virtual by default**: hardware cursor hidden; `Focusable` widgets emit
   `CURSOR_MARKER`; engine positions the hardware cursor for IME only.
6. **`invalidate()` is the theme/force-redraw signal**: widgets clear caches; widgets that
   pre-bake themed strings rebuild on invalidate.

## Failure modes when the modules disagree

- **Widget returns a line > width** → engine throws (hard width error). Fix the widget with
  `truncateToWidth`; don't relax the engine.
- **Widget forgets `CURSOR_MARKER` when focused** → IME candidate window stays at the last
  known position (usually top-left). Add the marker.
- **Container with embedded input doesn't propagate `focused`** → IME candidate window in
  the wrong place. Implement `Focusable` on the container and forward to the child.
- **Widget hardcodes `data === "\x1b[A"`** → breaks under Kitty/modifyOtherKeys. Use
  `KeybindingsManager.matches`.
- **UX listener consumes input but doesn't `requestRender()`** → state changed, screen
  didn't update. The engine only auto-renders after the focused-widget path.
- **Engine merges `cursorRow`/`hardwareCursorRow`** → the IME move over- or under-shoots,
  and the next diff writes to the wrong row. Track them separately.
- **Widget calls `requestRender()` inside `render()`** → infinite render loop. Only
  `Loader`'s timer callback may self-schedule.
- **Overlay component reused after `hide()`** → stale, disposed reference. Re-call
  `showOverlay` with a fresh instance.

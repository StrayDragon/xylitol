# Module 1 — Rendering Engine

How to build pi's render engine from scratch so it composes with the widget system
(`ui-components.md`) and the interaction layer (`ux.md`). The canonical implementation is
`packages/tui/src/tui.ts` (~1700 lines) and `packages/tui/src/terminal.ts`; open them
alongside this spec.

## What this module is responsible for

Own the terminal as a writable ANSI surface and render a flat array of styled lines in
place — no flicker, no scrollback pollution, correct viewport following, correct wide-char
width, and a hardware cursor positioned for IME. It does **not** know what widgets are; it
only knows `Component.render(width) => string[]` and a focus target. Layout is vertical
stacking of line arrays; 2D positioning is the overlay system (which lives in this module
but is specified in `ux.md` because it is the UX of modals).

## Build target

A `Terminal` abstraction and a `TUI` class with this surface:

```typescript
interface Terminal {
  start(onInput: (data: string) => void, onResize: () => void): void;
  stop(): void;
  drainInput(maxMs?: number, idleMs?: number): Promise<void>; // optional
  write(data: string): void;
  get columns(): number;
  get rows(): number;
  get kittyProtocolActive(): boolean;           // optional
  moveBy(lines: number): void;
  hideCursor(): void;
  showCursor(): void;
  clearLine(): void;
  clearFromCursor(): void;
  clearScreen(): void;
  setTitle(title: string): void;               // optional
  setProgress(active: boolean): void;          // optional (OSC 9;4)
}

class TUI extends Container {
  constructor(terminal: Terminal);
  start(): void;
  stop(): void;
  requestRender(force?: boolean): void;
  setFocus(component: Component | null): void;
  addInputListener(fn: (data: string) => {consume?: boolean; data?: string} | undefined): () => void;
  showOverlay(component: Component, options?: OverlayOptions): OverlayHandle; // see ux.md
  hideOverlay(): void;
  // ...plus the Container methods (addChild/removeChild/render/invalidate)
}
```

## Step 1 — Terminal abstraction

Implement `Terminal` against the real process I/O. Canonical: `packages/tui/src/terminal.ts`
(`ProcessTerminal`).

`start(onInput, onResize)` must:
1. Save `wasRaw = stdin.isRaw`; `stdin.setRawMode(true)`; `stdin.setEncoding("utf8")`; `stdin.resume()`.
2. Enable bracketed paste: `stdout.write("\x1b[?2004h")`.
3. Hook `stdout.on("resize", onResize)`. Refresh dims on start (send `SIGWINCH` on Unix —
   dims are stale after suspend/resume).
4. On Windows, enable `ENABLE_VIRTUAL_TERMINAL_INPUT` so modified keys arrive as VT
   sequences (`\x1b[Z` for Shift+Tab). Must run *after* `setRawMode(true)` (it resets
   console flags). See `enableWindowsVTInput()` + `native/win32/prebuilds/`.
5. Negotiate Kitty keyboard protocol: send `\x1b[>7u` (request flags 1+2+4), then
   `\x1b[?u` (query), then `\x1b[c` (DA sentinel). On Kitty flags response with
   `flags !== 0`, Kitty is active. If DA arrives first with no Kitty response, fall back
   to `modifyOtherKeys` (`\x1b[>4;2m`). Pipe stdin through a `StdinBuffer`
   (`packages/tui/src/stdin-buffer.ts`) that splits batched input into single key
   sequences and re-wraps paste content with `\x1b[200~`…`\x1b[201~`.

`stop()` must: disable bracketed paste (`\x1b[?2004l`); disable Kitty keyboard
(`\x1b[<u`) and `modifyOtherKeys` (`\x1b[>4;0m`) if pushed; destroy the stdin buffer;
remove handlers; `stdin.pause()` (prevents buffered Ctrl+D re-closing the parent shell
over SSH); restore `setRawMode(wasRaw)`.

`drainInput()` (optional but recommended over slow SSH): disable Kitty protocol first so
late key-release events don't generate new escapes, then read stdin for up to `maxMs`
(idle exit after `idleMs`) before `stop()`. See `terminal.ts` `drainInput`.

**Why a separate Terminal layer**: the engine never touches `process.stdin/stdout`
directly. Tests use a `VirtualTerminal` (`packages/tui/test/virtual-terminal.ts` with
`@xterm/headless`) and assert on the ANSI stream. Protocol negotiation is isolated. The
same engine runs anywhere a `Terminal` can be implemented.

## Step 2 — Engine state

`TUI` must own exactly this state (see `tui.ts` field list):

```typescript
previousLines: string[] = [];           // last frame, for diffing
previousKittyImageIds = new Set<number>(); // images currently on screen
previousWidth = 0; previousHeight = 0;
focusedComponent: Component | null = null;
inputListeners = new Set<...>();
renderRequested = false;
renderTimer: NodeJS.Timeout | undefined;
lastRenderAt = 0;
static readonly MIN_RENDER_INTERVAL_MS = 16; // ~60fps cap
cursorRow = 0;          // logical end-of-content row (viewport math)
hardwareCursorRow = 0;  // where the terminal cursor physically is (relative moves)
showHardwareCursor = env.PI_HARDWARE_CURSOR === "1";
clearOnShrink = env.PI_CLEAR_ON_SHRINK === "1";
maxLinesRendered = 0;   // working-area high-water mark (grows, resets on full clear)
previousViewportTop = 0;
fullRedrawCount = 0;
overlayStack: OverlayStackEntry[] = [];   // see ux.md
focusOrderCounter = 0;
```

**`cursorRow` vs `hardwareCursorRow`**: track them separately. `cursorRow` is the logical
end of rendered content (used to compute viewport scrolling). `hardwareCursorRow` is where
the terminal's cursor physically is (used to compute relative move sequences). They
diverge when a diff writes only a middle range and the IME marker is above the end of
content. Merging them is a bug that corrupts the diff (the IME move over- or under-shoots).

## Step 3 — Render scheduling

Coalesce and rate-limit. Canonical: `tui.ts` `requestRender`/`scheduleRender`/`doRender`.

```
requestRender(force = false):
  if force:
    previousLines = []; previousWidth = -1; previousHeight = -1;
    cursorRow = hardwareCursorRow = 0; maxLinesRendered = 0; previousViewportTop = 0;
    clear any renderTimer; renderRequested = true;
    process.nextTick(flush);           // run ASAP, bypassing the interval
    return
  if renderRequested: return            // already pending — coalesce
  renderRequested = true
  process.nextTick(schedule)

schedule:
  if stopped || renderTimer || !renderRequested: return
  delay = max(0, MIN_RENDER_INTERVAL_MS - (now() - lastRenderAt))
  renderTimer = setTimeout(flush, delay)

flush:
  renderTimer = undefined
  if stopped || !renderRequested: return
  renderRequested = false; lastRenderAt = now()
  doRender()
  if renderRequested: schedule()        // more work queued mid-frame
```

Properties: N `requestRender()` calls in one tick → one `doRender()`. A tight loop can't
starve the event loop. `requestRender(true)` invalidates `previousLines` so the next render
full-redraws and clears scrollback (use after theme changes where cached themed strings are
stale). Re-entrancy: a widget calling `requestRender()` during `doRender()` schedules
another frame.

## Step 4 — The per-frame pipeline (`doRender`)

```
1. width = terminal.columns; height = terminal.rows
2. widthChanged = previousWidth != 0 && previousWidth != width
   heightChanged = previousHeight != 0 && previousHeight != height
3. newLines = this.render(width)                       // widget tree → line array
4. if overlayStack.length > 0: newLines = compositeOverlays(newLines, width, height)  // ux.md
5. cursorPos = extractCursorPosition(newLines, height) // find + strip CURSOR_MARKER
6. newLines = applyLineResets(newLines)                // append \x1b[0m\x1b]8;;\x07 per non-image line
7. choose strategy (see Step 5)
8. positionHardwareCursor(cursorPos, newLines.length)  // IME
9. save: previousLines, previousKittyImageIds, previousWidth/Height, previousViewportTop
```

`extractCursorPosition`: scan only the bottom `height` lines (the visible viewport), from
the bottom up, for `CURSOR_MARKER` (`\x1b_pi:c\x07`, a zero-width APC terminals ignore).
`col = visibleWidth(text before marker)`, `row = line index`; strip the marker; return
`{row, col}` or `null`. Only `Focusable` widgets emit the marker (see `ui-components.md`).

`applyLineResets`: for every non-image line, append `"\x1b[0m\x1b]8;;\x07"` (full SGR
reset + OSC 8 hyperlink reset). This is the mechanism that enforces "styles do not cross
lines" — even if a widget forgot to close a color, the next line starts clean. Also
normalize the line (strip stray control bytes) before appending. Canonical: `applyLineResets`.

## Step 5 — The three strategies

After the pipeline, pick one. Canonical: `tui.ts` `doRender` strategy branches + `fullRender`.

### A. First render — `fullRender(clear = false)`

When `previousLines.length === 0 && !widthChanged && !heightChanged`: write all lines
**without clearing**, so scrollback the user has above the program is preserved.

```
buf = "\x1b[?2026h"
for i, line in newLines: buf += (i>0 ? "\r\n" : "") + line   // image rows special-cased
buf += "\x1b[?2026l"
terminal.write(buf)
cursorRow = hardwareCursorRow = max(0, newLines.length - 1)
maxLinesRendered = newLines.length
previousViewportTop = max(0, max(height, newLines.length) - height)
positionHardwareCursor(cursorPos, newLines.length)
```

### B. Full redraw — `fullRender(clear = true)`

Use when the diff would be unsafe or wasteful: width changed; height changed (except
Termux, where the software keyboard toggles height and a full redraw replays history on
every toggle — detect via `process.env.TERMUX_VERSION`); `clearOnShrink && newLines.length
< maxLinesRendered && no overlays`; first changed line above the viewport; or
`requestRender(true)`.

```
buf = "\x1b[?2026h"
if clear:
  buf += deleteKittyImages(previousKittyImageIds)
  buf += "\x1b[2J\x1b[H\x1b[3J"      // clear screen, home, clear scrollback
for i, line in newLines: buf += (i>0 ? "\r\n" : "") + line
buf += "\x1b[?2026l"
terminal.write(buf)
cursorRow = hardwareCursorRow = max(0, newLines.length - 1)
maxLinesRendered = clear ? newLines.length : max(maxLinesRendered, newLines.length)
previousViewportTop = max(0, max(height, newLines.length) - height)
positionHardwareCursor(cursorPos, newLines.length)
```

### C. Normal diff update

Compute `firstChanged`/`lastChanged` by comparing `previousLines[i]` vs `newLines[i]`
(treat missing lines as `""`). If `newLines.length > previousLines.length` (append),
`firstChanged = previousLines.length` when no earlier change, `lastChanged = newLines.length
- 1`. Expand the range for Kitty image blocks (a multi-row image at line `i` owns the
following empty rows; if any row in the block is touched, redraw the whole block).

Then:
- **No changes** (`firstChanged === -1`): only `positionHardwareCursor`, save state, return.
- **All deletions** (`firstChanged >= newLines.length`): move to end of new content, clear
  extra rows with `\r\n\x1b[2K` per row, move back. If extras > height or target above
  viewport, fall back to B.
- **`firstChanged < previousViewportTop`**: fall back to B (unreachable by diff).
- **Otherwise**: the normal write loop below.

Normal write loop (wrapped in `\x1b[?2026h`…`\x1b[?2026l`):

```
buf += deleteChangedKittyImages(firstChanged, lastChanged)
prevBottom = previousViewportTop + height - 1
moveTarget = appendStart ? firstChanged - 1 : firstChanged
if moveTarget > prevBottom:
  move cursor to viewport bottom; scroll = moveTarget - prevBottom
  buf += "\r\n" * scroll            // newline scrolls the terminal
  previousViewportTop += scroll; hardwareCursorRow = moveTarget
lineDiff = (moveTarget - viewportTop) - (hardwareCursorRow - prevViewportTop)
buf += lineDiff > 0 ? `\x1b[${lineDiff}B` : `\x1b[${-lineDiff}A`
buf += appendStart ? "\r\n" : "\r"
renderEnd = min(lastChanged, newLines.length - 1)
for i in firstChanged..renderEnd:
  if i > firstChanged: buf += "\r\n"
  line = newLines[i]
  if visibleWidth(line) > width: STOP + HARD ERROR (see below)
  buf += "\x1b[2K" + line              // clear line first: new may be shorter than old
let finalCursor = renderEnd
if previousLines.length > newLines.length:   // content shrank
  if renderEnd < newLines.length - 1: move down; finalCursor = newLines.length - 1
  for each extra row: buf += "\r\n\x1b[2K"
  buf += `\x1b[${extra}A`                   // move back to end of new content
buf += "\x1b[?2026l"
terminal.write(buf)
cursorRow = max(0, newLines.length - 1)      // logical end of content
hardwareCursorRow = finalCursor              // where the cursor actually stopped
maxLinesRendered = max(maxLinesRendered, newLines.length)
previousViewportTop = max(prevViewportTop, finalCursor - height + 1)
positionHardwareCursor(cursorPos, newLines.length)
```

The `\x1b[2K` before each line matters: a new line may be shorter than the old one;
without clearing, stale characters from the old line remain at the end. `appendStart` uses
`\r\n` instead of `\r` so the new content starts on a fresh line after the last unchanged
line (the "new message appended" case).

## Step 6 — The hard width invariant

In the **normal diff write loop**, if `visibleWidth(line) > width`: write a crash log (pi
uses `~/.pi/agent/pi-crash.log` with the terminal width, the offending line's width, and
all rendered lines), call `stop()` to restore terminal state, and throw:

> "Rendered line N exceeds terminal width (W > width). This is likely caused by a custom
> TUI component not truncating its output. Use visibleWidth() and truncateToWidth()."

This is load-bearing. Without it, an over-wide line wraps in the terminal, desyncs every
subsequent row, and the diff produces garbage. The crash turns a silent visual corruption
into a loud, fixable bug. Keep it when porting.

**Caveat (see `audit.md` Finding 2)**: in pi-tui today this check exists **only on the diff
path**. `fullRender` (Step 5 A/B) writes `line` with no width check, so an over-wide line
during a full-redraw frame (width change, theme change, shrink, `requestRender(true)`)
would wrap and desync silently. When porting, **put the check in both paths** — factor the
per-line write into one checked helper used by `fullRender` and the diff loop alike. This
closes the hole pi-tui currently has.

## Step 7 — Synchronized output (CSI 2026)

Every multi-line write is wrapped `\x1b[?2026h` … `\x1b[?2026l`. The terminal buffers
between begin/end and applies in one atomic paint. Without it, a slow terminal shows
intermediate cursor moves and partial lines as flicker. Always wrap a render batch, even
the no-change cursor-move path (wrapping is always safe).

## Step 8 — Viewport and scrollback math

The engine models the terminal as a buffer of height `height` looking into a logical
content array. `previousViewportTop` is the index of the top visible row. When content
grows past the viewport, `previousViewportTop` advances (the view scrolls to follow the
end — what you want for a chat/log UI).

- `maxLinesRendered` is the high-water mark of the terminal's *working area*. It grows with
  content, does not shrink unless `fullRender(true)` resets it. When content shrinks below
  it and there are no overlays, the orphaned rows at the bottom are stale; `clearOnShrink`
  triggers `fullRender(true)`. Disable (`PI_CLEAR_ON_SHRINK=0`) on slow terminals.
- On `widthChanged`: always `fullRender(true)`. Wrapping changes every line.
- On `heightChanged`: `fullRender(true)` to keep the viewport aligned — except Termux.

## Step 9 — The virtual IME cursor (`positionHardwareCursor`)

```
positionHardwareCursor(cursorPos, totalLines):
  if !cursorPos || totalLines <= 0: terminal.hideCursor(); return
  row = clamp(cursorPos.row, 0, totalLines - 1); col = max(0, cursorPos.col)
  d = row - hardwareCursorRow
  buf = (d > 0 ? `\x1b[${d}B` : d < 0 ? `\x1b[${-d}A` : "") + `\x1b[${col + 1}G`
  if buf: terminal.write(buf)
  hardwareCursorRow = row
  showHardwareCursor ? terminal.showCursor() : terminal.hideCursor()
```

The hardware cursor stays hidden by default; it's positioned only so terminals that anchor
the IME candidate window to the hardware cursor place it correctly. Enable a visible
hardware cursor with `showHardwareCursor` / `PI_HARDWARE_CURSOR=1` for terminals that
require a visible cursor for IME. `cursorRow`/`hardwareCursorRow` divergence is resolved
here — after positioning, `hardwareCursorRow` reflects the IME position, while `cursorRow`
still tracks end-of-content for the next frame's viewport math.

## Step 10 — `stop()` (clean exit)

```
stop():
  stopped = true; clear renderTimer
  if previousLines.length > 0:
    diff = previousLines.length - hardwareCursorRow
    write `\x1b[${diff}B` if diff > 0 else `\x1b[${-diff}A`
    write "\r\n"                           // move to a fresh line so the shell prompt doesn't overwrite
  terminal.stop()                          // restores raw mode, disables protocols, shows cursor
```

Handle `SIGINT`/`SIGTERM`/`SIGHUP` → `stop()` + exit so a Ctrl+C from another terminal or
a hangup doesn't leave the terminal in raw mode.

## Kitty image handling (porting note)

If you support inline images (Kitty graphics protocol), each image line is an `\x1b_G…`
APC sequence occupying `r` terminal rows. The engine: tracks all image IDs on screen
(`previousKittyImageIds`); before redrawing a range, deletes images whose IDs appear in
the changed range of `previousLines` (`deleteKittyImage(id)` = `\x1b_Ga=d,d=<id>\x1b\\`);
reserves `reservedRows` for a multi-row image (the image line + following empty/image
rows); when writing a multi-row image in the diff path, clears each reserved row, moves
back up, writes the image, moves back down. Canonical: `tui.ts`
`collectKittyImageIds`/`getKittyImageReservedRows`/`expandChangedRangeForKittyImages`/
`deleteChangedKittyImages`, `terminal-image.ts`. If you skip images, keep the set empty and
the image-line checks returning false so the code paths stay uniform.

## Debugging the engine

- `PI_TUI_WRITE_LOG=/path` — log the raw ANSI stream written to stdout (file or directory).
- `PI_DEBUG_REDRAW=1` — append each `fullRender` reason to `~/.pi/agent/pi-debug.log`
  (e.g. "terminal width changed (80 -> 120)", "firstChanged < viewportTop (3 < 5)").
- `PI_TUI_DEBUG=1` — dump each diff render to `/tmp/tui/render-<ts>.log` with
  `firstChanged`, `viewportTop`, `cursorRow`, `newLines`, `previousLines`, the buffer.

Keep equivalent debug hooks when porting — diff engines are impossible to tune without
visibility into *why* a full redraw happened.

## Agreement with the other modules

- **Widgets** (`ui-components.md`) must return lines ≤ `width` (Step 6 enforces it on the
  diff path; **port the check into `fullRender` too** per the Step 6 caveat), emit
  `CURSOR_MARKER` when focused (Step 5's `extractCursorPosition` consumes it), and
  `invalidate()` on theme/force-redraw (Step 3's `requestRender(true)` triggers it).
- **UX** (`ux.md`) routes input to `focusedComponent` then calls `requestRender()`; the
  engine's `handleInput` is the routing point and receives **one already-split sequence
  per call** (splitting is a Terminal-layer responsibility — `StdinBuffer` — see `ux.md`
  Step 1 and `audit.md` Finding 1). Overlays composite in Step 4. The `KeybindingsManager`
  is read by widgets, not the engine.

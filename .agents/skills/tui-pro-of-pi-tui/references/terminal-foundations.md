# Terminal Foundations & Low-Level Libraries

Background for building this infrastructure from scratch or porting it. Covers what a
terminal emulator actually is, the protocols a TUI depends on, the **immediate-vs-retained**
and **cell-grid-vs-line-array** design axes (and where ratatui and pi-tui sit on them), and
which low-level libraries give you what in each language. Read alongside
`rendering-engine.md` (which specifies pi-tui's choices).

## What a terminal emulator presents to a program

From the program's side, a terminal emulator is:

- **A byte stream out (stdout)**: you write ANSI/UTF-8 bytes; the emulator decodes them into
  a grid of cells (each cell = a grapheme + foreground/background color + style attributes).
- **A byte stream in (stdin)**: the user's keystrokes arrive as bytes, possibly as ANSI
  escape sequences (arrows, function keys, modified keys). In **cooked mode** the kernel
  line-buffers and generates signals (Ctrl+C → SIGINT, Ctrl+D → EOF, Ctrl+Z → SIGTSTP). In
  **raw mode** you get every byte as-is, no signals, no buffering — which is what a TUI
  needs, and why the TUI must handle Ctrl+C/exit itself.
- **A grid of `rows × columns` cells** with a cursor position. Dimensions come from
  `ioctl(TIOCGWINSZ)` on Unix / `GetConsoleScreenBufferInfo` on Windows, surfaced in Node as
  `process.stdout.columns`/`rows`. Resize is delivered as `SIGWINCH` (Unix) or
  `WindowBufferSizeEvent` (Windows) — in Node, `stdout.on("resize")`.
- **Scrollback**: lines that scroll off the top of the visible grid accumulate in
  scrollback. A TUI that prints with `\n` pushes content into scrollback; a TUI that
  repaints in place (using `\r`, cursor moves, and `\x1b[2K`) does not. This distinction is
  the whole reason pi-tui's diff engine exists.

A TUI is a program that takes over the visible grid, repaints it in place each frame, and
restores the terminal to a usable state on exit. Everything below is in service of that.

## The protocols a TUI depends on

You do not need a terminal emulator *library* to build a TUI — you need to emit/parse these
byte sequences. pi-tui emits them directly from the engine; a low-level lib just wraps the
same bytes.

| Protocol | Sequence | Purpose |
|---|---|---|
| SGR (colors/style) | `\x1b[<n>m` | Set foreground/background/bold/italic/reverse… |
| Cursor move | `\x1b[<n>A/B/C/D` (up/down/right/left), `\x1b[<col>G`, `\x1b[<row>;<col>H` | Position the cursor |
| Line clear | `\x1b[2K` (whole line), `\x1b[K` (to end), `\x1b[1K` (to start) | Erase before repainting |
| Screen clear | `\x1b[2J` + `\x1b[H` (home), `\x1b[3J` (scrollback) | Full redraw / clear scrollback |
| Synchronized output | `\x1b[?2026h` … `\x1b[?2026l` (CSI 2026) | Terminal buffers between begin/end → atomic, no flicker |
| Bracketed paste | `\x1b[?2004h/l` + `\x1b[200~`…`\x1b[201~` | Detect paste vs typed input |
| Kitty keyboard | `\x1b[>7u` (push flags 1+2+4), `\x1b[?u` (query), `\x1b[<u` (pop) | Disambiguate escapes, event types, alternate keys |
| modifyOtherKeys | `\x1b[>4;2m` (enable), `\x1b[>4;0m` (disable) | xterm enhanced modifier reporting (Kitty fallback) |
| OSC 11 | `\x1b]11;?\x07` | Query default background color |
| OSC 996/997 | `\x1b[?996n` | Query color scheme (dark/light) |
| OSC 0/2 | `\x1b]0;<title>\x07` | Set window title |
| OSC 9;4 | `\x1b]9;4;3\x07` | Indeterminate progress (Windows Terminal/ConEmu) |
| Kitty graphics | `\x1b_G...<payload>...\x1b\\` | Inline images (PNG/JPEG/GIF/WebP) |
| iTerm2 inline images | `\x1b]1337;File=...` | Inline images (iTerm2/Ghostty/WezTerm) |
| DA (Device Attributes) | `\x1b[c` | Capability query — used as a sentinel in Kitty negotiation |
| Cell size | `\x1b[16t` → `\x1b[6;<h>;<w>t` | Pixel size of a cell (for image scaling) |

**Progressive enhancement**: a TUI should work with only SGR + cursor moves + line/screen
clear (any ANSI terminal). Synchronized output, bracketed paste, Kitty keyboard, OSC
queries, and image protocols are enhancements — detect or negotiate them, degrade
gracefully. pi-tui's `ProcessTerminal` negotiates Kitty keyboard with a `modifyOtherKeys`
fallback; `getCapabilities()` (terminal-image.ts) probes for image support.

## The two design axes

### Axis 1 — Widget tree: immediate vs retained

This is the axis the user flagged with ratatui.

**Immediate-mode widgets (ratatui, tui-rs, many game UIs)**:
- Every frame, the app rebuilds the entire UI by calling stateless widget constructors that
  draw into a buffer. Widgets are *values* you create on the spot:
  `frame.render_widget(Paragraph::new(text), area)`.
- No widget object survives between frames. **All state lives in your app struct** (the
  single source of truth); widgets are pure functions of `(state, area) → cells`.
- No invalidation bookkeeping: you always re-describe the whole UI, so it's always correct.
- Mental model: "the UI is a function of state." Composes declaratively. Cheap to
  re-render because widgets are small values and the cell diff (Axis 2) is cheap.
- Cost: deeply stateful widgets (an editor with undo, kill-ring, autocomplete, paste
  markers) must externalize all their state into the app, which gets verbose.

**Retained-mode widgets (pi-tui, Qt, the DOM)**:
- Widgets are long-lived objects with identity, mutable state, and a render cache.
  `const editor = new Editor(tui, theme); editor.onSubmit = …`. You mutate them
  (`editor.setText`, `selectList.setFilter`) and call `invalidate()` when their render
  cache is stale.
- The widget tree persists across frames. State lives *on* the widget.
- Invalidation is explicit (`invalidate()` + the theming-on-invalidate rebuild pattern).
- Mental model: "the UI is a tree of objects." Natural fit for stateful widgets — pi-tui's
  `Editor` (cursor, undo stack, kill ring, autocomplete, paste markers, height-aware
  scroll) is deeply stateful and would be painful to externalize into an app struct each
  frame.
- Cost: invalidation discipline (cache keys, rebuild-on-invalidate) and the risk of stale
  caches if a widget forgets to invalidate.

**Where each sits**:
- **ratatui**: immediate widget tree, retained cell buffer (diffed).
- **pi-tui**: retained widget tree (with per-widget render cache), retained line array (diffed).
- Both retain *the screen representation* for diffing — neither redraws blindly. The
  difference is whether widgets are persistent objects or ephemeral draw calls.

**Choosing**:
- Mostly stateless widgets (dashboards, lists, tables, menus) → immediate (ratatui) is
  simpler and less error-prone.
- Deeply stateful widgets (rich text editors, interactive selectors with their own input)
  → retained (pi-tui) avoids externalizing a pile of editor state into the app each frame.
- Hybrid is common: ratatui apps keep an `App` struct that holds editor-like state and
  pass it to stateless widgets; pi-tui could host immediate widgets too (any `Component`
  whose `render` recomputes from scratch and ignores caching is effectively immediate).

### Axis 2 — Screen representation: cell grid vs line array

This axis is orthogonal to widget mode and matters more for the engine implementation.

**Cell-grid representation (ratatui, ncurses, tcell, notcurses)**:
- The screen is a 2D array of `Cell { grapheme, fg, bg, style }` of size `rows × columns`.
- Each frame: build a new `Buffer` by having widgets write cells into rectangular areas.
  Diff per-cell against the previous buffer; emit SGR + grapheme runs only for changed
  cells (run-length encode adjacent cells with the same style to minimize SGR escapes).
- Pro: fine-grained diff — a single changed character within a line updates only that cell.
- Pro: width is enforced by the grid (you can't write past column `width`; the area clips).
- Pro: the library handles cell → ANSI encoding; widgets think in cells, not escapes.
- Con: more memory (`rows × cols` cells per buffer, two buffers for diffing).
- Con: widgets must render into a `Cell` grid, not emit styled strings — so you can't
  reuse a markdown/ANSI renderer that outputs styled text without an ANSI→cell parser.
- Con: re-encoding styled cells to an efficient ANSI stream (SGR run-length) is non-trivial.

**Line-array representation (pi-tui)**:
- The screen is a 1D array of strings (`string[]`), each string a pre-rendered line with
  ANSI escapes embedded.
- Each frame: the widget tree returns `string[]`; the engine diffs by **string equality
  per line** and rewrites the whole changed line(s) with `\x1b[2K` + line.
- Pro: simpler diff (string equality, no cell comparison).
- Pro: widgets emit already-styled strings; you can reuse any ANSI-producing renderer
  (chalk, marked, syntax highlighters) directly.
- Pro: less memory (just the line strings).
- Con: coarser diff — one character change rewrites the whole line (mitigated by the 16ms
  coalesced render + the fact that terminal I/O is the bottleneck, not the diff).
- Con: **widgets must enforce the width contract themselves** (`truncateToWidth`,
  `wrapTextWithAnsi`) — there's no cell grid to clip into. This is why pi-tui has the hard
  width error: it's the backstop for the contract a cell grid would enforce structurally.
- Con: `visibleWidth` must parse ANSI to measure display width (a cell grid knows widths
  by construction).

**Where each sits**:
- **ratatui**: cell grid (`Buffer`/`Cell`), per-cell diff, run-length SGR encoding.
- **pi-tui**: line array (`string[]`), per-line string-equality diff, widgets emit styled
  strings.
- **ncurses/tcell/notcurses**: cell grid, per-cell diff (the classic approach).

**Choosing**:
- If your low-level lib already gives you a cell grid (ncurses, tcell, notcurses,
  @xterm/headless), use it — fighting the grid to do line-array diffing wastes the lib.
- If your lib is a thin ANSI wrapper (crossterm, termion, raw `process.stdout`), the
  line-array approach (pi-tui) is simpler to build than a cell grid + run-length encoder.
- If you need inline images or per-cell styling precision, a cell grid helps; if you need
  to reuse existing ANSI renderers (markdown, syntax highlighting), line arrays help.

## Low-level libraries by language (what each gives you)

The tiers: **raw I/O** (just raw mode + bytes + events) → **cell grid** (raw I/O + a
`Buffer`/cell diff layer) → **widget framework** (cell grid + a widget system). Pick the
lowest tier that covers what you need; building pi-tui's engine on a raw I/O lib is
straightforward, on a cell-grid lib you'd either use the grid (and become ratatui-like) or
bypass it (wasteful).

### Rust
- **`crossterm`** — raw I/O tier. Cross-platform raw mode, event reading (keys/mouse/resize),
  ANSI emission helpers. The most common backend for ratatui. **Good base for a pi-tui
  port**: you own the line array + diff, crossterm handles raw mode + events.
- **`termion`** — raw I/O tier, Unix-only, lighter than crossterm.
- **`ratatui`** — widget framework tier, **immediate-mode**, cell-grid. If you're using
  ratatui you're not porting pi-tui's engine — you're adopting ratatui's model. Useful to
  study for its `Buffer`/`Cell` diff and its widget catalog; the widget *behaviors*
  (editor, list, table) are portable, the widget *mode* is not.
- **`ansi-to-tui`** — parses ANSI-styled text into ratatui `Cell`s. Relevant if you want
  pi-tui's "reuse ANSI renderers" benefit on top of ratatui's cell grid.

### Go
- **`tcell`** — cell-grid tier (a terminal emulator in process: it maintains a screen
  buffer, diffs per-cell, emits ANSI). Mode: widgets are usually immediate (you implement
  `Draw` on your app). Using tcell means you get the cell grid; a pi-tui-style line-array
  engine would bypass it.
- **`termbox-go`** — cell-grid tier, termbox port, simpler than tcell.
- **`bubbletea`** — widget framework tier (Elm architecture: `Model`/`Update`/`View`),
  **retained-model, immediate-view**. State lives in `Model`; the `View` is rebuilt each
  frame from state using `lipgloss` styling. Closer to ratatui's mode than pi-tui's.
- **`lipgloss`** — styling library (ANSI helpers), not a framework. Usable under any mode.

### Python
- **`curses`** (stdlib) — cell-grid tier, the classic. Mode: usually immediate (you redraw
  windows each frame). Cross-platform via the stdlib but Windows support is limited.
- **`urwid`** — widget framework tier, **retained-mode** widgets (closest Python analog to
  pi-tui's widget mode), with a cell-based renderer underneath.
- **`textual`** — widget framework tier, **retained-mode** widgets on async, CSS-like
  styling. The most pi-tui-like in widget philosophy; its rendering is cell-based.
- **`rich`** — rendering library (ANSI output), not a framework. Usable as the "styled
  string" producer under a pi-tui-style line-array engine.

### Node / TypeScript
- **`process.stdin/stdout` + `readline`** — raw I/O tier (what pi-tui's `ProcessTerminal`
  builds on). `setRawMode(true)`, emit ANSI directly. This is the canonical base for a
  pi-tui port in TS.
- **`node-pty`** — pseudo-terminal spawning (for testing or hosting shells), not a UI lib.
- **`@xterm/headless`** — a headless terminal emulator (cell grid) with no I/O. pi-tui
  uses it for `VirtualTerminal` tests: feed it the ANSI stream the engine emits, assert on
  the resulting cell grid. **The recommended test harness for any port.**
- **blessed / blessed-contrib** — older widget framework, retained widgets, less maintained.

### C / C++
- **ncurses** — cell-grid tier, the classic. Mode: usually immediate (you redraw windows).
- **notcurses** — cell-grid tier, richer than ncurses (images, multimedia), per-cell diff.
- **libtickit** — terminal abstraction tier (raw I/O + cell grid), smaller surface.

### Zig
- **libzterm** / community TUI libs — emerging; check current state. Raw I/O tier is easy
  to build directly from termios + write.

## Building pi-tui's engine on each tier

### On a raw I/O lib (crossterm, termion, raw Node)
This is the direct port. You implement:
- `Terminal` (raw mode, resize, write, cursor) — wraps the lib's raw-mode + event APIs.
- `StdinBuffer` — split batched stdin into complete sequences (the lib may already do this;
  crossterm's `event::read` yields one `Event` at a time, so you may not need a buffer).
- `TUI` — line array + diff + synchronized output + the three strategies + hard width
  error + virtual IME cursor, exactly as `rendering-engine.md` specifies.
- `visibleWidth`/`truncateToWidth`/`wrapTextWithAnsi` — needed because there's no cell grid
  to enforce width structurally.
This is the path that most faithfully reproduces pi-tui. pi-tui itself is on this tier.

#### Rust + crossterm specifics (the cost case for a retained-mode port)

A Rust port on crossterm is materially cheaper than pi-tui was to build, because crossterm
**collapses pi-tui's hardest module — the cross-protocol key model — to near-zero**.

**What crossterm gives you for free (what pi-tui had to build by hand):**
- The key model. `event::read()` returns `KeyEvent { code, modifiers, kind, state }`, not
  raw bytes. You match `KeyCode::Char('c') + KeyModifiers::CONTROL`, `KeyCode::Enter`,
  `KeyCode::Up`, etc. — no `matchesKey` over legacy VT / `modifyOtherKeys` / Kitty CSI-u.
  pi-tui spends ~1400 lines in `keys.ts` + the `StdinBuffer` CSI/OSC/DCS/APC splitter on
  exactly this. crossterm's `PushKeyboardEnhancementFlags` + `KeyEventKind::Press/Repeat/`
  `Release` covers the Kitty event-type flags too. **You do not need `keys.ts` or
  `stdin-buffer.ts` at all.**
- Bracketed paste: `Event::Paste(String)` is one event, no `[200~`…`[201~`
  buffering/re-wrapping.
- Synchronized output: `BeginSynchronizedUpdate`/`EndSynchronizedUpdate`.
- Raw mode, resize events, mouse events, alternate screen, cursor show/hide.

So the three pi-tui modules that are pure protocol plumbing (`terminal.ts`,
`stdin-buffer.ts`, `keys.ts`) become thin crossterm wrappers. You still must build:
- **Width utilities** with `unicode-width` + `unicode-segmentation` + a small ANSI state
  machine for `stripAnsi`/`wrapTextWithAnsi`'s SGR carry. ~600–1000 lines. The one real
  cross-protocol correctness burden that stays.
- **Line-array diff engine** (three strategies + sync output + hard width error + IME
  cursor). ~500–800 lines.
- **Overlay system** — minimal ~200 lines, full focus-restore state machine ~600.
- **Widget catalog**, by need; `Editor` is the heavy one.

Rough budget: MVP ~1.5–3k lines, full ~3–5k lines — versus pi-tui's ~7k across the same
scope (much of the delta is the protocol plumbing crossterm absorbs).

**Important: do not over-credit pi-tui here.** ratatui *also* does differential rendering —
it diffs the cell buffer and emits only changed cells. "Diff rendering" is not pi-tui's
exclusive win. pi-tui's real, exclusive wins over ratatui are:
1. **Widget render caching** — long markdown / syntax-highlighted output isn't re-wrapped
   and re-highlighted every frame. This is the one that bites at long context lengths.
2. **Reuse ANSI renderers directly** — `syntect`/a markdown crate emits styled text you
   use as a line verbatim; no ANSI→cell parser, no rewrite to ratatui `Span`/`Cell`.
3. **Deeply stateful widgets are natural** — editor undo/kill-ring/autocomplete/paste-
   markers/height-aware scroll live on the widget, not externalized to an `App` struct each
   frame.
4. **pi-tui's exact behaviors** — in-place diff without scrollback pollution, the virtual
   IME cursor marker, the overlay focus-restore state machine.

These wins are real but narrow. They matter most for **editor-centric apps with long
context** (a coding-agent / chat TUI). For a general TUI (dashboards, lists, tables, menus,
forms) they rarely pay off, and ratatui gives you a cell grid + layout engine + widget
catalog + ecosystem for free.

See the decision table below.

### On a cell-grid lib (tcell, ncurses, notcurses, @xterm/headless)
You have a choice:
- **Embrace the grid** (becomes ratatui-like): widgets write cells into the grid each
  frame; the lib diffs and emits. You give up pi-tui's "reuse ANSI renderers" benefit and
  its line-array simplicity, but you get structural width enforcement and finer-grained
  diffing. The widget *behaviors* from `ui-components.md` still apply, but widgets implement
  `render(area, buffer)` instead of `render(width) => string[]`.
- **Bypass the grid** (build pi-tui's line-array engine anyway): ignore the lib's cell
  buffer, use only its raw I/O, and emit ANSI to `terminal.write`. This works but wastes
  the lib. Only do this if you're locked into the lib for raw-mode/event reasons but want
  pi-tui's exact engine.

### On a widget framework (ratatui, bubbletea, textual, urwid)
You're adopting that framework's mode, not porting pi-tui's engine. What you *can* port:
- The widget **behaviors** (editor with kill-ring/undo/paste-markers, select list with
  wrap navigation, settings list with submenus) — reimplement on the framework's widget
  primitives.
- The **UX layer** (`ux.md`): the key model, keybinding registry, single-focus routing,
  overlay stack. Most frameworks have their own focus/event system; map pi-tui's concepts
  onto them.
- The **invariants** (width contract, styles-per-line, IME cursor positioning) still apply
  and may already be handled by the framework.

### Combining modes (what's worth hybridizing)

- **Widget-tree hybrid (free, recommended)**: a retained `Editor` (self-managed state) and
  an immediate header (recomputed from app state each frame) in the *same* tree. pi-tui
  already supports this — `Component` doesn't mandate caching, so any widget whose `render`
  recomputes from external state is an immediate widget. The engine only calls `render(width)`
  and diffs. Do this by default; no mode purity is worth enforcing.
- **Representation hybrid (only one combo pays off)**: *line-array production + cell-grid
  test oracle*. pi-tui does this — the engine emits an ANSI stream, tests feed it to
  `@xterm/headless` and assert on the resulting cell grid. This is the one worth stealing,
  and it's independent of widget mode. The reverse (cell-grid production + an ANSI→cell
  parser to keep "widgets emit ANSI strings") is what `ansi-to-tui` does for ratatui;
  viable but you pay for an ANSI state machine and lose the simplicity of "what the widget
  emits is what the terminal receives". Mixed-granularity diff (cell-diff for complex lines,
  line-diff for simple ones) is an optimization, not a mode — only if profiling shows
  line-level diff is the bottleneck (it almost never is; terminal I/O is).
- **Two engines coexisting** is not coherent. You pick one engine. But "retained state
  objects + immediate drawing" (ratatui holding a long-lived `Editor` struct, redrawing it
  each frame) is *normal ratatui usage*, not two engines — that's the middle path below.

### Decision: when the retained-on-raw-I/O port is worth it

| Your target | Recommendation |
|---|---|
| Editor-centric app with long context (coding-agent / chat TUI: rich editor + long markdown history) | **Retained on crossterm, high payoff.** Widget render caching is real at long context; editor state on the widget is natural; crossterm gives you the key model for free. Start from pi-tui's architecture. |
| General TUI (dashboards, lists, tables, menus, forms; no rich editor) | **Use ratatui**, keep state in an `App` struct, draw immediately. Don't build an engine. ratatui's cell-grid diff + layout + widget catalog + ecosystem are free. |
| Want to reuse an ANSI-rendering pipeline (syntect, a markdown crate) but otherwise happy with ratatui | **ratatui + `ansi-to-tui` bridge.** This is the practical way to get pi-tui's "renderer reuse" win inside ratatui without building an engine. |
| Time-boxed, unsure | **Start with ratatui + retained `App` state** (the Elm-ish middle path). Only drop to crossterm + a hand-built line-array engine when you hit a concrete wall: measurable cost of re-rendering long markdown every frame, a need for pi-tui's exact overlay semantics, or an ANSI renderer that's painful to convert to `Span`s. |

**The middle path, expanded**: most apps land best on *ratatui + retained state objects +
immediate drawing*. You take ratatui's cell-grid diff, layout, and widget catalog for free,
and make your editor a struct that persists across frames (retained state), redrawing it
each frame from that state. You give up pi-tui's "widget self-manages a render cache + the
engine diffs render output" — but ratatui recomputes every frame anyway, so there's no
invalidation protocol to get wrong (one fewer class of bug). Drop to crossterm + a
line-array retained engine only when long-context re-rendering is a measured bottleneck
or you genuinely need to feed ANSI renderers through without a converter. Editor-centric
long-context apps hit exactly those conditions; general TUIs usually don't.

## Testing strategy (independent of tier)

A diff engine is impossible to test deterministically against a real terminal. Use a
**headless terminal emulator** as a `VirtualTerminal`:
- Feed it the exact ANSI stream the engine emits (`terminal.write` bytes).
- Assert on the resulting cell grid (cursor position, cell content, styles).
- Replay recorded stdin sequences (including batched and partial chunks) to test the
  splitter and key model.

pi-tui uses `@xterm/headless` for this (`packages/tui/test/virtual-terminal.ts`). In Rust,
`xterm` headless or a custom cell grid works; in Go, `tcell` in simulation mode; in Python,
`pyte` (a terminal emulator parser). The test patterns in `packages/tui/test/*.test.ts`
(tui-render, overlay-options, shrink, CJK boundary, etc.) are directly portable as
assertion shapes.

## Negotiation and capability detection (do this at startup)

When building from a low-level lib, replicate pi-tui's startup negotiation
(`rendering-engine.md` Step 1):
1. Enter raw mode.
2. Enable bracketed paste; remember to disable on exit.
3. Negotiate Kitty keyboard (push flags, query, DA sentinel) → enable Kitty or fall back to
   `modifyOtherKeys`. Detect via the response sequence.
4. (Optional) Query OSC 11 background, OSC 996 color scheme, CSI 16 cell size.
5. (Optional) Probe image support via `getCapabilities()` (DA responses + terminfo).

On exit: disable bracketed paste, pop Kitty keyboard, disable `modifyOtherKeys`, restore
raw mode, show cursor, move to end of content, drain stdin over slow SSH. Handle
`SIGINT`/`SIGTERM`/`SIGHUP` so a signal doesn't leave the terminal in raw mode.

## How this maps back to the skill

- `rendering-engine.md` specifies the **line-array + retained-widget** choice (pi-tui's
  position on both axes). If you're porting to a cell-grid lib, read this file as "the
  behavior to reproduce" and adapt the representation.
- `ui-components.md` specifies the widget contract for **retained** widgets. For an
  immediate-mode port, the widget *behaviors* still apply but the contract changes (widgets
  become stateless draw functions; state moves to the app). The widget-tree hybrid
  (retained + immediate in one tree) needs no special support.
- `ux.md` is mode-agnostic — key model, keybindings, focus, overlays apply regardless of
  widget mode or screen representation. On crossterm specifically, the key model and stdin
  splitting are absorbed by the lib (see "Rust + crossterm specifics" above), so `ux.md`
  Steps 1–2 become thin wrappers.
- `audit.md` records where pi-tui's implementation diverges from what these specs claim, so
  a port doesn't inherit the gaps.
- The **decision table** above is the short answer to "should I build a retained-on-raw-I/O
  engine or just use ratatui?" — re-read it before committing to a port.

# Module 2 — UI Components / Widgets

How to build pi's widget system from scratch so it composes with the render engine
(`rendering-engine.md`) and the interaction layer (`ux.md`). Canonical implementations are
under `packages/tui/src/components/`; open them alongside this spec.

## What this module is responsible for

Define the `Component` contract that the engine renders, provide a widget catalog that
covers the common terminal UI needs, and enforce the shared invariants (width contract,
styles-per-line, cache + invalidate, theming-on-invalidate). Widgets do **not** touch the
terminal directly, do **not** schedule renders (except `Loader`'s animation timer), and do
**not** interpret keys except via the UX layer's `matchesKey`/`KeybindingsManager`.

## Build target

```typescript
interface Component {
  render(width: number): string[];     // REQUIRED: each line's display width MUST be <= width
  handleInput?(data: string): void;    // OPTIONAL: keyboard input when focused
  wantsKeyRelease?: boolean;            // OPTIONAL: receive Kitty key-release events (default false)
  invalidate(): void;                   // REQUIRED: clear cached render state
}

interface Focusable {
  focused: boolean;                     // set by the engine; widget emits CURSOR_MARKER when true
}

const CURSOR_MARKER = "\x1b_pi:c\x07";  // zero-width APC; engine strips it and positions the hardware cursor

class Container implements Component { children: Component[]; addChild/removeChild/clear; ... }
```

Plus the built-in widgets: `Text`, `TruncatedText`, `Box`, `Spacer`, `Container`,
`Input`, `Editor`, `SelectList`, `SettingsList`, `Markdown`, `Loader`,
`CancellableLoader`, `Image`.

## Step 0 — Implement the width utilities FIRST (mandatory)

No widget is correct without these. Canonical: `packages/tui/src/utils.ts`. Use
`Intl.Segmenter` (granularity "grapheme") + an ANSI escape parser + an East-Asian-Width
table (`get-east-asian-width`). Do not use `String.length` or `substring`.

- **`visibleWidth(str): number`** — display width: ANSI/SGR/OSC/APC = 0, `CURSOR_MARKER` =
  0, narrow = 1, wide (CJK, emoji, regional indicators) = 2, combining marks = 0. Canonical
  helpers: `getGraphemeSegmenter`, `stripAnsi`-equivalent inside `visibleWidth`.
- **`truncateToWidth(str, width, ellipsis = "…"): string`** — truncate to `width` display
  columns, preserve + close ANSI, optional ellipsis (pass `""` for none). Must handle
  wide chars at the boundary (don't split a wide char). Canonical: `utils.ts` `truncateToWidth`.
- **`wrapTextWithAnsi(text, width): string[]`** — word-wrap to `width`, preserving ANSI
  across breaks by carrying SGR state to the next line. Canonical: `utils.ts` `wrapTextWithAnsi`.
- **`sliceByColumn(line, startCol, length, strict = false): string`** — ANSI-aware
  substring by display column. `strict` excludes a wide char that would be split at the
  boundary. Canonical: `utils.ts` `sliceByColumn`.
- **`sliceWithWidth(line, startCol, width, strict = false): { text, width }`** — like
  `sliceByColumn` but also returns the actual display width (may be < `width` if a wide
  char was excluded). Used by overlay compositing.
- **`extractSegments(line, startCol, endCol, afterWidth, strict): { before, beforeWidth, after, afterWidth }`** —
  split a line for overlay splicing. Canonical: `utils.ts` `extractSegments`.
- **`applyBackgroundToLine(line, width, bgFn): string`** — pad to `width` with spaces and
  wrap the whole thing (spaces included) in `bgFn` so the background fills the terminal
  width. Used by `Box`/`Text`.

Unit-test these in isolation before any widget. Smallest meaningful checks:
`visibleWidth("\x1b[31mhi\x1b[0m") === 2`, `visibleWidth("你好") === 4`,
`truncateToWidth("Hello World", 8)` === `"Hello…"` (default) / `"Hello Wo"` (with `""`),
`wrapTextWithAnsi` keeps a colored paragraph colored on every wrapped line.

## Step 1 — `Container` (the only layout primitive for base content)

```typescript
class Container implements Component {
  children: Component[] = [];
  addChild(c: Component) { this.children.push(c); }
  removeChild(c: Component) { /* splice by identity */ }
  clear() { this.children = []; }
  invalidate() { for (const c of this.children) c.invalidate?.(); }
  render(width: number): string[] {
    const lines: string[] = [];
    for (const c of this.children) for (const l of c.render(width)) lines.push(l);
    return lines;
  }
}
```

Base layout is a vertical stack of line arrays. No flex, no absolute positioning — that's
overlays only (`ux.md`). `TUI extends Container`, so the root is a container.

## Step 2 — The render-cache + invalidate protocol

Every widget should cache its rendered line array keyed by `(content signature, width)`
and clear the cache in `invalidate()`. A resize changes `width` and invalidates implicitly
(the equality check fails); `invalidate()` is still required for theme changes and
`requestRender(true)`. Canonical example: `components/text.ts`.

```typescript
class Text implements Component {
  private cachedText?: string; private cachedWidth?: number; private cachedLines?: string[];
  setText(t: string) { this.text = t; this.invalidate(); }
  invalidate() { this.cachedText = this.cachedWidth = this.cachedLines = undefined; }
  render(width: number): string[] {
    if (this.cachedLines && this.cachedText === this.text && this.cachedWidth === width) return this.cachedLines;
    // ...compute, then cache:
    this.cachedText = this.text; this.cachedWidth = width; this.cachedLines = result;
    return result;
  }
}
```

Rules:
1. Cache key is `(content signature, width)`.
2. Mutators (`setText`, `setBgFn`, selection change, …) call `invalidate()`.
3. `invalidate()` clears the cache unconditionally.
4. The engine calls `invalidate()` on theme change and on `requestRender(true)`.
5. After a state change in `handleInput`, call `this.invalidate()`; the engine calls
   `requestRender()` for you after routing input. A consuming input listener (see `ux.md`)
   must call `requestRender()` itself since the engine returns before the focused-widget path.

## Step 3 — Theming-on-invalidate (the rebuild pattern)

Widgets receive a `theme` object (via a render callback parameter, not a direct import —
the active theme can change at runtime). Theme methods return styled strings:
`theme.fg("accent", "text")`, `theme.bg("selectedBg", "text")`.

The hazard: if you pre-bake themed strings into child widgets and cache them, a theme
change leaves stale ANSI in the cache. Just clearing the render cache isn't enough — the
*content* itself contains old colors.

Wrong (theme change has no effect):
```typescript
class Bad extends Container {
  constructor(msg: string, theme: Theme) { super(); this.addChild(new Text(theme.fg("accent", msg), 1, 0)); }
}
```

Right (rebuild on invalidate):
```typescript
class Good extends Container {
  private msg: string; private content: Text;
  constructor(msg: string) { super(); this.msg = msg; this.content = new Text("", 1, 0); this.addChild(this.content); this.updateDisplay(); }
  private updateDisplay() { this.content.setText(theme.fg("accent", this.msg)); }
  override invalidate() { super.invalidate(); this.updateDisplay(); }
}
```

When this matters: pre-baking theme colors, syntax highlighting (`highlightCode`), complex
themed layouts. When it doesn't: stateless render (compute themed output fresh each
`render()`), theme callbacks passed as functions and called during render, simple
containers with no themed content. Canonical discussion: `packages/coding-agent/docs/tui.md`
§"Invalidation and Theme Changes".

## Step 4 — `Focusable` and the IME cursor marker

Widgets that show a text cursor and need IME (CJK input) implement `Focusable` and emit
`CURSOR_MARKER` immediately before the on-screen cursor position when `focused` is true:

```typescript
class MyInput implements Component, Focusable {
  focused = false;                      // set by the engine
  render(width: number): string[] {
    const marker = this.focused ? CURSOR_MARKER : "";
    // fake cursor: reverse-video the glyph under the cursor
    return [`${beforeCursor}${marker}\x1b[7m${atCursor}\x1b[27m${afterCursor}`];
  }
}
```

The engine scans the visible viewport for the marker, computes `col = visibleWidth(before
marker)`, strips it, and positions the hardware cursor there (`rendering-engine.md`
Step 9). The hardware cursor stays hidden; it's positioned only for IME.

**Containers with embedded inputs**: if a container (dialog, selector with a search box)
holds an `Input`/`Editor` child, the container must implement `Focusable` and propagate
`focused` to the child, or the IME candidate window appears in the wrong place:

```typescript
class SearchDialog extends Container implements Focusable {
  private searchInput: Input;
  private _focused = false;
  get focused() { return this._focused; }
  set focused(v: boolean) { this._focused = v; this.searchInput.focused = v; }   // propagate
  constructor() { super(); this.searchInput = new Input(); this.addChild(this.searchInput); }
}
```

Canonical: `components/input.ts`, `components/editor.ts` (both implement `Focusable`).

## Step 5 — The widget catalog

Build these in order of dependency. Each lists its responsibility and the invariant that
makes it composable. Open the canonical file for the full behavior.

### `Text` — `components/text.ts`
Multi-line text with word wrap, padding, optional background. The base display widget.
Wraps with `wrapTextWithAnsi`, pads margins, applies `customBgFn` via
`applyBackgroundToLine`, adds top/bottom padding rows. Caches by `(text, width)`.
`setText`/`setCustomBgFn` invalidate.

### `TruncatedText` — `components/truncated-text.ts`
Single-line text that truncates to viewport width. Use for status lines, headers. No wrap.

### `Box` — `components/box.ts`
`Container` with padding and a background applied to all children. `paddingX`/`paddingY`
inset the content; `bgFn` fills the full width. `setBgFn` to change background dynamically.

### `Spacer` — `components/spacer.ts`
`n` empty lines (default 1). The only layout adjustment for base content.

### `Container` — `tui.ts`
Groups children vertically (Step 1). `TUI` itself is a `Container`.

### `Input` — `components/input.ts`
Single-line `Focusable` text input. Features: horizontal scrolling when content exceeds
width (keeps cursor in view with a half-width bias), grapheme-aware cursor movement
(`Intl.Segmenter`), Emacs-style kill/yank via `KillRing`, undo via `UndoStack` with
word-coalescing, bracketed-paste handling (`\x1b[200~`…`\x1b[201~`), Kitty CSI-u printable
decoding (`decodeKittyPrintable`) so plain letters arriving as `\x1b[97u` still insert,
control-char rejection (C0/DEL/C1). Reads keybindings via `getKeybindings()` (see `ux.md`)
— never hardcodes keys. `onSubmit`/`onEscape` callbacks. Render emits `CURSOR_MARKER` +
reverse-video glyph under the cursor.

### `Editor` — `components/editor.ts` (~2300 lines, the largest widget)
Multi-line `Focusable` editor. Features: word wrap with CJK break opportunities
(`cjkBreakRegex`), slash-command + file-path autocomplete (`AutocompleteProvider` +
`CombinedAutocompleteProvider`), large-paste markers (`[paste #1 +50 lines]` treated as
atomic graphemes via `segmentWithMarkers`), height-aware vertical scrolling (uses the `TUI`
ref to know terminal height), jump-to-char (`Ctrl+]`), page up/down, full Emacs
kill/yank/undo, `disableSubmit` toggle, dynamic `borderColor`/`paddingX`. The canonical
example of a stateful widget that touches most of the UX layer.

### `SelectList` — `components/select-list.ts`
Keyboard-navigable list. `SelectItem { value, label, description? }`. Features: filter
(prefix match), wrap-around navigation (top→bottom on Up at top), scroll window with
`(index/total)` indicator, two-column layout (label + description) when width > 40 with
dynamic primary-column width, single-column fallback otherwise, `onSelect`/`onCancel`/
`onSelectionChange`. Reads `tui.select.*` keybindings. No `Focusable` (no text cursor).

### `SettingsList` — `components/settings-list.ts`
Toggle/cycle panel with submenus. `SettingItem { id, label, description?, currentValue,
values?, submenu? }`. Enter/Space cycles `values` or opens `submenu(currentValue, done)`.
Fuzzy search by label (optional). Reads `tui.select.*` keybindings.

### `Markdown` — `components/markdown.ts`
Renders markdown with theming. `MarkdownTheme` has per-element color functions (heading,
link, code, codeBlock, quote, hr, listBullet, bold, italic, …) and an optional
`highlightCode(code, lang) => string[]` hook for syntax highlighting. HTML tags render as
plain text. Render caching. Use `getMarkdownTheme()` from `@earendil-works/pi-coding-agent`
for the themed instance.

### `Loader` / `CancellableLoader` — `components/loader.ts`, `components/cancellable-loader.ts`
Animated spinner extending `Text`. `frames` + `intervalMs` (default Braille spinner 80ms).
`Loader` drives its own animation via `setInterval` and calls `tui.requestRender()` each
frame (the one exception to "widgets don't schedule renders"). `CancellableLoader` adds
Escape handling + an `AbortSignal` (`signal`/`aborted`/`onAbort`) for cancellable async
operations.

### `Image` — `components/image.ts`
Inline image for Kitty/iTerm2/Ghostty/WezTerm/Warp terminals. `ImageTheme.fallbackColor`
for unsupported terminals. Parses PNG/JPEG/GIF/WebP dimensions from headers, computes cell
rows via `calculateImageRows`, falls back to a placeholder. Depends on
`terminal-image.ts` capabilities detection.

## Step 6 — Widget invariants (the composable contract)

1. **Width**: every `render(width)` line has `visibleWidth(line) <= width`. Use
   `truncateToWidth(line, width, "")` as the final safeguard. Overflow is a hard engine
   error (`rendering-engine.md` Step 6).
2. **Styles per line**: re-open colors/styles at the start of every line. For multi-line
   styled text, use `wrapTextWithAnsi` (carries SGR across breaks) or manually re-apply.
   The engine appends a reset per line, so styles never cross lines, but a forgotten
   close within a line still bleeds to the end of that line.
3. **No direct terminal writes**: never `process.stdout.write`/`console.log` from a widget.
   All output is the `render()` return value.
4. **No render scheduling from `render()`**: calling `requestRender()` inside `render()`
   is an infinite loop. `Loader`'s `setInterval` is the only sanctioned self-scheduling,
   and it calls `requestRender()` from the timer callback, not from `render()`.
5. **`invalidate()` clears all caches**: including pre-baked themed strings (rebuild them).
6. **Keys via the UX layer**: use `getKeybindings().matches(data, "tui.*")` or `matchesKey`,
   never byte comparisons. See `ux.md`.
7. **`CURSOR_MARKER` only when focused**: emit it iff `this.focused`; the engine strips it.

## Step 7 — When to build a new widget vs. compose

Before building a new widget, check:
- Can it be a `Container` of `Text`/`TruncatedText`/`Spacer`? Then compose.
- Is it a selection UI? Use `SelectList` + `DynamicBorder` framing (see
  `packages/coding-agent/docs/tui.md` Pattern 1).
- Is it an async operation with cancel? Use `BorderedLoader` (Pattern 2 there).
- Is it a settings/toggle panel? Use `SettingsList` (Pattern 3).
- Is it a custom editor (vim mode, etc.)? Extend `CustomEditor` from
  `@earendil-works/pi-coding-agent` (Pattern 7) so app keybindings still work.

Only build a new widget when none of the built-ins compose to your case. The engine knows
nothing about widget types, so a new widget is just another `Component` implementation.

## Agreement with the other modules

- **Engine** (`rendering-engine.md`): calls `render(width)`, enforces the width invariant,
  consumes `CURSOR_MARKER`, calls `invalidate()` on theme/force-redraw.
- **UX** (`ux.md`): routes `handleInput(data)` to the focused widget; widgets read
  `KeybindingsManager` to interpret `data`; the `Loader` widget is the sanctioned exception
  that calls `requestRender()` from a timer.

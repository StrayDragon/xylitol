# Audit: pi-tui vs the Skill (Reverse Check)

This file is the reverse direction of the other references: instead of specifying what to
build, it checks where `packages/tui/src/` (the canonical implementation) **diverges from**
or **is under-specified by** the skill, so a port doesn't inherit the gaps or the
simplifications. Every finding is verified against the code with file:line references.

Findings are classified:
- **Skill wrong** — the skill says X, the code does Y. The skill must be fixed (done inline
  where noted; listed here for the record).
- **Skill gap** — the code does something the skill doesn't mention. The skill is
  incomplete but not wrong.
- **pi-tui issue** — the code has a real gap or risk worth flagging against pi-tui itself
  (not just the skill).

## Finding 1 — stdin splitting happens in the Terminal layer, not the engine (skill wrong)

**Skill said** (`ux.md` Step 1, before the fix): "The engine's `handleInput` loops over the
split sequences and routes each one." `rendering-engine.md` agreement section echoed this.

**Code does**: splitting lives in `StdinBuffer` (`packages/tui/src/stdin-buffer.ts`), which
is owned by `ProcessTerminal` (`terminal.ts` `setupStdinBuffer`, called from `start`).
`StdinBuffer` accumulates raw stdin chunks, detects complete sequences
(`isCompleteSequence` → CSI/OSC/DCS/APC/SS3/meta), and emits one `data` event per complete
sequence (or a `paste` event for bracketed paste, which `terminal.ts` re-wraps as
`\x1b[200~<content>\x1b[201~`). `forwardInputSequence` calls `this.inputHandler(sequence)`
— which is `TUI.handleInput` — **one sequence at a time**. The engine's `handleInput`
never loops over a split; it receives already-split sequences.

**Additional nuance the skill missed**: `StdinBuffer` splits *escape sequences*; a
printable run like `"hi"` is **not** split into `"h"` and `"i"` (it's not an escape
sequence). `Input.handleInput` handles this by inserting the whole printable run
(`insertCharacter(data)` iterates, but the batched printable case is handled by the widget
accepting multi-char `data` after rejecting control chars). So "three Up arrows arrive as
one blob" is solved by `StdinBuffer` splitting the three CSI sequences; "a typed word
arrives as one event" is solved by the widget inserting the whole run.

**Fix applied**: `ux.md` Step 1 now says splitting is a Terminal-layer responsibility
(`StdinBuffer`), with the printable-run nuance. `rendering-engine.md` agreement updated.

**Port implication**: put the splitter in your `Terminal` implementation (or use a lib that
yields one event at a time, e.g. crossterm's `event::read`). Don't loop-split in the
engine. See `terminal-foundations.md` "On a raw I/O lib".

## Finding 2 — the hard width error is only on the diff path (skill gap + pi-tui issue)

**Skill said** (`rendering-engine.md` Step 6, `ui-components.md` invariant 1, `SKILL.md`
shared invariant 1): "every `render(width)` line has display width ≤ `width`. The engine
hard-errors on overflow." Presented as a universal invariant.

**Code does**: the `visibleWidth(line) > width` check + crash log + `stop()` + `throw`
exists **only in the normal diff write loop** (`tui.ts` ~line 1540). `fullRender`
(`tui.ts` line 1284) writes `buffer += line` for each line with **no width check**.

**Consequence**: if a widget returns an over-wide line during a frame that takes the
`fullRender` path (width change, height change, `clearOnShrink`, change above viewport,
`requestRender(true)`), the line wraps in the terminal, desyncs every subsequent row, and
the diff produces garbage on the next frame — **silently**, without the crash that would
make it fixable. The crash backstop has a hole exactly on the frames where a full repaint
happens.

**Assessment**: low probability (widgets are expected to honor the contract via
`truncateToWidth`), but the backstop's value is precisely the cases where a widget fails
to. The hole is a real pi-tui issue worth filing: **`fullRender` should also width-check
each line and throw the same error.** The skill's claim is *aspirational* vs the code.

**Port implication**: when porting, put the width check in **both** `fullRender` and the
diff loop (or factor the per-line write into one checked helper). Don't inherit this hole.

## Finding 3 — `compositeOverlays` deliberately excludes `maxLinesRendered` (skill gap)

**Skill said** (`ux.md` Step 6, `rendering-engine.md` Step 5): the overlay padding
`workingHeight` is computed as `max(result.length, termHeight, max(row + len))`.

**Code does** (`tui.ts` line 1065):
```ts
// Excludes maxLinesRendered: the historical high-water mark caused self-reinforcing
// inflation that pushed content into scrollback on terminal widen
const workingHeight = Math.max(result.length, termHeight, minLinesNeeded);
```
The comment documents a prior bug: including `maxLinesRendered` in `workingHeight` made
overlay padding grow on each frame, which pushed content into scrollback when the terminal
widened. The fix was to exclude it. The skill describes the current state but not the
*invariant* (never pad overlays by the historical high-water mark) or why.

**Port implication**: when porting overlays, do **not** use `maxLinesRendered` for overlay
padding. Use `max(result.length, termHeight, max(row + len))`. Inherit the fix, not the bug.

## Finding 4 — `extractCursorPosition` scans bottom-up (skill gap, benign)

**Skill said** (`rendering-engine.md` Step 4): "scan only the bottom `height` lines, from
the bottom up." Didn't say why.

**Code does** (`tui.ts` `extractCursorPosition`): `for (let row = lines.length - 1; row >=
viewportTop; row--)`. Bottom-up is an optimization — the cursor is almost always near the
end (the focused editor/input is at the bottom of the content stack), so scanning from the
end finds the marker in O(1) typical case.

**Port implication**: scan bottom-up. Benign, but worth knowing.

## Finding 5 — `prevViewportTop` recomputation on height change (skill gap)

**Skill said** (`rendering-engine.md` Step 5): on height change, take the full-redraw path.
Didn't detail the viewport-top bookkeeping.

**Code does** (`tui.ts` `doRender` top):
```ts
const previousBufferLength = this.previousHeight > 0 ? this.previousViewportTop + this.previousHeight : height;
let prevViewportTop = heightChanged ? Math.max(0, previousBufferLength - height) : this.previousViewportTop;
```
This recomputes the viewport top when the height changes so the next diff's relative cursor
moves are computed against the post-resize viewport, not the stale one. The full redraw
itself doesn't use `prevViewportTop`, but the subsequent *normal* frame would, so the
recompute is defensive.

**Port implication**: replicate this recompute on the height-change branch. Minor but
correctness-relevant for the frame *after* a resize.

## Finding 6 — Kitty image pre-clear scroll guard (skill gap)

**Skill said** (`rendering-engine.md` "Kitty image handling"): reserve rows, expand the
changed range, clear reserved rows before writing the image. Didn't mention the scroll
fallback.

**Code does** (`tui.ts` normal path, ~line 1520):
```ts
const imageStartScreenRow = i - viewportTop;
if (imageStartScreenRow < 0 || imageStartScreenRow + imageReservedRows > height) {
  logRedraw(`kitty image pre-clear would scroll ...`);
  fullRender(true);
  return;
}
```
If clearing the reserved rows for an image would scroll the terminal (the image starts
above the viewport or extends past the bottom), fall back to `fullRender(true)`. The skill
omitted this guard.

**Port implication**: include the guard if you support images. If you skip images, ignore.

## Finding 7 — `applyLineResets` also normalizes (skill gap, minor)

**Skill said** (`rendering-engine.md` Step 4): "append `\x1b[0m\x1b]8;;\x07` per non-image
line. Also normalize the line (strip stray control bytes)."

**Code does** (`tui.ts` `applyLineResets`): `lines[i] = normalizeTerminalOutput(line) +
reset` where `normalizeTerminalOutput` (`utils.ts`) does more than strip control bytes —
it handles OSC 8 hyperlink state and other normalization. The skill's parenthetical
understates it.

**Port implication**: implement a `normalizeTerminalOutput` equivalent, not just control-byte
stripping. Mirror `utils.ts`. Minor.

## Finding 8 — `setFocusInternal` has a `clear`/`preserve` policy (skill simplification, OK)

**Skill said** (`ux.md` Step 4): "`setFocus(component)`" with the simple three-step
toggle, then "Overlay focus restore complicates this — see Step 6."

**Code does** (`tui.ts` `setFocusInternal`): takes an `overlayFocusRestore: "clear" |
"preserve"` policy and has dense branching for the blocked/eligible restore states
(`getVisibleOverlayFocusRestore`, `isOverlayFocusAncestor`, `resolveBlockedOverlayFocusResume`).
`setFocus(component)` calls `setFocusInternal({ component, overlayFocusRestore: "clear" })`;
overlay handles call it with `"preserve"`.

**Assessment**: the skill's simplification is fair — the base rule is one focused component,
and the overlay restore is specified in Step 6. The policy parameter is an implementation
detail of how the base rule and the overlay rule cooperate. A port that implements the
state machine will need the policy; a minimal port can ignore it. OK as-is.

## Finding 9 — `CURSOR_MARKER` is APC-prefixed but BEL-terminated (nuance, OK)

**Skill said**: "a zero-width APC sequence."

**Code**: `CURSOR_MARKER = "\x1b_pi:c\x07"`. APC is `\x1b_`; its spec terminator is ST
(`\x1b\\`), but BEL (`\x07`) is widely accepted and shorter. Terminals ignore the whole
sequence either way (it's an unknown APC payload `pi:c`).

**Port implication**: use `\x1b_pi:c\x07` verbatim (or any zero-width sequence terminals
ignore). The marker must be zero display width and not rendered. OK.

## Finding 10 — `drainInput` disables Kitty protocol first (skill gap, minor)

**Skill said** (`rendering-engine.md` Step 10, `ux.md` Step 9): "drain stdin over slow SSH."

**Code does** (`terminal.ts` `drainInput`): **disables Kitty keyboard first**
(`\x1b[<u`) so late key-release events don't generate *new* Kitty escapes during the drain,
then drains. The skill mentioned drain but not the protocol-disable ordering.

**Port implication**: disable Kitty keyboard before draining, then restore raw mode. Minor
but correctness-relevant over slow SSH.

## Summary table

| # | Finding | Type | Severity |
|---|---|---|---|
| 1 | stdin splitting in Terminal layer, not engine | skill wrong (fixed) | medium — a port that loop-splits in the engine mis-locates responsibility |
| 2 | hard width error only on diff path, not fullRender | skill gap + pi-tui issue | medium — silent corruption on full-redraw frames if a widget overflows |
| 3 | compositeOverlays excludes maxLinesRendered (prior bug fix) | skill gap | medium — a port that includes it reintroduces scrollback inflation on widen |
| 4 | extractCursorPosition scans bottom-up (optimization) | skill gap | low |
| 5 | prevViewportTop recompute on height change | skill gap | low — affects the frame after a resize |
| 6 | Kitty image pre-clear scroll guard | skill gap | low — only if porting images |
| 7 | applyLineResets also normalizes (more than control-byte strip) | skill gap | low |
| 8 | setFocusInternal clear/preserve policy | simplification (OK) | none |
| 9 | CURSOR_MARKER BEL-terminated APC | nuance | none |
| 10 | drainInput disables Kitty first | skill gap | low — slow SSH correctness |

## What the skill got right (confirmed against code)

- Three strategies (first render / full redraw / normal diff) and their trigger conditions
  — matches `doRender` branches.
- `cursorRow` vs `hardwareCursorRow` tracked separately on the diff path — matches
  `tui.ts` (`this.cursorRow = max(0, newLines.length-1); this.hardwareCursorRow =
  finalCursorRow`).
- `MIN_RENDER_INTERVAL_MS = 16` coalesced scheduling — matches.
- `CURSOR_MARKER` extraction + `positionHardwareCursor` — matches.
- `KeybindingsManager` + `TUI_KEYBINDINGS` + conflict detection + declaration-merge
  extension — matches `keybindings.ts`.
- `matchesKey` over legacy + Kitty + modifyOtherKeys — matches `keys.ts`.
- Overlay stack z-order via `focusOrder`, `preFocus` retargeting on hide — matches.
- Focus-restore state machine (inactive/eligible/blocked + resume) — matches (skill is a
  fair simplification of dense code).
- `clearOnShrink` + the no-overlays condition — matches.
- Termux height-change exception — matches.
- Hard width error on the diff path (crash log + stop + throw) — matches (but see Finding 2).
- Kitty image reserved-rows + changed-range expansion + delete-on-redraw — matches.

## Recommendation for pi-tui itself

The one finding worth filing as a pi-tui issue is **Finding 2**: `fullRender` should
width-check each line and throw the same hard error as the diff path, so the backstop
covers full-redraw frames too. The fix is small (a checked per-line write helper used in
both paths) and closes a silent-corruption hole. The other findings are documentation gaps
in the skill (now noted here) rather than code defects.

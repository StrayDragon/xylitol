# Handoff — c2070 dual interaction modes (Mode B)

**Branch:** `sdd/c2070-add-package-tui-dual-interaction-modes`
**Worktree:** this tree (`tspt`) — always `eval "$(just cargo-wt-env)"` before cargo.
**Demo:** `just demo-tui-alt-screen` (`XYLITOL_AGENT_DEMO_MODE=b`)

## Product intent (locked)

- Mode B = alt-screen + app selection/copy (not foundation-only).
- Copy notice first (ptim15 / ath31); Editor multi-line selection (ptim13).
- Executable BDD: `package-tui-interaction-modes.feature` + steps under `tests/bdd/`.

## Human verify snapshot

| Item | Status |
|---|---|
| ptim15 copy notice | PASS |
| ath31 product notice (not Error toast) | PASS |
| Copy char count (fit-pad trim) | PASS |
| Narrow `↑ N more` | PASS |
| Ctrl+G expands `[paste #N]` | fixed + unit test — re-check in demo |
| ptim13 edge scroll feel | hardened + automated — re-check feel |
| Short paste typewriter | bracketed re-arm + burst paint suppress — re-check |
| Shift+Enter (Ctrl+J OK) | alt-screen keyboard re-arm + Char(`\\n`) — re-check |
| H6 overall | partial |

## Fixes landed in this sync

### 1. Ctrl+G / external editor paste expand

- **Bug:** `$EDITOR` tempfile could contain `[paste #1 +26 lines]` instead of body.
- **Demo path:** already `get_expanded_text()` before suspend (`agent_demo` after-dispatch).
- **Product stub:** was `get_text()` → now `get_expanded_text()` (`layout/root/mod.rs`).
- **Product real path:** already `root.editor_text()` → expanded.
- **Test:** `external_editor_path_expands_paste_markers` in editor unit tests.
- **Pi parity:** `getExpandedText` before external editor.

### 2. ptim13 edge scroll (no human-only gate)

- Step **6** lines; edge zone **2** rows (border + near content).
- Tick while dragging continues scroll (hold at edge).
- Tests: `editor_selection_edge_scroll_reveals_above`, `editor_selection_edge_scroll_steps_by_zone_and_tick`.

### 3. Short non-bracketed paste (typewriter)

- **Do not** retract/buffer chars into a hidden coalesce string — that broke harness
  typing (`get_text` empty mid-burst).
- Instead: after `PASTE_BURST_MIN_CHARS` (8) consecutive fast chars, **suppress
  mid-burst `input_wants_rerender`**; model still updates; idle `tick` paints once.
- Bracketed paste remains the preferred path; Mode B `enter_alternate_screen`
  re-enables it (main fix for short pastes looking like typewriter).
- Test: `paste_burst_suppresses_mid_burst_rerender`.
- **Pi note (research):** checked pi HEAD has atomic bracketed paste only — **no**
  `PasteBurst` / unbracketed coalesce. xylitol paint-suppress is an extension.

### 4. Shift+Enter on Mode B alt-screen (pi research)

Source: [pi Shift+Enter research](f9f7928d-2aad-40db-9b14-bec7ad171c5e) against pi
`47b021a6…`.

**Pi bindings / matchers**

- Newline: `shift+enter` + `ctrl+j`; submit: `enter` (`keybindings.ts`).
- Matchers: Kitty CSI-u, modifyOtherKeys `CSI 27;2;13~`, kitty-active Ghostty `\n`,
  Kitty map `\x1b\r`; backslash+Enter editor fallback.
- Mouse capture does **not** change keyboard decode; active transcript search
  can steal Enter / Shift+Enter (pi alt-screen only).

**Lifecycle (important delta vs xylitol)**

- Pi: enter alt-buffer **then** `ProcessTerminal.start()` → negotiate Kitty /
  modifyOtherKeys while already on alt screen.
- xylitol: `start()` negotiates first, Mode B later calls `enter_alternate_screen`.
- **Parity path we use:** `rearm_keyboard_after_alt_screen` after alt enter
  (also covers Ctrl+G suspend → `start()` → re-enter alt). Do **not** remap bare
  `KeyCode::Enter` to newline (Shift state is gone).

**xylitol today**

- `matches_key_event`: `KeyCode::Char('\n')` → `shift+enter`; `\n`/`\r` not printable.
- `enter_alternate_screen`: bracketed paste + Kitty re-push + modifyOtherKeys arm.
- Tests: `ghostty_shift_enter_char_lf_inserts_newline`,
  `matches_key_event_ghostty_lf_is_shift_enter`, modifyOtherKeys VT tests.

**Open debt (not blocking this sync)**

- Optimistic `set_kitty_protocol_active(true)` without consuming positive
  `CSI ?Nu` (crossterm owns stdin). True pi-style “only trust positive detection
  else modifyOtherKeys-only” needs a response drain path — track separately.
- Crossterm may not surface raw `CSI 27;…~` the way pi's byte matcher does;
  prefer Enter+SHIFT / Char(`\n`) KeyEvent tests over assuming VT strings reach
  `matches_key`.

## Key paths

| Area | Path |
|---|---|
| Mode B runtime | `packages/xylitol-tui/src/mode_b.rs` |
| Selection / OSC52 | `selection.rs`, `scroll_view.rs`, `tui.rs` |
| Editor selection + coalesce | `components/editor.rs` |
| Keys / Shift+Enter | `keys.rs`, `keybindings.rs` |
| Alt-screen / keyboard | `terminal.rs` |
| Demo | `examples/agent_demo.rs` |
| Product host / root | `src/app/tui/host/`, `layout/root/` |
| Specs / tasks | `llmanspec/changes/c2070-…/`, `llmanspec/specs/package-tui-interaction-modes/` |

## Verify commands

```bash
eval "$(just cargo-wt-env)"
cargo test -p xylitol-tui --lib
cargo test -p xylitol-tui --test keys_test
cargo test -p xylitol --test bdd package_tui_interaction
# optional: just test-tui / just demo-tui-alt-screen
```

## Open / next on other machine

1. Human re-check Shift+Enter + edge scroll + short paste feel in Mode B demo.
2. If Shift+Enter still fails on a specific emulator: capture raw `KeyEvent`
   (code/modifiers/kind); keep Ctrl+J / `\`+Enter; do not remap bare Enter.
3. Optional later: response-driven Kitty vs modifyOtherKeys (drop optimistic
   `set_kitty_protocol_active(true)`).
4. `llman sdd validate --strict` may still ERROR on unchecked tasks (5.4, 7.3–7.4).
5. Keep updating this `_HANDOFF.md` when shipping handoff-worthy chunks; commit + push.

## Do not

- Share `CARGO_TARGET_DIR` across worktrees.
- Treat human-only H6 as done without the automated edge-scroll tests staying green.

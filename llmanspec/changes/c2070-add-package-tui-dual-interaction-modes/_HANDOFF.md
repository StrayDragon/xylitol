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
| Ctrl+G expands `[paste #N]` | **Fix in this handoff** — see below |
| ptim13 edge scroll feel | **Hardened + automated** — re-check feel |
| Short paste typewriter | **Visual coalesce @ 2 chars** — re-check |
| Shift+Enter (Ctrl+J OK) | **Alt-screen keyboard re-arm** — re-check |
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

- Visual coalesce starts at **2** consecutive fast chars: retract painted prefix, buffer until idle / next key / `InputEvent::Paste`.
- Flush goes through `paste()` (markers if >10 lines / >1000 chars).
- Enter-suppress still at `PASTE_BURST_MIN_CHARS` (8).
- Test: `paste_visual_coalesce_batches_fast_chars`.
- Bracketed paste remains the preferred path; Mode B `enter_alternate_screen` re-enables it.

### 4. Shift+Enter on Mode B alt-screen (pi research)

**Pi (`packages/tui/src/keys.ts` + `terminal.ts`):**

- Kitty CSI-u / modifyOtherKeys `CSI 27;2;13~` = shift+enter.
- When kitty active: Ghostty `\n` and Kitty map `\x1b\r` = shift+enter.
- When kitty inactive: `\n` = plain enter.
- Apple Terminal / Win32: native Shift poll on `\r` (not Linux; N/A here).

**xylitol:**

- `matches_key_event`: `KeyCode::Char('\n')` → `shift+enter`; `\n`/`\r` not printable.
- `enter_alternate_screen`: re-enable bracketed paste + **`rearm_keyboard_after_alt_screen`** (re-push Kitty flags **and** arm modifyOtherKeys — emulators often clear both on alt buffer).
- Tests: `ghostty_shift_enter_char_lf_inserts_newline`, `matches_key_event_ghostty_lf_is_shift_enter`, existing modifyOtherKeys VT tests.

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
2. If Shift+Enter still fails on a specific emulator: capture raw `KeyEvent` (code/modifiers/kind) and extend matcher; Linux has no pi native-modifier helper.
3. `llman sdd validate --strict` may still ERROR on unchecked tasks (5.4, 7.3–7.4) until tasks.md updated.
4. Keep updating this `_HANDOFF.md` when shipping handoff-worthy chunks; commit + push the SDD branch so other worktrees can pull.

## Do not

- Share `CARGO_TARGET_DIR` across worktrees.
- Treat human-only H6 as done without the automated edge-scroll tests staying green.

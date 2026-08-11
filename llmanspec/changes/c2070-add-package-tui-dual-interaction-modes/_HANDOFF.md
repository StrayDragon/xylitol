# Handoff — c2070 dual interaction modes (Mode B)

**Branch:** `sdd/c2070-add-package-tui-dual-interaction-modes`
**Remote:** `origin/sdd/c2070-add-package-tui-dual-interaction-modes`
**Worktree:** this tree (`tspt`) — always `eval "$(just cargo-wt-env)"` before cargo.
**Demo:** `just demo-tui-alt-screen` (`XYLITOL_AGENT_DEMO_MODE=b`)
**Last sync:** 2026-08-11 EOD — human **未验收** latest mouse/dock/lag batch (`fe73efe3`).

## EOD status (read this first)

Human left for the day **without** re-verifying the latest fix batch. Treat items marked
「待人验」as **code+unit tests only**, not PASS.

### Human message before leaving (verbatim intent)

> ok 先这样请记录最近的更改和最后一条消息的内容 我也没有验收 然后更新_HANDOFF
> 并且commit 然后push这个分支到远程 我要下班了

### Prior human verify message (the one that drove `fe73efe3`)

| Topic | Human report |
|---|---|
| Ctrl+G expand `[paste #N]` | **ok pass** |
| 短粘贴打字机 | **好了 pass** |
| Shift+Enter | **好了** but **渲染 bug** — content duplicated around editor `↑ N more` (transcript line + same line again in editor viewport; screenshot) |
| ptim13 边沿滚动 | **仍诡异**：点输入框里 `primary` 的 `p` 会视觉上选中之前内容但 **不触发 copy**；仍不能自然拖到输入框边缘自动滚动 |
| 通用 | 鼠标左键拖选 **滞后不跟手**；拖选时 **spinner 变慢** |

## Human verify snapshot

| Item | Status |
|---|---|
| ptim15 copy notice | PASS (earlier) |
| ath31 product notice (not Error toast) | PASS (earlier) |
| Copy char count (fit-pad trim) | PASS (earlier) |
| Narrow `↑ N more` | PASS (earlier) |
| Ctrl+G expands `[paste #N]` | PASS (human) |
| Short paste typewriter | PASS (human) |
| Shift+Enter keys (Ctrl+J / Shift+Enter) | PASS keys (human); **dock-seam ghost 待人验** after `fe73efe3` |
| ptim13 editor edge scroll / click ghost | **待人验** after `fe73efe3` |
| Drag lag / spinner slow | **待人验** after `fe73efe3` |
| H6 overall | partial |

## Recent commits on this branch (newest first)

| Commit | Summary |
|---|---|
| `fe73efe3` | Mode B editor drag routing, dock sync, mouse paint coalesce |
| `57861d83` | Fold pi Shift+Enter research into handoff |
| `b32df605` | Suppress paste-burst paints without retracting buffer |
| `9db46e9a` | Paste expand, edge scroll, coalesce, Shift+Enter re-arm |
| (+ earlier c2070 Mode B foundation / copy-notice / demo) | see `git log` |

## What `fe73efe3` changed (not human-verified)

Root causes from explore:

1. Demo dropped Drag/Up outside cached editor rect → stale highlight, no copy, dead edge-scroll.
2. Mode B dropped all bare `Moved` → some terminals never extend editor drag.
3. `input_wants_rerender` queried **before** `handle_input` → first Down painted late.
4. Stale `dock_rows` on Shift+Enter growth → transcript/dock seam duplicate look.
5. Mouse flood starved `idle_tick` → spinner lag; immediate `do_render` every drag.
6. `PasteBurst::is_coalescing` stuck on consecutive count forever → paint suppress sticky.

Fixes landed:

- Demo `handle_editor_mouse`: while dragging, clamp & forward (don't drop).
- `Component::wants_pointer_motion` + Mode B Moved fallthrough when dragging.
- Query mouse wants **after** handle; Editor / SharedUiRoot / demo implement hints.
- `mode_b_dock_rows_hint` applied in `do_render` before `project_frame`; dock change forces full clear.
- Mouse path: `request_render` + `idle_tick` + `try_render` (16ms coalesce).
- Time-bound `is_coalescing`.
- Tests: `editor_mouse_down_wants_rerender_after_handle`, `is_coalescing_expires_after_idle…`.

## Earlier batches (already human-PASS where noted)

### Ctrl+G / external editor paste expand — PASS

- Demo: `get_expanded_text()` before suspend.
- Product stub: `get_expanded_text()`; real path: `editor_text()` → expanded.
- Test: `external_editor_path_expands_paste_markers`.

### Short paste typewriter — PASS

- No retract/buffer coalesce (broke harness).
- Burst≥8: suppress mid-burst `input_wants_rerender`; idle tick catch-up.
- Bracketed paste re-armed on alt-screen enter.

### Shift+Enter protocol — PASS keys (ghost separate)

- Pi research: [pi Shift+Enter research](f9f7928d-2aad-40db-9b14-bec7ad171c5e).
- `rearm_keyboard_after_alt_screen` after alt enter; `Char('\n')` → shift+enter.
- Do **not** remap bare `Enter` to newline.
- Open debt: optimistic `set_kitty_protocol_active(true)` without CSI response drain.

### ptim13 edge scroll (unit)

- Step 6 / edge zone 2; tick while dragging.
- Tests exist; **feel still 待人验** after routing fix.

## Key paths

| Area | Path |
|---|---|
| Mode B runtime | `packages/xylitol-tui/src/mode_b.rs` |
| Selection / OSC52 | `selection.rs`, `scroll_view.rs`, `tui.rs` |
| Editor selection / mouse | `components/editor.rs`, `examples/agent_demo.rs` |
| Keys / Shift+Enter | `keys.rs`, `keybindings.rs`, `terminal.rs` |
| Product host / root | `src/app/tui/host/`, `layout/root/` (SharedUiRoot dock hint) |
| Specs / tasks | `llmanspec/changes/c2070-…/`, `llmanspec/specs/package-tui-interaction-modes/` |

## Next machine checklist

```bash
git fetch origin
git checkout sdd/c2070-add-package-tui-dual-interaction-modes
git pull
eval "$(just cargo-wt-env)"
just demo-tui-alt-screen
```

**Must human-verify (priority):**

1. Editor: click in multi-line buffer — no stale “selected previous content” ghost; empty click no copy.
2. Editor: drag selection to ↑/↓ more edges — auto-scroll feels natural.
3. Shift+Enter growth — no duplicate line above/below `↑ N more`.
4. Drag select — follows pointer; spinner keeps ticking under drag.

**Then optional:**

5. `llman sdd validate --strict` / unchecked tasks (5.4, 7.3–7.4).
6. Response-driven Kitty vs modifyOtherKeys (debt).

## Do not

- Share `CARGO_TARGET_DIR` across worktrees.
- Mark ptim13 / Shift+Enter ghost / drag-lag as PASS until human re-checks after `fe73efe3`.

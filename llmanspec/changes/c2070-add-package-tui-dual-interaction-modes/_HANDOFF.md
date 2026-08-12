# Handoff — c2070 dual interaction modes (Mode B)

**Branch:** `sdd/c2070-add-package-tui-dual-interaction-modes`
**Remote:** `origin/sdd/c2070-add-package-tui-dual-interaction-modes`
**Worktree:** this tree — always `eval "$(just cargo-wt-env)"` before cargo.
**Demo:** `just demo-tui-alt-screen` (`XYLITOL_AGENT_DEMO_MODE=b`)
**Last sync:** 2026-08-12 — click-ghost PASS; edge-scroll cut; Ghostty drag/spinner OK.

## Human verify snapshot (2026-08-12)

| Item | Status |
|---|---|
| Ctrl+G / short paste / keys | PASS (earlier) |
| Shift+Enter dock seam 重复行 | **PASS** |
| Editor 点击鬼影 / 空点不 copy | **PASS** (after edge-scroll removal) |
| Editor 边沿自动滚 | **WONTFIX / 已裁** |
| Drag lag / spinner (Ghostty) | **PASS** — human: 没什么感觉了 |
| H6 Editor 多行选区 | **PASS** |
| H7 Copied 短提示 (demo) | OK for demo; **产品落点延后讨论** |

### Human notes (product defer)

- **`Copied` 通知位置**：当前 demo 无所谓；`src/app/tui` Mode B 集成时再讨论落点（壳层槽 / dock / 其它）。
- **误触检测**（单击 vs 微拖、是否 copy）：同样放到产品集成时再议；本 change 库侧保持「有跨格才选区+copy，纯单击清选」。

### Earlier FAIL → fix (kept for archaeology)

Click on first visible rows with `↑ N more` ran `selection_edge_scroll` on Moved/tick → remapped focus to earlier buffer → ghost「选中之前全部」。Removed Editor edge-scroll entirely.

## Next (SDD close-out, not more hand-feel)

```bash
eval "$(just cargo-wt-env)"
# 7.3 Mode B 下游接入清单（ptim14）
# 7.4 llman sdd validate --strict + verify
```

**Do not** re-litigate demo Copied chrome or editor edge-scroll without a new human decision.
**Do not** share `CARGO_TARGET_DIR` across worktrees.

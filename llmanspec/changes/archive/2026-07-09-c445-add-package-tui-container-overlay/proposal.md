---
change_id: c445-add-package-tui-container-overlay
title: "add Container + OverlayHandle; trim unused image surface for app shell"
status: proposed
priority: 445
depends_on: []
author: agent
---

# c445-add-package-tui-container-overlay

## Why

产品面 `src/app/tui/` 将基于 `xylitol-tui` 从零接线。pi interactive 的组件树依赖：

1. **`Container`** — 垂直栈（header / chat / status / editor / footer），应用面需要嵌套而非把整棵树压成单个 `Component`。
2. **`OverlayHandle`** — `show_overlay` 目前返回 `()`，host 无法 hide / setHidden / focus / unfocus 已打开的 overlay（命令面板、设置、确认框）。

同时 `REPORT.tmp.md` / `_HANDOFF` 建议在接线前裁掉应用层不用的图片表面，减小包面与维护面。

本变更只动 `packages/xylitol-tui`（`package-tui-*` specs），不实现产品 App Shell（留给后续 `c450`）。

## What Changes

1. **`Container`**：对齐 pi `tui.ts` `Container` — `add_child` / `remove_child` / `clear` / `render` 垂直拼接 / `invalidate` 下传 / `handle_input` 默认 no-op（焦点由 TUI 路由，不自动扇出）。
2. **`OverlayHandle`**：`show_overlay` 返回可控制句柄（`hide` / `set_hidden` / `is_hidden` / `focus` / `unfocus` / `is_focused`）。最小可用版：单层/栈顶控制即可；**完整 overlay focus-restore 状态机**（pi eligible/blocked）本变更不做，记入 future。
3. **按需 `InputListener`**：若 overlay/调试键需要在焦点组件前拦截输入，补 `add_input_listener` 薄管道；否则跳过。
4. **裁剪**：`components/image` 与 `terminal_image` 中应用层未用的编码路径按「保留 `is_image_line` / 能力探测核心，其余 `#[cfg]` 或删除」处理；以 `cargo test -p xylitol-tui` 与 `agent_demo` 不依赖图片为准。
5. **验证**：扩 `test-tui-harness`（Container 嵌套渲染、OverlayHandle hide 后不再合成）。

## Capabilities

- `package-tui-engine`（新建：引擎级 Container / OverlayHandle / 输入拦截边界）

## Impact

- `packages/xylitol-tui/src/tui.rs`、`lib.rs`、测试 harness
- 可能触及 `terminal_image.rs` / `components/image.rs` 裁剪
- **不改** `src/app/tui/`（仍占位）
- Spec：`package-tui-engine`；产品面 `app-tui` 不动

## 不在范围

- 产品 App Shell / Driver 合流（`c450`）
- Overlay focus-restore 完整状态机
- DESIGN.md 强制 enforce（本变更可引用，不实现主题系统）
- KeyEvent 路径（已在硬切中完成）

## 验证

```bash
cargo test -p xylitol-tui
cargo clippy -p xylitol-tui --all-targets -- -D warnings
llman sdd validate c445-add-package-tui-container-overlay --strict --no-interactive
```

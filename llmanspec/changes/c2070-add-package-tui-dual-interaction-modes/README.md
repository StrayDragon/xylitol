# c2070 — 双交互架构（完整大需求）

> 顶层基础 change。级联后续已拆为 **独立** `llmanspec/changes/<id>/`，经 frontmatter `depends_on` / 本 change `blocks` 约束。

| 路径 | 内容 |
|---|---|
| [`proposal.md`](./proposal.md) | Mode A inline ↔ Mode B alt-screen；Mode B 选区 MUST |
| [`design.md`](./design.md) | 架构权衡与子系统切分 |
| [`tasks.md`](./tasks.md) | FF / apply 任务切片 |
| [`research/`](./research/) | 架构与一手对照（Pi / Zellij / xylitol） |

## 依赖本 change 的后续（原 cascade）

| id | depends_on（摘要） |
|---|---|
| [`c1760-add-tui-activity-fold`](../c1760-add-tui-activity-fold/) | `c1755`, **`c2070`** |
| [`c2040-add-tui-mouse-click-fold-triangle`](../c2040-add-tui-mouse-click-fold-triangle/) | 已归档 **`c2020`** + **`c2070`** |
| [`c2050-update-activity-fold-mouse-leader`](../c2050-update-activity-fold-mouse-leader/) | `c1760`, `c2020`, `c2040`, **`c2070`** |
| [`c1505-add-tui-scrollback-viewport-slice`](../c1505-add-tui-scrollback-viewport-slice/) | **`c2070`** |
| [`c1535-optimize-tui-stream-wrap-tail`](../c1535-optimize-tui-stream-wrap-tail/) | **`c2070`** |

产品近期：继续打磨 **inline**；本 change Specs landing 后实现引擎，再考虑 app 切 Mode B。

## 已落地地基：c2020（保留，勿当「产品已支持鼠标」）

归档：`llmanspec/changes/archive/2026-08-11-c2020-add-package-tui-mouse-input/`。

| 仍保留 | 产品现状 |
|---|---|
| 包 API：`enable_mouse_capture` / `disable`、`InputEvent::Mouse`、Moved 默认不刷帧 | **保留** — Mode B 实现时复用 |
| `XYLITOL_TUI_MOUSE` → `env_requests_mouse_capture()` | **仅 lab / e2e**；**不是**正式 inline 鼠标 UX |
| 产品 `TerminalGuard` | **不读**该 env；inline 会话默认不开 capture |

Mode B 路径：在 alt-screen 上显式 Enable，并交付选区 MUST；勿再把「设个 env」当成产品鼠标开关。

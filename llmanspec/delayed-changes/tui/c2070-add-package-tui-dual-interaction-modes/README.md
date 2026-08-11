# c2070 延后族（双交互架构）

> 顶层基础 change + 级联后续。整包在 `delayed-changes/tui/` 下，便于升格时 **整目录移动**。

| 路径 | 内容 |
|---|---|
| [`proposal.md`](./proposal.md) | **c2070** — Mode A inline ↔ Mode B alt-screen；Mode B 选区 MUST |
| [`research/`](./research/) | 架构调研（oneof / Pi / starline / 代价） |
| [`cascade/`](./cascade/) | 依赖本 change 的后续草案 |

## cascade 一览

| id | depends_on（摘要） |
|---|---|
| `c1760-add-tui-activity-fold` | `c1755`, **`c2070`** |
| `c2040-add-tui-mouse-click-fold-triangle` | 已归档 **`c2020`**（鼠标管道）+ **`c2070`** Mode B；见下方 c2020 节 |
| `c2050-update-activity-fold-mouse-leader` | `c1760`, `c2020`, `c2040`, **`c2070`** |
| `c1505-add-tui-scrollback-viewport-slice` | **`c2070`** |
| `c1535-optimize-tui-stream-wrap-tail` | **`c2070`** |

产品近期：继续打磨 **inline**；本 change 保持 delay，实现后再考虑 app 切 Mode B。

## 已落地地基：c2020（保留，勿当「产品已支持鼠标」）

归档：`llmanspec/changes/archive/2026-08-11-c2020-add-package-tui-mouse-input/`。

| 仍保留 | 产品现状（2026-08-11） |
|---|---|
| 包 API：`enable_mouse_capture` / `disable`、`InputEvent::Mouse`、Moved 默认不刷帧 | **保留** — Mode B 实现时复用 |
| `XYLITOL_TUI_MOUSE` → `env_requests_mouse_capture()` | **仅 lab / e2e**（`agent_demo`、PTY ath29）；**不是**正式 inline 鼠标 UX |
| 产品 `TerminalGuard` | **不读**该 env；inline 会话默认不开 capture |

升格本 change / Mode B 时：在 alt-screen 路径上显式 Enable，并交付选区 MUST；勿再把「设个 env」当成产品鼠标开关。

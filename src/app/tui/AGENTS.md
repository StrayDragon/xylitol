# src/app/tui/

本面专属边界。分层与 seam SSOT：`src/AGENTS.md`。引擎库：`packages/xylitol-tui/AGENTS.md`。

## 现状

旧 in-tree engine/widgets **已移除**。本目录为占位；基于 `xylitol-tui` **从零重做**（不以旧 UI/UX 为参考）。跨面 seam（`Driver` / `dispatch` / `composition` / `XyEvent`）保留。

`tui` feature 下 `run()` 当前返回明确错误，直到重做落地。

## 硬约束

- 渲染/通用组件只用 `xylitol_tui`；禁止在本目录再实现差分引擎或通用 Editor/Markdown。
- 产品路径 **host 驱动**同步引擎；异步事件合流在本面；勿调 `TUI::start()`（demo 专用）。
- 驱动 agent 只经 `app/core/driver::Driver`；禁止 reach `agent::session` / `runtime` / `infra`。
- slash 语义复用 `protocol::Command`，经 `app/core/dispatch`。
- 组件不直接调 `Driver`、不读写 session；颜色走本面 theme 语义 token。
- 需要新 agent 行为 → 先扩 `runtime_protocol/` / `agent/`，本面只消费。

## HOW（指针）

| 任务 | 去哪 |
|---|---|
| 写/改本面 | `write-tui` skill |
| 新增/改造应用面方法论 | `write-surface` skill |
| 包内组件与五层验证 | `test-tui-harness` skill |
| 排查（禁 println） | `XYLITOL_DEBUG=1` + `~/.xylitol/logs/xylitol.log`（细节见 `write-tui`） |

文件布局以重做落地为准；落地后只更新本面地图，不把 how-to 堆进本文件。

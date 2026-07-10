# src/app/tui/

本面专属边界。分层与 seam：`src/AGENTS.md`。引擎库：`packages/xylitol-tui/AGENTS.md`。AGENTS 写法：根 `AGENTS.md`。

## 现状

旧 in-tree engine/widgets **已移除**。本目录为占位；基于 `xylitol-tui` **从零重做**（不以旧 UI/UX 为参考）。跨面 seam（`Driver` / `dispatch` / `composition` / `XyEvent`）保留。

`tui` feature 下 `run()` 当前返回明确错误，直到重做落地。

## 视觉 / UX

产品终端视觉 SSOT：本目录 **`DESIGN.md`**（语义 token + layout + 复制友好规则；拆分见 `design/*`，c449）。包内组件只收闭包主题，不承载产品 layout。

## Specs

产品面 capability：`app-tui-*`（`app-tui-host` / `bridge` / `transcript` / `chrome` / `input` / `commands`）。跨切面索引：`app-tui`。合约已归档：`archive/2026-07-10-c450-revise-app-tui-contract`。

steer / follow-up 键位依赖 **c461**（Agent+Driver 队列 seam）；本面只调 `Driver`，不持有 ReAct 队列。

## Debug 日志

**目标**：debug 构建默认写即时日志（可 `tail -f`）；release 默认关。
**现状**：仍以 `XYLITOL_DEBUG` / `RUST_LOG` 为准；`cfg(debug_assertions)` 兜底待 **c460** 落地（见 `_tmp_prompts/03b-logging-findings.md`）。
路径见 `app/cli/logging.rs`；`target: "xylitol::tui"`。禁止 `println!`。

## 硬约束

- 渲染/通用组件只用 `xylitol_tui`；禁止在本目录再实现差分引擎或通用 Editor/Markdown。
- **需要底层 TUI 能力时**（新组件、键协议、overlay、布局容器、渲染/输入管线等）：先到 `packages/xylitol-tui` 查是否已有或可扩展；缺能力在包内补，再由本面接线。不要在本目录复制「准通用」实现。
- 产品路径 **host 驱动**同步引擎；异步事件合流在本面；勿调 `TUI::start()`（demo 专用）。
- 驱动 agent 只经 `app/core/driver::Driver`（含日后 `steer` / `follow_up` / `clear_queue`）；禁止 reach `agent::session` / `runtime` / `infra`。
- slash 语义复用 `protocol::Command`，经 `app/core/dispatch`（含 Steer/FollowUp 变体，见 c461）。
- 组件不直接调 `Driver`、不读写 session；颜色走本面 theme 语义 token（对齐 `DESIGN.md`）。
- 需要新 agent 行为 → 先扩 `runtime_protocol/` / `agent/`，本面只消费。

## HOW（指针）

| 任务 | 去哪 |
|---|---|
| 写/改本面 | `write-tui` skill |
| UX / token / layout | 本目录 `DESIGN.md` |
| 底层能力是否已有 / 如何扩展 | `packages/xylitol-tui/AGENTS.md`（先查包，再改本面） |
| 新增/改造应用面 | `write-surface` skill |
| 包内组件与五层验证 | `test-tui-harness` skill |
| 排查（禁 println） | `XYLITOL_DEBUG=1` + `~/.xylitol/logs/xylitol.log`（见 `write-tui`） |

文件布局以重做落地为准；落地后只更新本面地图，不把 how-to 堆进本文件。

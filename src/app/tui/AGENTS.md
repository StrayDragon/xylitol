# src/app/tui/

本面专属边界。分层与 seam：`src/AGENTS.md`。引擎库：`packages/xylitol-tui/AGENTS.md`。AGENTS 写法：根 `AGENTS.md`。

## 现状

基于 `xylitol-tui` 的 **host 驱动空壳**（c460）：`HostSession` + `Shell` + 终端 lifecycle。`run()` 可进入；XyEvent / slash / steer 接线在后续 change。

## 视觉 / UX

产品终端视觉 SSOT：本目录 **`DESIGN.md`**（语义 token + layout + 复制友好规则；拆分见 `design/*`，c449）。包内组件只收闭包主题，不承载产品 layout。

## Specs

产品面 capability：`app-tui-*`（`app-tui-host` / `bridge` / `transcript` / `chrome` / `input` / `commands`）。跨切面索引：`app-tui`。合约已归档：`archive/2026-07-10-c450-revise-app-tui-contract`。

steer / follow-up 键位依赖 **c461**（Agent+Driver 队列 seam）；本面只调 `Driver`，不持有 ReAct 队列。

## Debug 日志

**目标 / 现状（c460）**：debug 构建默认写即时日志；release 默认关。
- 路径：`~/.xylitol/logs/xylitol.log`（`<agent_dir>/logs/xylitol.log`）
- 查看：`tail -f ~/.xylitol/logs/xylitol.log`
- 覆盖：`RUST_LOG=…` 或 `XYLITOL_DEBUG=1`（release 也可用）
- 装配：`app/cli/logging.rs`；埋点 `target: "xylitol::tui"`。禁止 `println!`。

## 硬约束

- 渲染/通用组件只用 `xylitol_tui`；禁止在本目录再实现差分引擎或通用 Editor/Markdown。
- **需要底层 TUI 能力时**：先到 `packages/xylitol-tui` 查是否已有或可扩展；缺能力在包内补，再由本面接线。
- 产品路径 **host 驱动**同步引擎；异步事件合流在本面；勿调 `TUI::start()`（demo 专用）。
- 驱动 agent 只经 `app/core/driver::Driver`（含 `steer` / `follow_up` / `clear_queue`）；禁止 reach `agent::session` / `runtime` / `infra`。
- slash 语义复用 `protocol::Command`，经 `app/core/dispatch`。
- 组件不直接调 `Driver`、不读写 session；颜色走本面 theme 语义 token（对齐 `DESIGN.md`）。
- 已确认需求须有足够 **harness / 模拟环境** 验证（`HostEvent` 注入 + `TestTerminal`）；难且易错逻辑不外包给低质量实现。

## HOW（指针）

| 任务 | 去哪 |
|---|---|
| 写/改本面 | `write-tui` skill |
| UX / token / layout | 本目录 `DESIGN.md` |
| 底层能力是否已有 / 如何扩展 | `packages/xylitol-tui/AGENTS.md` |
| 新增/改造应用面 | `write-surface` skill |
| 包内组件与五层验证 | `test-tui-harness` skill |
| 排查（禁 println） | `tail -f ~/.xylitol/logs/xylitol.log` |

模块：`host.rs`（步进机）、`shell.rs`（空壳）、`terminal_guard.rs`（restore）、`tests.rs`（harness）。

# src/app/tui/

本面专属边界。分层与 seam：`src/AGENTS.md`。引擎库：`packages/xylitol-tui/AGENTS.md`。AGENTS 写法：根 `AGENTS.md`。

## 现状

基于 `xylitol-tui` 的 **host 驱动空 UI**（c460）：`HostSession` + `UiRoot` + 终端 lifecycle。`run()` 可进入；XyEvent / slash / steer 接线在后续 change。当前空场景仅为框架占位，**未**按 `DESIGN.md` 实现产品视觉。

## 优先路径（2026-07-10）

**不做 Codex 式 TranscriptView**（`c470` 已 `paused`；`app-tui-transcript` 已降级为 live scrollback）。下一优先：

1. ~~**c454** — 包 `TreeSelector`~~（已归档；`agent_demo` 双 Esc 冒烟已通）
2. **c456** — 收紧 demo 键位/文档（可选，冒烟已在 c454）
3. **c491** — 产品面双 Esc 会话树（travel/fork）

历史/分支 UX 以会话树为准；live 输出若有，只进 scrollback 行，见 `design/transcript.md` / `design/session-tree.md`。

## 视觉 / UX

产品终端视觉 SSOT：本目录 **`DESIGN.md`**（索引 + 全局 tokens，遵循 `common-design-md-zh`）与 **`design/*.md`**（组件级 MUST；`tokens_from: "../DESIGN.md"`，`{colors.*}` 等表达式解析到主文件）。包内组件只收闭包主题，不承载产品 layout。当前 `run()` 空场景仅为 host 框架占位，**未**按 DESIGN 实现产品视觉。

## Specs

产品面 capability：`app-tui-*`（`app-tui-host` / `bridge` / `transcript` / `chrome` / `input` / `commands`）。跨切面索引：`app-tui`。合约已归档：`archive/2026-07-10-c450-revise-app-tui-contract`。`app-tui-transcript` 壳仍在；**实现上不按 Codex 浏览面推进**。

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
| UX / token / layout | 本目录 `DESIGN.md` + `design/*` |
| 底层能力是否已有 / 如何扩展 | `packages/xylitol-tui/AGENTS.md` |
| 新增/改造应用面 | `write-surface` skill |
| 包内组件与五层验证 | `test-tui-harness` skill |
| 排查（禁 println） | `tail -f ~/.xylitol/logs/xylitol.log` |

模块：`host.rs`（步进机）、`ui_root.rs`（产品根布局）、`terminal_guard.rs`（restore）、`tests.rs`（harness）。勿用 `shell`/`scene` 命名，以免与 bash/`infra::process::shell` 或泛化「场景」混淆。

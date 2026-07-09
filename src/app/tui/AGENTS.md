# src/app/tui/ 终端 UI

本文件只放 `src/app/tui/` **专属**规则。分层不变量、应用面状态、seam（`composition → Driver → XyEvent`）的 SSOT 是 `src/AGENTS.md`；全局规则见根 `AGENTS.md`。通用引擎与组件库规则见 `packages/xylitol-tui/AGENTS.md`。

> **写或改 TUI？** 先读 `src/AGENTS.md` 的分层不变量与 `packages/xylitol-tui/AGENTS.md` 的已定决议，再用 `write-tui` skill（`.agents/skills/write-tui/SKILL.md`）。新增/改造应用面的方法论见 `write-surface`。

> **排查问题？** 禁止 `println!`/`eprintln!`/`dbg!`。走文件日志：`XYLITOL_DEBUG=1 cargo run --features tui --`，`tail -f ~/.xylitol/logs/xylitol.log`。完整指引见 `write-tui` skill。

> **现状**：旧实现（自研 engine / widgets / 旧 host）**已移除**。本目录为占位；即将基于 `packages/xylitol-tui` **从零重做** UI/UX（不以旧面为设计参考）。跨面 seam（`Driver` / `dispatch` / `composition` / `XyEvent`）保留。

## 入口与状态

入口链：`main.rs → lib::run → app::cli::run`（mode 分发）`→ app::tui::run`。

- `tui` feature 开启且进入 TUI 模式时，当前 `run()` 返回明确错误（占位），提示尚未基于 `xylitol-tui` 重做。
- 重做完成前，不要在本目录恢复旧 engine/widgets，也不要再实现第二套差分渲染引擎。

## 重做方向（硬边界）

- **渲染底座**：只用 `xylitol_tui`。产品路径 host 驱动（`dispatch_input` / `request_render` / `try_render` / `idle_tick`）；`TUI::start()` 仅 demo。
- **事件循环**：应用面拥有异步 host，合流键盘 / agent 事件 / tick；不把 tokio 绑进 `xylitol-tui`。
- **驱动 agent**：只经 `app/core/driver::Driver`；禁止 reach `agent::session` / `agent::runtime` / `infra`。
- **slash**：应用面解析，语义复用 `protocol::Command`，经 `app/core/dispatch`。
- **样式 / 主题 / 流式**：包层 `Vec<String>` + 闭包 theme；语义 token 与流式业务缓冲在本面（见 package `AGENTS.md` 已定决议）。

## 模块职责（重做后应对齐）

对标 kimi-code `apps/kimi-code` 的分工精神，适配单 crate + Driver seam：

- **host / coordinator**（`mod.rs` 等）：合流事件、驱动渲染；不堆可独立测试的业务块。
- **commands**：slash 声明与解析；执行走共享 dispatch。
- **surface / 组件装配**：组合 `xylitol_tui` 组件；组件层不直接调 `Driver`、不读写 session。
- **theme**：语义颜色 token SSOT → 映射为 xylitol-tui 闭包；组件不硬编码颜色字面量。

具体文件布局以重做落地 PR 为准；落地后更新本节地图，保持「只写本面专属、不重复 `src/AGENTS.md`」。

## TUI 专属约束（normative）

- slash 语义复用 `crate::protocol::Command`，经 `app::core::dispatch`。
- 颜色走 theme 语义 token。
- 组件只负责呈现与局部交互，禁止直接调 `Driver` 或读写 agent/session。
- 渲染与业务流解耦：UI 状态由应用面维护；通用 widget 优先进 `packages/xylitol-tui`。
- 需要新 agent 行为 → 不进 TUI；先扩 `runtime_protocol/` / `agent/`。

## Out of scope（本占位阶段）

- 在本目录重新实现差分引擎、键协议、通用 Editor/Markdown（一律用 `xylitol-tui`）。
- 把多 session / 业务编排逻辑写进 `packages/xylitol-tui`。

## 编码约定

- 不过度封装；一两行逻辑直接内联。
- 无 UI 副作用的纯函数放外部工具，不堆在 coordinator 私有方法里。
- 常量归 theme 或对应模块，不散落在逻辑代码里。

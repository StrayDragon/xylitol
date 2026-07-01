# src/app/tui/ 终端 UI

本文件只放 `src/app/tui/` **专属**规则。分层架构、不变量、应用面状态表、seam（`composition → Driver → XyEvent`）的单一真值源是 `src/AGENTS.md`，本文件不重复。全局规则见根 `AGENTS.md`。

> **写或改 TUI？** 先读 `src/AGENTS.md` 的分层不变量，再用 `write-tui` skill（`.agents/skills/write-tui/SKILL.md`，覆盖目标文件布局、新特性落点、测试放置、约定）。新增应用面的方法论总纲见 `write-surface` skill。

> **现状（2026-07-01）**：TUI 已落地（c340）。`mod.rs` 的 inline REPL 经 `InProcessDriver` 驱动，`render.rs` 把 `XyEvent` 流渲染到 inline viewport（`Viewport::Inline` + `insert_before`），`/exit` `/model` 两条 slash 命令可用。`diff_review/` 保留为未来集成的样例（c350），未接入 agent 主线。

## diff_review/ 的处置

`diff_review/` 是孤立 demo（与 agent 主线零契约），**不是** TUI 的起点，已在本轮（c340）明确保留不动。集成进 TUI 是 c350 的职责：届时经 Driver + XyEvent 重写为正式组件，删除 `run_demo`/`lib::run_review_demo`，不得在 diff_review 死码上直接扩展。

## 文件布局（已落地）

对标 kimi-code `apps/kimi-code/src/tui/`，适配单 crate + Driver seam。入口链：`main.rs → lib::run → app::cli::run`（mode 分发）`→ app::tui::run`（REPL 主循环）。目标目录：

- `mod.rs` — `run()` REPL 主循环：`loop { 读输入 → driver.run(prompt) → render }`。协调器，不堆业务逻辑。
c340（MVP）落地的文件：

- `mod.rs` — `run()` REPL 主循环：`tokio::select!` 三源（XyEvent mpsc / crossterm EventStream / cancel）。协调器，不堆业务逻辑。
- `terminal.rs` — `InlineTerminal`：`Viewport::Inline` 生命周期，RAII `Drop` 恢复 raw mode（spec tui15）。
- `app.rs` — `TuiApp` 状态机：输入缓冲 / 流式累积 / spinner / streaming 态；spawn 任务 drain `EventStream` → mpsc。
- `render.rs` — `XyEvent` → ratatui：`draw_tail_frame`（尾部）+ `commit_lines_for`（scrollback 行）。纯函数优先，可单测。
- `input.rs` — crossterm 键位 → `InputOutcome`（Submit/Slash/Abort/Quit/Idle）。MVP 单行输入。
- `commands.rs` — slash 解析 `/exit` `/model`，复用 `protocol::Command` 语义。
- `theme.rs` — 语义颜色 token 单一真值源。
- `diff_review/` — 保留（c350 集成）。

后续扩展（按需，不预先铺骨架）：`components/{messages,chrome,dialogs}/`、多行编辑器、FrameScheduler、EventBroker pause/resume。

## TUI 专属约束

下列是分层不变量之外的 TUI 局部约定：

- **slash 命令语义复用** `crate::protocol::Command` 同名变体（对标 `app/rpc.rs::dispatch`），不另造命令体系。
- **颜色一律走** `theme.rs` 语义 token，组件不得硬编码颜色字面量。
- **组件只负责呈现与局部交互**，禁止直接调 `Driver`、读写 agent/session 状态（即应用面 seam 约束的具体化，见 `src/AGENTS.md`）。
- **新 `XyEvent` 渲染 / transcript 消息类型** → `render.rs` 或 `components/messages/`；**需要新 agent 行为** → 不进 TUI，先在 `runtime_protocol/` 加 port、`agent/` 加实现，TUI 只消费。

## 编码约定

- 不过度封装，尤其一两行的函数直接内联，不套两层 wrapper。
- 无状态 / 无 UI 副作用的函数不作为 `mod.rs` 私有方法，放外部工具函数。
- 常量归 `theme.rs` 或对应 `components/` 内，不散落在逻辑代码里。
- Rust 命名遵循 `rustfmt.toml`：snake_case 模块/函数/变量，PascalCase 类型。

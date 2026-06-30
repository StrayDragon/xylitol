# src/app/tui/ 终端 UI

本文件只放 `src/app/tui/` **专属**规则。分层架构、不变量、应用面状态表、seam（`composition → Driver → XyEvent`）的单一真值源是 `src/AGENTS.md`，本文件不重复。全局规则见根 `AGENTS.md`。

> **写或改 TUI？** 先读 `src/AGENTS.md` 的分层不变量，再用 `write-tui` skill（`.agents/skills/write-tui/SKILL.md`，覆盖目标文件布局、新特性落点、测试放置、约定）。新增应用面的方法论总纲见 `write-surface` skill。

> **现状（2026-06-30）**：TUI 尚未真正落地。当前目录下只有 `diff_review/`——一个与 agent 主线零关联的交互式 diff 审查 demo（不引用 `XyEvent`/`Driver`/`agent::*`），其入口 `run_demo()` 经 `lib::run_review_demo()` 暴露但无调用方，属真死代码（见 `audit-dead-code`）。下列布局与职责是**目标**，落地时按需逐文件建，不要一次性铺空骨架。

## diff_review/ 的处置（先决）

`diff_review/` 是孤立 demo，**不是** TUI 的起点。建 TUI 面时先二选一处置（写进变更 `design.md`）：

- **剥离**成独立子命令（如 `xylitol review`），挪出 `app/tui/`。
- **删除**（含 `lib::run_review_demo` 及相关 `tui` feature 代码）。

禁止在 `diff_review/` 基础上扩展成 TUI——它与 agent 主线无契约，扩展它等于在死码上盖楼。

## 目标文件布局

对标 kimi-code `apps/kimi-code/src/tui/`，适配单 crate + Driver seam。入口链：`main.rs → lib::run → app::cli::run`（mode 分发）`→ app::tui::run`（REPL 主循环）。目标目录：

- `mod.rs` — `run()` REPL 主循环：`loop { 读输入 → driver.run(prompt) → render }`。协调器，不堆业务逻辑。
- `render.rs` — `XyEvent` 流 → ratatui 渲染（复用 `app/cli/print.rs` 渲染思路，换后端）。纯函数优先，便于单测。
- `input.rs` — 行编辑 / 键位解码（crossterm）。
- `commands.rs` — slash 命令声明 + 解析；执行复用 `protocol::Command` 语义（对标 `app/rpc.rs::dispatch`），不另造一套。
- `theme.rs` — 颜色 / 样式 token 单一真值源。
- `components/` — 按 UI 类型：`messages/`（transcript 块）、`chrome/`（footer/status）、`dialogs/`（selector/popup）。

落地顺序：`mod.rs` 最简 REPL + `render.rs` 跑通 `XyEvent` + `/exit` `/model` 两条命令 → 再扩。

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

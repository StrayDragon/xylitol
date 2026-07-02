---
name: "write-tui"
description: "编写或改造 xylitol 终端 UI（src/app/tui/）时使用。覆盖复用契约（经 Driver + XyEvent，绝不 reach into agent/infra）、文件布局、新特性落点、测试放置、编码约定。TUI 已落地（c340-c365），本 skill 是 how-to。"
---

# 编写 TUI（src/app/tui/）

xylitol 的终端 UI 位于 `src/app/tui/`（`tui` feature 门控，已落地：c340 inline REPL → c341 依赖瘦身 → c360 渲染 harness + RenderedLine seam → c365 流式 mutable-last-line + 组件化）。**写或改 TUI 之前**先读 `src/app/tui/AGENTS.md`（地图 + 模块边界 + 硬约束 + Out of scope）与 `src/AGENTS.md` 的「分层不变量」。

本 skill 是 **how-to**：复用契约、新特性落点、测试放置、约定。它对标 kimi-code 的 `write-tui` skill，但适配 xylitol 的 Rust 单 crate + Driver seam 架构。

**方法论总纲见 `write-surface` skill**（`.agents/skills/write-surface/SKILL.md`）——改 TUI 面之前先做死代码分诊（`audit-dead-code`）、只在复用契约内接线、必须端到端可跑通才算存在。本 skill 不重复方法论，只讲 TUI 特有部分。

## 1. 复用契约（TUI 专属硬约束）

TUI 是 `app/` 的一个面，受 `write-surface` 复用契约约束。对 TUI 而言具体是：

- **输入采集**（行编辑 / 键位 / 终端原始模式）：TUI 内部，用 `crossterm`（阻塞 `event::poll`/`read`，**不用** `EventStream`——会吞掉 ratatui inline 光标查询的 DSR 响应，见 `mod.rs` 头注释）。
- **渲染**（`XyEvent` 流 → transcript / mutable tail / 面板）：TUI 内部，直依 `ratatui-core` + `ratatui-crossterm`（**不引** umbrella `ratatui` crate，c341）。
- **驱动 agent**：**只能**经 `app/core/driver::Driver`（`InProcessDriver` 本地，或将来 `RemoteDriver` 连 server）。TUI 代码**禁止** import `crate::agent::session::*`、`crate::agent::runtime::*`、`crate::infra::*`（部分守卫：`arch_guard::app_driver_does_not_import_infra`）。
- **slash 命令**：TUI 内部解析（`commands.rs`）；执行语义复用 `protocol::Command` 同名变体（对标 `app/rpc.rs::dispatch`），当前是 MVP 本地分发，统一 dispatch 是 c335 的职责。
- **构造 agent**：**只能**经 `app/core/composition::build_agent`。TUI 不自己拼 `AgentBuilder`。

一句话：TUI 接收一个 `&mut dyn Driver`，对它 `run(prompt)`，把回来的 `XyEvent` 流喂给 ratatui buffer。除此之外不碰 core。

## 2. 文件布局（已落地）

对标 kimi-code `apps/kimi-code/src/tui/`，适配单 crate。地图与每个文件职责的 SSOT 是 `src/app/tui/AGENTS.md` 的「文件布局」段；本节只给一句话速览，细节不重复：

```
src/app/tui/
├── AGENTS.md            # 地图 + 边界 + 硬约束 + Out of scope（本面 SSOT）
├── mod.rs               # run() REPL 主循环（tokio::select! 多源）
├── init.rs              # panic 恢复 hook + 本地终端初始化
├── terminal.rs          # InlineTerminal（Viewport::Inline 生命周期）+ wrap_to_width
├── app.rs               # TuiApp 状态机（双 stream buffer + MutableKind）
├── render.rs            # XyEvent → RenderedLine 单一 seam + draw_tail_frame
├── input.rs             # 键位 → InputOutcome（MVP 单行）
├── commands.rs          # slash 解析 /exit /model（MVP 本地分发）
├── theme.rs             # Palette 语义颜色 token SSOT
└── components/          # 6 个可复用 widget（TestBackend 可独立验证）
    ├── transcript_line.rs   # RenderedLine → Buffer（wrap + CJK）
    ├── mutable_line.rs      # pending_tail 顶行（MutableKind 选 style）
    ├── input_prompt.rs      # 输入框（无 ❯）
    ├── bottom_panel.rs      # border + bg 容器（只含 InputPrompt，固定 3 行）
    ├── tail.rs              # 组合 MutableLine + BottomPanel
    └── spinner.rs           # 单 glyph spinner（底层组件，当前未接线）
```

改 TUI 时**沿此布局落子**，新增文件前先确认无既有归属（见第 4 节）。

## 3. 新特性落点

特性类型决定落点（对标 kimi-code write-tui 的 "Where new features go"）：

- **新的 `XyEvent` 渲染** → `render.rs` seam 函数加分支 + 必要时 `RenderedLine` 加变体。
- **slash 命令** → `commands.rs` 声明 + 解析；执行复用 `protocol::Command` 语义。
- **transcript 新消息类型** → `components/messages/` 下新增渲染组件（当前仅有 `transcript_line.rs`，按需拆）。
- **selector / popup / dialog** → `components/dialogs/`，并按 `DESIGN.md`（见第 5 节）的交互规范。
- **颜色 / 样式** → `theme.rs` 的 `Palette`，新增语义方法而非硬编码颜色。
- **mutable 内容新类别** → `app.rs` 的 `MutableKind` 加变体 + `tail.rs` 选 style。
- **需要新的 agent 行为** → **不进 TUI**。先在 `runtime_protocol/` 加 port、`agent/` 加实现，TUI 只消费。

## 4. 测试放置

- **渲染行为**：`ratatui_core::backend::TestBackend` + `assert_buffer_lines`，测行为非实现（c360 spec tui41）。每个 `components/` widget 有独立 harness 测试。
- **slash 命令解析**：`commands.rs` 内 `#[cfg(test)]`。
- **端到端（Driver + 事件流）**：用 `infra/provider/fake.rs` 的 `FakeModel` 喂确定性事件，避免真实 provider。
- 不为每个小特性新建测试文件，就近扩既有文件（对标 kimi-code "Test placement"）。

## 5. DESIGN.md（交互规范，按需创建）

TUI 出现第一个 dialog / selector / 复杂输入框时，在本目录建 `DESIGN.md`（对标 kimi-code `write-tui/DESIGN.md`），作为该面所有交互组件的单一真值源：选中指针、当前态标记、边框样式、hint 文案、颜色 token 对照、提交前自查清单。在此之前，复用契约 + `theme.rs` + 本 skill 即规范。

## 6. 提交前

- `just qa` 通过，`arch_guard` 不报新增的 `agent ↔ infra` 违规。
- 确认 TUI 代码没有 `crate::agent::session` / `crate::agent::runtime` / `crate::infra::*` 的 import。
- 确认没有为「暂时没用」的 TUI 骨架加 `#[allow(dead_code)]`（见 `audit-dead-code`）。
- 渲染层改动有 TestBackend 覆盖（不只是手写 cell 断言）。
- 若有 dialog/selector，走 `DESIGN.md` 自查清单。

---
name: "write-tui"
description: "编写或改造 xylitol 终端 UI（src/app/tui/）时使用。覆盖复用契约（经 Driver + XyEvent，绝不 reach into agent/infra）、目标文件布局、模块职责、编码约定。当前 TUI 尚未真正落地，本 skill 既是 how-to 也是规划。"
---

# 编写 TUI（src/app/tui/）

xylitol 的终端 UI 将位于 `src/app/tui/`（当前 `tui` feature 已存在，但 `tui/diff_review/` 是一个与 agent 主线无关的孤立 demo，见下文「现状」与 `src/app/tui/AGENTS.md`）。**写 TUI 之前**先读 `src/app/tui/AGENTS.md`（地图 + 模块边界 + 硬约束）与根 `AGENTS.md` 的「分层不变量」。

本 skill 是 **how-to**：复用契约、新特性落点、测试放置、约定。它对标 kimi-code 的 `write-tui` skill，但适配 xylitol 的 Rust 单 crate + Driver seam 架构。

**方法论总纲见 `write-surface` skill**（`.agents/skills/write-surface/SKILL.md`）——建 TUI 面之前先做死代码分诊（`audit-dead-code`）、只在复用契约内接线、必须端到端可跑通才算存在。本 skill 不重复方法论，只讲 TUI 特有部分。

## 1. 复用契约（TUI 专属硬约束）

TUI 是 `app/` 的一个面，受 `write-surface` 复用契约约束。对 TUI 而言具体是：

- **输入采集**（行编辑 / 键位 / 终端原始模式）：TUI 内部重写，可用 `crossterm`（已在 optional deps）。
- **渲染**（把 `XyEvent` 流画成 transcript / spinner / 状态栏）：TUI 内部重写，可用 `ratatui`（已在 optional deps）。
- **驱动 agent**：**只能**经 `app/core/driver::Driver`（`InProcessDriver` 本地，或将来 `RemoteDriver` 连 server）。TUI 代码**禁止** import `crate::agent::session::*`、`crate::agent::runtime::*`、`crate::infra::*`。
- **slash 命令**：TUI 内部解析；执行语义应复用 `protocol::Command` 的同名变体（对标 `app/rpc.rs` 的 dispatch），不要另造一套命令语义。
- **构造 agent**：**只能**经 `app/core/composition::build_agent`。TUI 不自己拼 `AgentBuilder`。

一句话：TUI 持有一个 `Box<dyn Driver>`，对它 `run(prompt)`，把回来的 `XyEvent` 流喂给 ratatui。除此之外不碰 core。

## 2. 目标文件布局（规划）

TUI 尚未真正落地，下列是目标结构，对标 kimi-code `apps/kimi-code/src/tui/` 但适配单 crate：

```
src/app/tui/
├── AGENTS.md            # 地图 + 边界 + 硬约束（本面的 SSOT 规则）
├── mod.rs               # 入口：run() REPL 主循环（loop { 读输入 → driver.run → render }）
├── render.rs            # XyEvent → ratatui 的流式渲染（复用 print.rs 的渲染思路，换后端）
├── input.rs             # 行编辑 / 键位解码（crossterm）
├── commands.rs          # slash 命令解析 → 复用 protocol::Command 语义
├── theme.rs             # 颜色/样式 token 单一真值源（对标 kimi-code theme/）
└── components/          # 按 UI 类型组织：messages/（transcript 块）、chrome/（footer/status）、dialogs/（selector/popup）
```

落地时**按需逐个建文件**，不要一次性铺满空骨架（那是逻辑死码的温床，见 `audit-dead-code`）。第一个里程碑：`mod.rs` 的最简 REPL + `render.rs` 复用 `XyEvent` + `/exit` `/model` 两条命令跑通。

## 3. 现状：`diff_review/` 是孤立 demo

当前 `src/app/tui/` 下只有 `diff_review/`（CLI/`mod.rs`/`types.rs`，约 2.5k 行）。它是一个独立的交互式 diff 审查 demo，**零引用** `XyEvent` / `Driver` / `agent::*`，且其入口 `run_demo()` 经 `lib::run_review_demo()` 暴露但无任何调用方。它是真死代码（见 `audit-dead-code` 第 3 节）。

建 TUI 面时，先处置它（二选一，写到变更的 `design.md` 里）：

- **剥离**：移成独立子命令（如 `xylitol review`），从 `app/tui/` 挪走，不再占 `tui` 名字。
- **删除**：若无独立价值，直接删 `diff_review/`、`lib::run_review_demo`、以及 `tui` feature 下与之相关的代码。

不要在 `diff_review/` 基础上「扩展成 TUI」——它和 agent 主线没有契约关系，扩展它等于在死码上盖楼。

## 4. 新特性落点

TUI 落地后，特性类型决定落点（对标 kimi-code write-tui 的 "Where new features go"）：

- **新的 `XyEvent` 渲染** → `render.rs` 的对应 match 分支。
- **slash 命令** → `commands.rs` 声明 + 解析；执行在 `mod.rs` 分派，复用 `protocol::Command` 语义。
- **transcript 新消息类型** → `components/messages/` 下新增渲染组件。
- **selector / popup / dialog** → `components/dialogs/`，并按 `DESIGN.md`（见第 6 节）的交互规范。
- **颜色 / 样式** → `theme.rs`，新增语义 token 而非硬编码颜色（对标 kimi-code 的 color-token 约束）。
- **需要新的 agent 行为** → **不进 TUI**。先在 `runtime_protocol/` 加 port、`agent/` 加实现，TUI 只消费。

## 5. 测试放置

- `XyEvent → 渲染` 的纯函数测试：`src/app/tui/render.rs` 内 `#[cfg(test)]` 或 `tests/` 下就近文件。
- slash 命令解析：`commands.rs` 内 `#[cfg(test)]`。
- 端到端（Driver + 事件流）：用 `infra/provider/fake.rs` 的 `FakeModel` 喂确定性事件，避免真实 provider。
- 不为每个小特性新建测试文件，就近扩既有文件（对标 kimi-code "Test placement"）。

## 6. DESIGN.md（交互规范，按需创建）

TUI 出现第一个 dialog / selector / 输入框时，在本目录建 `DESIGN.md`（对标 kimi-code `write-tui/DESIGN.md`），作为该面所有交互组件的单一真值源：选中指针、当前态标记、边框样式、hint 文案、颜色 token 对照、提交前自查清单。在此之前，复用契约 + 本 skill 即规范。

## 7. 提交前

- `just qa` 通过，`arch_guard` 不报新增的 `agent ↔ infra` 违规。
- 确认 TUI 代码没有 `crate::agent::session` / `crate::agent::runtime` / `crate::infra::*` 的 import。
- 确认没有为「暂时没用」的 TUI 骨架加 `#[allow(dead_code)]`（见 `audit-dead-code`）。
- 若有 dialog/selector，走 `DESIGN.md` 自查清单。

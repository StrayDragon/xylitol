---
name: "write-tui"
description: "编写或改造 xylitol 终端 UI（src/app/tui/）时使用。覆盖复用契约（经 Driver + XyEvent，绝不 reach into agent/infra）、与 packages/xylitol-tui 的边界、新特性落点、测试放置。旧 ratatui/自研 engine 实现已移除；本 skill 描述基于 xylitol-tui 的重做方向。"
---

# 编写 TUI（src/app/tui/）

> **现状**：旧 `src/app/tui/` 实现已删除并占位。产品 TUI 将基于 `packages/xylitol-tui` **从零重做**（UI/UX 不继承旧面）。写/改前先读：
>
> 1. `src/app/tui/AGENTS.md`（本面地图 + 硬约束）
> 2. `packages/xylitol-tui/AGENTS.md`（引擎/组件库边界与已定决议）
> 3. `src/AGENTS.md`（分层不变量）
>
> 方法论总纲见 `write-surface` skill。本 skill 是 TUI how-to，不重复分层事实。

xylitol 的终端 UI 位于 `src/app/tui/`（`tui` feature 门控）。渲染与通用组件来自 workspace 包 `xylitol-tui`，**禁止**在 `src/app/tui/` 再实现一套差分引擎或通用 Editor/Markdown。

## 1. 复用契约（TUI 专属硬约束）

TUI 是 `app/` 的一个面，受 `write-surface` 复用契约约束。对 TUI 而言：

- **渲染 / 组件 / 键协议**：只用 `xylitol_tui`（`TUI`、`Component`、`Editor`/`Input`/`Markdown`/…）。本 package 是同步库；产品面 **host 驱动**（`dispatch_input` / `request_render` / `try_render` / `idle_tick`），**不要**在产品路径调用 `TUI::start()`（那是 demo 用）。
- **事件合流**：应用面拥有异步 host 循环（如 `tokio::select!`），合流键盘、agent 事件、tick 等；不要把 tokio 绑进 `xylitol-tui`。
- **驱动 agent**：**只能**经 `app/core/driver::Driver`。禁止 import `crate::agent::session::*`、`crate::agent::runtime::*`、`crate::infra::*`。
- **slash 命令**：应用面解析；执行语义复用 `protocol::Command`，经 `app/core/dispatch`。
- **构造 agent**：**只能**经 `app/core/composition::build_agent`。

一句话：TUI 拿 `&mut dyn Driver`，`run(prompt)`，把 `XyEvent` 流变成组件状态，再驱动 `xylitol_tui` 出帧。除此之外不碰 core。

## 2. 与 packages/xylitol-tui 的分工

| 放 `packages/xylitol-tui` | 放 `src/app/tui/` |
|---|---|
| 差分引擎、终端抽象、键解码、通用组件 | App Shell（host 循环）、UX 状态机 |
| `Container` / Overlay / theme 闭包接口 | 语义 theme token → 闭包映射 |
| 五层测试的 1–4 层 | Driver seam、slash、`XyEvent`→UI 状态 |

已定决议（细节 SSOT：`packages/xylitol-tui/AGENTS.md`）：`Vec<String>` ANSI 样式；主题闭包在包层、语义 token 在应用面；流式业务缓冲在应用面。

## 3. 文件布局（重做目标）

旧 engine/widgets/ratatui 路径已作废，**不要**恢复。重做时沿此方向落子（具体文件名以落地 PR 为准，先读当时的 `src/app/tui/AGENTS.md`）：

```
src/app/tui/
├── AGENTS.md       # 本面 SSOT
├── mod.rs          # run()：异步 host 循环，驱动 xylitol_tui
├── …               # surface / theme / commands / 状态机等按设计落地
```

新增文件前确认无既有归属；通用 widget 优先进 `packages/xylitol-tui`，不要在应用面复制。

## 4. 新特性落点

- **新的 `XyEvent` 呈现** → 应用面 seam（事件→UI 状态），再更新组件。
- **slash 命令** → 应用面 commands + `app/core/dispatch`。
- **通用交互组件**（列表/编辑/markdown）→ `packages/xylitol-tui`；先查是否已有。
- **颜色 / 语义样式** → 应用面 theme（token → xylitol-tui 闭包）。
- **需要新的 agent 行为** → **不进 TUI**。先扩 `runtime_protocol/` / `agent/`，TUI 只消费。

## 5. 测试放置

- **通用组件 / 引擎**：`packages/xylitol-tui` 五层 harness（见该 package `AGENTS.md`）。
- **应用面 seam / slash**：就近 `#[cfg(test)]` 或既有测试文件；优先扩既有，不为小特性新建。
- **端到端**：`infra/provider/fake` 的确定性事件；真终端 E2E 走 `just test-tui-e2e`。

## 6. 提交前

- `just qa`；`arch_guard` 无新增违规。
- TUI 无 `crate::agent::session` / `runtime` / `infra::*` import。
- 不为「暂时没用」的骨架加 `#[allow(dead_code)]`（见 `audit-dead-code`）。
- 渲染相关改动有对应层测试（包内或应用面）。

## 7. 排查 TUI 问题（debug 日志）

**禁止**在 TUI 路径使用 `println!`/`eprintln!`/`dbg!`（会与 raw mode / 差分输出交错毁屏）。走文件日志。

### 日志管线

- 装配：`src/app/cli/logging.rs::init_logging()`（组合根内、模式分发前）。
- 激活：`RUST_LOG=<directive>` 或 `XYLITOL_DEBUG=1`（默认 `xylitol=debug,warn`）；都不设则不装 subscriber。
- 落点：`~/.xylitol/logs/xylitol.log`（file-only，不碰 stdout/stderr）。

```bash
XYLITOL_DEBUG=1 cargo run --features tui --
RUST_LOG=xylitol=trace cargo run --features tui --
```

埋点只用 `tracing::` 宏（`target: "xylitol::tui"` 等）；不要在 TUI 里装 subscriber 或 import `infra`。

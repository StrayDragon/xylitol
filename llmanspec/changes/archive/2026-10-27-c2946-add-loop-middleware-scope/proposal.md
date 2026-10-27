---
depends_on:
  - c2944-add-loop-middleware
title: Loop 中间件会话作用域与嵌入注册入口
branch: sdd/c2946-add-loop-middleware-scope
base_branch: main
base_sha: f8e66061f052c68d30182c19644095ba363ce20b
---

# Loop 中间件会话作用域与嵌入注册入口

## Why

c2944 的注册表是 **root `Ctx` 全局服务**（`agent-react` 一次 `provide_as`），因此：

- **Host 多会话共享一条链**：`ProductOccupancyFactory::new_driver_at` 给每个 session 槽取同一个 registry。插件无法「只对某个会话启用」「按工作区启用」。
- **库嵌入无入口**：`xylitol::embed` 只导出 `build_agent` / `assemble_driver`，不暴露注册入口，嵌入者只能手工摸 `AgentRuntime::set_loop_middleware`。

## What Changes

- `LoopMiddlewareRegistry` 支持 **派生子作用域**：`derive()` 冻结父链当前快照并允许本地注册；父后续增删不影响子，子增删不影响父。
- **Host 每 session 槽一份**：槽创建时从 root 派生，driver 用槽内那份。已存在 session 不受新挂中间件影响（延续 c2944「run 启动快照」语义）。
- **embed 注册入口**：`xylitol::embed::with_loop_registry(driver, registry)`，让嵌入者在不接触 `rutis` 类型的前提下注册。
- **释放语义**：作用域是纯数据，槽丢弃即释放，不得泄漏到 root。
- 观测：trace 增加 `loop.middleware.scope`（`root` / `session`，不做全 id）。

## Out of Scope

- 第三方插件市场 / 动态安装。
- 跨进程（Host↔TUI）传递中间件链。
- 按 turn 动态增删中间件（仍为 run 启动快照）。
- 用 rutis `isolate` / instance key 做服务级作用域（留给未来「按会话替换 compaction / permission」等独立 change）。

## Capabilities

- `agent-runtime`：会话作用域 MUST + 场景（会话隔离、root 默认、释放不泄漏）。
- `workspace-assembly`：embed 注册入口（不泄漏 `rutis` 类型）可由 crate 图门禁 + 单测覆盖。

## Impact

- crate：`xylitol-plugin-sdk`（`derive`）、`xylitol-kernel`（槽接线）、`xylitol-runtime`（无需改）、根包 `src/embed.rs`。
- 测试：sdk 单测（派生隔离 / 父快照冻结）、kernel 契约测试（两槽互不影响）、BDD 会话隔离场景。
- 文档：`src/AGENTS.md` 作用域说明。

## Further Notes

来源：c2944 verify 的缺口 G/K。

方案取舍（2026-10-27）：曾评估用 rutis `isolate` 做框架级作用域，实测 `Ctx::isolate` 复用原 fiber（不建新 fiber），且 Host 的 `sessions` map 只增不删、`SessionSlot` 无 `Drop`、`Disposer` 无自动清理——「随 fiber 卸载消失」在该路径下不可达。链是**数据**不是**服务**，故取注册表派生（纯数据，槽丢弃即释放）；服务级作用域留给未来独立 change。

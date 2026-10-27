---
depends_on:
  - c2944-add-loop-middleware
title: Loop 中间件会话作用域与嵌入注册入口
---

# Loop 中间件会话作用域与嵌入注册入口

## Why

c2944 的注册表是 **root `Ctx` 全局服务**（`agent-react` 一次 `provide_as`），因此：

- **Host 多会话共享一条链**：`ProductOccupancyFactory::new_driver_at` 给每个 session 槽取同一个 registry。插件无法「只对某个会话启用」「按工作区启用」。
- **库嵌入无入口**：`xylitol::embed` 只导出 `build_agent` / `assemble_driver`，不暴露 `Ctx`，嵌入者只能手工 `set_loop_middleware`，第三方插件在 embed 场景等于不可用。

## What Changes

- 注册表支持 **scoped 实例**：`LoopMiddlewareRegistry::new()` 可派生子作用域（父链继承 + 本地覆盖），Host 每个 session 槽取 session 作用域实例；root 作用域仍为默认。
- 作用域解析口径：session 槽创建时从 root 快照派生；已存在 session 不受新挂中间件影响（延续 c2944「run 启动快照」语义）。
- `xylitol::embed` 暴露最小注册入口：`embed::loop_middleware_registry()` 或 `embed::with_loop_registry(...)`，让嵌入者在不接触 `rutis` 类型的前提下注册（不泄漏 `Ctx` 到库入口）。
- **卸载语义**：session 槽释放时其作用域随 fiber 卸载消失，不得泄漏到 root。
- 观测：trace 增加 `loop.middleware.scope`（`root` / `session:<id>` 摘要，不做全 id）。

## Out of Scope

- 第三方插件市场 / 动态安装。
- 跨进程（Host↔TUI）传递中间件链。
- 按 turn 动态增删中间件（仍为 run 启动快照）。

## Capabilities

- `agent-runtime`：新增会话作用域 MUST + 场景（会话隔离、root 默认、卸载不泄漏）。
- `workspace-assembly`：embed 注册入口（不泄漏 `rutis` 类型）可由 crate 图门禁 + 单测覆盖。

## Impact

- crate：`xylitol-plugin-sdk`（作用域派生）、`xylitol-kernel`（session 槽接线）、`xylitol-runtime`（快照来源）、根包 `src/embed.rs`。
- 测试：kernel 契约测试（两 session 槽互不影响）、BDD 会话隔离场景。
- 文档：`src/AGENTS.md` 作用域说明；`llmanspec/AGENTS.md` 无需改。

## Further Notes

来源：c2944 verify 的缺口 G/K。

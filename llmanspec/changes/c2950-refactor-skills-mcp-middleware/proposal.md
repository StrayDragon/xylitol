---
depends_on:
  - c2944-add-loop-middleware
title: Skills 展开与 MCP 结果摘要迁为官方中间件
---

# Skills 展开与 MCP 结果摘要迁为官方中间件

## Why

c2944 建立中间件 SPI 后，两处「本应挂中间件却仍在 core / 旁路」的缝还在：

- **Skills 展开**：`expand_skills_in_agent_messages` 在 `run_react_loop` 里直接调用（`before_model` 之前）。按目标形态应是官方 middleware 挂 `before_model`，可被后续插件再挂一层。c2944 为守「无 middleware = 基线」把 `before_model` 放在展开之后，属于有意让步。
- **MCP 结果摘要**：`tool_result_post` 脚本 hook 承担截断/摘要，MCP bridge 自身不摘要。按目标形态摘要应是 `after_tool` 中间件，且不硬编码进 MCP bridge。

## What Changes

- 新增官方插件（或扩展 `xylitol-official-plugin-loop-example` 的同类位置）提供 `SkillsExpandMiddleware`，挂 `before_model`、order 取最小（链首），默认启用。
- core 移除内联 `expand_skills_in_agent_messages` 调用；`before_model` 执行点前移到 skills 展开之前。
- **基线等价**：未启用任何第三方中间件时，messages 形状与今日逐字节一致（skills 展开结果不变）。
- MCP 结果摘要：在官方中间件侧提供 `after_tool` 摘要能力（截断策略走既有 `obs_gate` 常量），MCP bridge 不改。
- 卸载：插件停用后 messages 恢复未展开形状（无残留）。
- 观测：`loop.middleware.changed` 能指认 skills 展开导致的 messages 变化。

## Out of Scope

- 改变 skills 发现 / 热重载语义（仍走 `runtime-resource-discovery`）。
- MCP 传输 / 准备门改造。
- 让第三方插件替换 skills 展开（只允许再叠一层）。

## Capabilities

- `agent-runtime`：skills 展开经中间件 MUST + 场景（默认启用等价、卸载恢复、order 最前）。
- `agent-prompt`：skills 展开结果形状不变（回归）。
- `infra-mcp`：摘要走 `after_tool`、bridge 不硬编码（核对既有条款）。

## Impact

- crate：`xylitol-plugin-sdk`（无）、`xylitol-runtime`（移除内联展开 + 执行点前移）、新官方插件或扩展现有插件、`xylitol-kernel`（默认装配）。
- 测试：既有 skills 展开单测改挂中间件；BDD 基线等价场景；`agent-prompt` 回归。
- 文档：`src/AGENTS.md`「Loop 中间件」段补 skills 归属。

## Further Notes

来源：c2944 verify 的缺口 L。c2944 的 design §3「为什么 before_model 在 skills 展开之后」是本 change 的输入约束。

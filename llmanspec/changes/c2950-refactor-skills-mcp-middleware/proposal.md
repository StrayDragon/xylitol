---
depends_on:
  - c2944-add-loop-middleware
  - c2946-add-loop-middleware-scope
title: Skills 展开与 MCP 结果截断迁为官方中间件
---

# Skills 展开与 MCP 结果截断迁为官方中间件

## Why

c2944 建立中间件 SPI 后，两处「本应挂中间件却仍在 core / 旁路」的缝还在：

- **Skills 展开**：`expand_skills_in_agent_messages` 在 `run_react_loop` 里直接调用（`before_model` 之前）。按目标形态应是官方 middleware 挂 `before_model`，可被后续插件再挂一层。c2944 为守「无 middleware = 基线」把 `before_model` 放在展开之后，属于有意让步。
- **MCP 结果**：MCP 工具输出当前**无任何截断或摘要**——adapter 把调用结果 `serde_json::to_string` 全量返回。`tool_result_post` 只是用户脚本 hook 的事件位，缺省 `post_tool_call` 为空，不承担截断。按目标形态截断应是官方 `after_tool` 中间件，且不硬编码进 MCP bridge。

## What Changes

- 新增官方插件（或扩展 `xylitol-official-plugin-loop-example` 的同类位置）提供 `SkillsExpandMiddleware`，挂 `before_model`、order 取最小（链首），默认启用。
- core 移除内联 `expand_skills_in_agent_messages` 调用；`before_model` 执行点前移到 skills 展开之前。
- **基线等价**：未启用任何第三方中间件时，messages 形状与今日逐字节一致（skills 展开结果不变）。该等价须有 BDD 场景钉住。
- MCP 结果截断：在官方中间件侧提供 `after_tool` 截断能力，MCP bridge 不改。**这是新增产品行为**（今日无截断），须显式定上限（既有工具截断口径 `DEFAULT_MAX_BYTES = 50 KiB`，非观测 IO 常量）与是否可配置，不得以「搬家」措辞掩盖。
- 卸载：插件停用后 messages 恢复未展开形状（无残留）。
- 观测：`loop.middleware.changed` 能指认 skills 展开导致的 messages 变化。

## Out of Scope

- 改变 skills 发现 / 热重载语义（仍走 `runtime-resource-discovery`）。
- MCP 传输 / 准备门改造。
- 让第三方插件替换 skills 展开（只允许再叠一层）。
- 复用观测 IO 截断常量（`PROVIDER_TRACE_TEXT_MAX` / `OBSERVATION_IO_FULL_MAX`）作为工具输出上限——两者语义无关。

## Capabilities

- `agent-runtime`：skills 展开经中间件 MUST + 场景（默认启用等价、卸载恢复、order 最前）。
- `agent-prompt`：skills 展开结果形状不变（回归）。
- `infra-mcp`：MCP 结果截断走 `after_tool` 中间件、bridge 不硬编码；须定上限口径（新增行为，非既有条款核对）。

## Impact

- crate：`xylitol-plugin-sdk`（无）、`xylitol-runtime`（移除内联展开 + 执行点前移）、新官方插件或扩展现有插件、`xylitol-kernel`（默认装配）。
- 测试：既有 skills 展开单测改挂中间件；BDD 基线等价场景；`agent-prompt` 回归；MCP 截断上限边界。
- 文档：`src/AGENTS.md`「Loop 中间件」段补 skills 归属。

## Further Notes

来源：c2944 verify 的缺口 L。c2944 的 design §3「为什么 before_model 在 skills 展开之后」是本 change 的输入约束。

事实修正（2026-10-27 核对）：提案早期版本称「MCP 摘要由 `tool_result_post` 脚本 hook 承担、截断走 `obs_gate` 常量」，实测两者均不成立——`tool_result_post` 缺省空、MCP adapter 无截断、`obs_gate` 常量属观测 IO。MCP 部分按「新增能力」重述。

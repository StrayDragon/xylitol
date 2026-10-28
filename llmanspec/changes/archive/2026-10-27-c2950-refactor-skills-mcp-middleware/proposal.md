---
depends_on:
  - c2944-add-loop-middleware
  - c2946-add-loop-middleware-scope
title: Skills 展开内置化与 MCP 结果截断
branch: sdd/c2950-refactor-skills-mcp-middleware
base_branch: main
base_sha: 3ec6a817902f55c28f601aec69e7d616b21d22ec
---

# Skills 展开内置化与 MCP 结果截断

## Why

c2944 建立中间件 SPI 后，两处「本应挂在 loop 决策点上却仍在 core / 旁路」的缝还在：

- **Skills 展开**：`expand_skills_in_agent_messages` 在 `run_react_loop` 里直接调用（`before_model` 之前）。c2944 为守「无 middleware = 基线」把 `before_model` 放在展开之后，属于有意让步——第三方中间件看不到展开后的 messages，也无法在展开前介入。
- **MCP 结果**：MCP 工具输出当前**无任何截断**——adapter 把调用结果 `serde_json::to_string` 全量返回。`tool_result_post` 只是用户脚本 hook 的事件位，缺省 `post_tool_call` 为空，不承担截断。

## What Changes

### Skills 展开 = 内置项（不是可注册插件）

- 把 skills 展开定义为 **loop 内置项**：与 `before_model` 同阶段、**先于**任何第三方 `before_model` 中间件执行。
- 内置项 **不进入** `LoopMiddlewareRegistry`，不产生 `loop.middleware` span，不占 order 位。
- `r2931` 的「未配置 root 链须为空」口径随之澄清：**空链指无第三方可注册中间件**；内置项不计入链。
- 第三方仍可在展开**之后**叠 `before_model`（看到展开后的 messages），但**不得**替换或跳过展开。

### 基线等价

- 未注册任何第三方中间件时，messages 形状与今日**逐字节一致**（skills 展开结果不变、位置不变）。
- 该等价须有 BDD 场景钉住。

### MCP 结果截断

- 在官方中间件侧提供 `after_tool` 截断能力，MCP bridge 不改。
- **这是新增产品行为**（今日无截断）：须显式定上限口径（对齐既有工具截断 `DEFAULT_MAX_BYTES = 50 KiB`，非观测 IO 常量）与是否可配置。
- 截断须在 tool result 进入 history 前生效，且不得破坏结构化分片（`r2929`）。

## Out of Scope

- 改变 skills 发现 / 热重载语义（仍走 `runtime-resource-discovery`）。
- MCP 传输 / 准备门改造。
- 让第三方插件替换或跳过 skills 展开（只允许在其后叠加）。
- 复用观测 IO 截断常量（`PROVIDER_TRACE_TEXT_MAX` / `OBSERVATION_IO_FULL_MAX`）作为工具输出上限——两者语义无关。
- 改 `MessageUpdate` 载荷形状（journal 内存问题，独立评估）。

## Capabilities

- `agent-runtime`：skills 展开内置化 + `before_model` 执行顺序 + 基线等价场景；`r2931` 空链口径澄清。
- `agent-prompt`：skills 展开结果形状不变（回归）。
- `infra-mcp`：MCP 结果截断走 `after_tool` 中间件、bridge 不硬编码；须定上限口径（新增行为）。

## Impact

- crate：`xylitol-plugin-sdk`（无）、`xylitol-runtime`（展开点前移 + 内置项接线）、官方插件（MCP 截断）、`xylitol-kernel`（默认装配）。
- 测试：既有 skills 展开单测；BDD 基线等价场景；`agent-prompt` 回归；MCP 截断上限边界。
- 文档：`src/AGENTS.md`「Loop 中间件」段补内置项 vs 可注册项的分层。

## Further Notes

来源：c2944 verify 的缺口 L。c2944 的 design §3「为什么 before_model 在 skills 展开之后」是本 change 的输入约束。

事实修正（2026-10-27 核对）：提案早期版本称「MCP 摘要由 `tool_result_post` 脚本 hook 承担、截断走 `obs_gate` 常量」，实测两者均不成立——`tool_result_post` 缺省空、MCP adapter 无截断、`obs_gate` 常量属观测 IO。MCP 部分按「新增能力」重述。

定位决策（2026-10-27）：skills 展开**不**做成默认启用的可注册插件——那会与 c2946 刚落地的 `r2931`「未配置 root 链须为空」直接冲突。改为**内置项**：skills 展开是产品必需行为（影响 messages 形状，与 core 持消息落盘格式同层），不是可选能力；中间件 SPI 仍只承载第三方。

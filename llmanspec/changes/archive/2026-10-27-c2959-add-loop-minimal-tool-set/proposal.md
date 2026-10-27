---
depends_on:
  - c2957-add-loop-example-plan-state
title: 最小工具集：按当前步骤、身份与风险收窄模型可见工具
branch: sdd/c2959-add-loop-minimal-tool-set
base_branch: main
base_sha: 510895537332591f04ada9baa509012d820b66a7
---

# 最小工具集：按当前步骤、身份与风险收窄模型可见工具

## Why

今天每次 `before_model` 都把**全部已装配工具**的 schema 交给模型：

```text
BeforeModelCtx.tool_schemas = turn_tool_schemas.clone()
```

工具越挂越多，模型选择空间膨胀、同名能力冲突、越权面扩大。ADPS A5 的要点是：候选集由代码/策略计算，模型只在**已准入集合**中选择；限制的是单步可见范围，不是删除系统能力。

现状证据：

```text
crates/xylitol-plugin-sdk/src/dto/chunk.rs
  XyToolSchema { name, description, parameters }   ← 无 read_only / risk / requires_approval

crates/xylitol-plugin-sdk/src/ports/tool.rs
  XyTool 只有 execution_mode()（并发/串行）与 prompt 片段，无风险元数据

crates/xylitol-runtime/src/agent/runtime/react/mod.rs
  before_model 已可改 tool_schemas，但没有任何过滤策略
```

## What Changes（候选，promote 时逐条重估）

### 1. `before_model` 过滤

官方 `tool-exposure` 中间件在 `before_model` 改写 `ctx.tool_schemas`：

```text
候选集 = 全部已装配工具
  ∩ 当前 plan step 的 allowlist（若启用 plan-as-state）
  ∩ 身份/信任允许的工具类别（只读 / 可写 / 需审批）
  ∩ 风险闸（高风险工具仅在显式步骤可见）
```

被过滤的工具**仍在 ToolSet 中**，只是本步不可见；不得删除工具、不得改权限裁决。

### 2. 元数据来源

| 方案 | 成本 | 说明 |
|---|---|---|
| **A. 配置映射**（推荐先做） | 零 core | `tool_exposure.rules`：tool name → `{ read_only, risk, steps }`；缺省全部可见 |
| B. core 元数据 | 中 | `XyTool` 增 `ToolMetadata { read_only, risk, requires_approval, concurrency_safe }`，经 `BeforeModelCtx` 暴露 |
| C. 从权限引擎推导 | 低-中 | 用 `permission_target` / 权限配置推导写路径与网络风险，但无法表达「当前步骤」 |

先做 A，证明过滤有效后再评估 B；C 只作为 B 的补充。

### 3. 与 plan-as-state 的衔接

若 `c2957` 的 plan 状态可见（todo 的 `in_progress` 项），按步骤映射 allowlist；没有 plan 时退回身份/风险两维。不得假设 todo 一定存在。

### 4. 可观测

每次过滤记录：候选数、过滤后数、被过滤工具与理由（trace span / 日志），便于回答「为什么这个工具不可见」。

## Capabilities

- `agent-runtime`：`before_model` 的工具过滤行为合约（不删除、不越权、可见性可解释）。
- `runtime-config`：`tool_exposure` 配置节。
- `tool-system`（或现有 tool capability）：工具风险元数据（若走方案 B）。

## Impact

- crate：`plugins/xylitol-official-plugin-tool-exposure`（新官方插件）或扩展现有插件；方案 B 才动 `xylitol-plugin-sdk` / `xylitol-runtime`。
- 配置：`tool_exposure.enabled` / `rules`。
- 测试：过滤后 schema 集合、被过滤工具仍可经显式步骤恢复、权限裁决不变、空规则不改形状。
- 文档：`src/AGENTS.md` 补「工具可见性收窄不是权限裁决」。

## Out of Scope

- 删除工具或改变 permission 终裁。
- MCP 前缀的串行纪律（仍由 core 保持）。
- 用模型自评决定工具可见性。
- 为每个工具手写自然语言风险描述。

## Further Notes

**ROI**：高。纯 `before_model` 插件，直接降 token 与误选；默认关闭，风险可控。

**开放问题**：

1. 当前步骤的来源：todo 的 `in_progress` 是否足够，还是需要 A2 的 `PlanStep`？
2. 身份/信任维度是否已有稳定输入（trust 门禁 / permission config）？
3. 过滤是否应影响 `tool_schemas` 之外的工具提示片段（`prompt_snippet` / `prompt_guidelines`）？建议一并过滤，避免提示泄漏不可见工具。
4. 与 `r2922`「路由目标须已装配」的关系：过滤只影响模型可见性，不影响 `before_tool` 路由目标校验。

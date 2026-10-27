---
depends_on:
  - c2957-add-loop-example-plan-state
title: 生成评审：critic 指令注入与有界修订循环
---

# 生成评审：critic 指令注入与有界修订循环

## Why

单次生成缺少复核。ADPS F1 的 Generator-Critic 要求：Generator 产出 → Critic 按明确 rubric 评估（带证据引用）→ Generator 修订；Critic 要有停止条件，避免同一个模型自评自改产生 phantom issue。

现状证据：

```text
crates/xylitol-plugin-sdk/src/loop_middleware.rs
  AfterModelCtx { turn_index, assistant }
  BeforeModelCtx { turn_index, messages, tool_schemas, options }
  DecideContinueCtx { turn_index, settle_count, has_new_user_messages, assistant, history_len, compact }

crates/xylitol-runtime/src/agent/runtime/react/mod.rs
  after_model 可改写 assistant；decide_continue 的 Continue 由 core 调度下一轮
```

## What Changes（候选，promote 时逐条重估）

### 1. 状态机

官方 `generator-critic` 中间件：

```text
Idle
  --after_model(assistant 无 tool_calls 且 critic 启用)--> Critiquing
Critiquing
  --before_model--> 注入 critic 指令（rubric + 证据要求 + 只输出结构化 verdict）
  --decide_continue--> Continue（critic 回合）
Critiqued
  --after_model(读取 verdict)-->
      verdict = approve  --> Stop（或回到 Idle，按配置）
      verdict = revise   --> 注入修订指令，Continue
      达到 max_rounds   --> Stop（带稳定错误或摘要）
```

**关键**：critic 是 core 调度的**下一次模型调用**；插件只注入指令与状态，不自行调模型。

### 2. rubric 与 verdict

- rubric 可配置（正确性、证据、边界、可验证性等）；
- critic 输出必须是结构化 verdict（JSON 或固定前缀），便于中间件判断；
- 没有结构化 verdict 时按 `revise` 处理一次，超过一次则停止，避免无限循环。

### 3. 有界

`max_rounds`（默认 1–2）小于 core 的 32；达到上限时停止并输出可观测摘要，不得静默继续。

### 4. 与工具结果的关系

critic 可以要求引用工具结果/文件路径；但中间件不读取文件系统，只把 rubric 与「必须引用证据」写进指令，由模型自行取证。

## Capabilities

- `agent-runtime`：`after_model` / `before_model` / `decide_continue` 的评审状态机合约；有界与停止语义。
- `runtime-config`：rubric 与 `max_rounds` 配置。
- `agent-prompt`：critic 指令的注入位置与长度。

## Impact

- crate：`plugins/xylitol-official-plugin-generator-critic`（新官方插件）或扩展现有 loop 插件。
- 配置：`generator_critic.enabled` / `rubric` / `max_rounds`。
- 测试：approve 停止、revise 续跑、无 verdict 只重试一次、达到上限停止、禁用时基线不变。
- 文档：`src/AGENTS.md` 补「critic 是 core 调度的模型调用，不是插件内调用」。

## Out of Scope

- 独立 critic 模型/提供商（先同模型，后续再评估）。
- 自动改代码/自动提交。
- 用 critic 替代测试、规则或人工判断（高风险场景仍以确定性验证为准）。

## Further Notes

**ROI**：中。能提升单次输出质量，但成本是额外模型调用；默认关闭，按任务/配置启用。

**开放问题**：

1. verdict 的结构化格式（JSON schema vs 固定前缀）与解析失败策略。
2. critic 是否应看到完整工具轨迹？`BeforeModelCtx.messages` 有历史，但可能需要显式引用工具结果。
3. 与 `c2961`（自愈）的边界：自愈处理确定性失败，critic 处理质量评审；两者可同时启用，但都要有界。
4. 是否允许 critic 触发 `compact`？建议不允许，避免评审回合把上下文压缩掉。

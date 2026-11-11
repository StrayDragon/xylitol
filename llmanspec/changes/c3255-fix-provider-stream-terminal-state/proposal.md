---
depends_on: []
title: "Fix provider stream terminal chunk semantics"
---

# Provider 流终态语义修复

## Why

XyChunk 流的终态约束（ToolCallEnd 唯一、Done 唯一且最后）在三个方言适配器里各自实现、互相不一致，直接污染上层 ReAct 状态（扫描证据见 research/implicit-state-machines.md，B5-B8/B20/B21；Lead 已复核 openai_responses.rs:400-420 状态结构）：

- **Responses**（openai_responses.rs:403）：done_emitted 仅用于 EOF 兜底；finish_responses_terminal 自身不检 done_emitted，第二个 response.completed 再发一次 Done；流中断时 function_calls 不 flush，已发 ToolCallStart 的调用永无 ToolCallEnd。
- **Completions**（openai.rs:237）：无 done_emitted，Done 在 choices 循环内发出，一个 chunk 含两个 choice 发两次 Done；EOF 无终态不报错（另两个实现报）。
- **Anthropic**（anthropic_messages.rs:370）：容忍 delta 早于 start 自建累积器、同 index 重复 start 覆盖已发内容，非法 SSE 序列被静默接受。

上层只能靠约定，缺一个流内状态机的唯一实现与共享校验。

## What Changes

- 引入共享的 XyChunk 流终态校验器（plugin-sdk 侧纯函数或测试用 validator），三个适配器统一：Done 恰一次且最后、每个 ToolCallStart 恰一个 ToolCallEnd、EOF 无终态报错、终态后不再产出。
- 三个适配器的流状态结构对齐（done_emitted + 终态错误内聚），异常中断时 flush 悬挂工具调用（对齐 anthropic 的 drain 行为）。
- 非法 SSE 序列（delta 无 start、重复 start）改为显式错误而非静默容忍。
- rider：provider trace finalize 后属性写入拦截（B20）；fake provider Cyclic 回绕跳过第 0 步（B21）。

## Capabilities

- infra-provider（方言装配与共享契约）；provider-openai* BDD 补终态矩阵；anthropic 方言在 provider-anthropic 插件内同步。

## Impact

- 范围：plugins/xylitol-official-plugin-provider-{openai,anthropic}、crates/xylitol-plugin-sdk（如 validator 落 sdk 词表层）。
- 行为变更：异常流从静默截断变为显式错误 → ReAct 侧错误处理路径（retry 判定）随之变化，需回归 is_retryable 分类。
- 验证：三方言终态矩阵单测（双 Done / 无 Done / 悬挂 ToolCall / 乱序 delta）；SSE 回放 fixture 回归。

## 决策记录（2026-11-11，全部已决）

1. （已决）校验器 = sdk 纯函数 chunk_stream_check（状态机）+ 三适配器共享测试宏；runtime 消费侧 debug_assert。
2. （已决）三适配器统一 StreamOutcome { chunks, terminal }；终态后零产出内聚。
3. （已决）中断 flush = 已开 ToolCall 补空参 ToolCallEnd + 流错误。
4. （已决）乱序（delta 无 start / 重复 start / 终态后事件）= 显式流错误。
5. （已决）Completions 的 Done 移出 choices 循环；EOF 无终态 = 错误。
6. （已决）源头正确性优先，不因下游可能过滤而降级；rider：trace finalize 属性拦截（两 crate）、fake Cyclic 回绕含第 0 步。

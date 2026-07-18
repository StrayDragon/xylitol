---
change_id: c1270-update-ai-bridge-responses-input-align-pi
title: Responses 请求体：system/developer 每轮注入 + thinking 不并入 output_text（对齐 pi）
status: full
priority: 1270
depends_on:
- c1260-update-app-tui-tool-stream-chrome
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: infra
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: false
---

# c1270-update-ai-bridge-responses-input-align-pi

## Why

1. 同 Ornith/llama.cpp + `enable_thinking:true` 时，pi 多轮原生 `toolCall` 稳定；xylitol 易把中文/XML 挤进 thinking、无 `function_call`、出现「Need go on」。
2. 对照 pi `openai-responses-shared`：system 每请求以 **`developer`/`system`** 角色注入；thinking 仅在有 `thinkingSignature` 时回放为 reasoning item——**绝不**并入 assistant `output_text`。
3. xylitol 现状：ReAct 把 system 塞成首条 **user**；Responses convert 用 `as_text()` 把 Thinking 并进 `output_text`——与 pi 协议面直接冲突。

## Purpose

1. `XyGenerateOptions` / `AiBridgeGenerateOptions` 携带 `system_prompt`；ReAct **不再**把 system 写入 `AgentMessage::user` history。
2. Responses `build_body`：有 system 时每请求前置 `{role: developer|system, content}`（thinking 开 → developer，否则 system；对齐 pi）。
3. Completions / Anthropic：经 options 注入正式 system 通道（Completions system message；Anthropic `system` 字段）。
4. Responses 历史回放：assistant **仅** `Text` → `output_text`；`Thinking` 无 signature 则省略；有 `thinkingSignature` 则 JSON 解析为 reasoning item 推入 input。

## What Changes

- `runtime_protocol::XyGenerateOptions` + bridge `AiBridgeGenerateOptions` + infra map
- `react.rs`：去掉 system-as-user；options 带 system
- `openai_responses`：prepend system；assistant collect 分 Text / Thinking
- `openai.rs` Completions：`convert_agent_messages(..., options.system_prompt)`
- `anthropic_messages`：`options.system_prompt` → body.system
- live specs：`pab15`（Responses input 对齐）；可选 `ar22`（system 不入 history user）
- 单测 + BDD 场景

## Capabilities

- `package-ai-bridge`（modify）
- `agent-runtime`（modify）

## Out of scope

- XML `<tool_call>` 核心抽取
- 完整 encrypted_content 捕获流水线（本 change 只：有 signature 则回放；无则省略）
- c1265 fastrace
- Completions 侧 thinking 通道大改

## Ethics

- risk_level: low
- prohibited_actions: 双写 Partitioned SSOT；引入 XML 抢救路径
- required_evidence: 单测证明 Responses input 含 developer/system 且 thinking 不进 output_text；ReAct 空会话 history 首条为真实 user
- escalation_policy: llama.cpp 拒 developer 时回退 system 并记文档

## Depends

- `c1260-update-app-tui-tool-stream-chrome`（流式 UX 已归档；本 change 修协议面根因）

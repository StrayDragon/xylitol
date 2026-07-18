---
change_id: c1250-update-ai-bridge-assistant-stream-events
title: "ai-bridge：助手流式内容块事件（对齐 pi AssistantMessageEvent）"
status: purpose-draft
priority: 1250
depends_on: []
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: packages
---

# c1250-update-ai-bridge-assistant-stream-events

## Why

1. 现有 `AiBridgeChunk` 只有 `TextDelta | ThinkingDelta | FunctionCall(完整) | Done`。工具参数在适配器内缓冲，**直到块结束**才吐出一次 `FunctionCall`。
2. 真机证据（session `t0718`，`XYLITOL_PROVIDER_TRACE=1`，`openai-responses` + `Ornith-1.0-35B-APEX`）：
   - 原生 `response.function_call_arguments.delta` **已在流**（单轮 47–70 次）；
   - mapped `FunctionCall` 仅在 `response.output_item.done` 出现；
   - 首个 args delta → 首个 mapped `FunctionCall` **滞后约 1.3–2.0s**。
3. 同 session **无** `<tool_call>` / XML 伪方言——本地模型在 Responses 路径上走的是原生 function_call。体验缺口是 **协议过粗**，不是缺 XML 抽取器。
4. pi（`packages/ai`）以 `toolcall_start|delta|end` + `parseStreamingJson` 为 SSOT；xylitol 要对齐该形状，才能让 agent/TUI 在首个可判别 chunk 起展示工具意图。

## Purpose

把 `xylitol-ai-bridge`（及主仓 `XyChunk` 映射）升级为 **助手流式内容块事件协议**（语义对齐 pi `AssistantMessageEvent`）：

1. text / thinking / toolcall 均具备 start/delta/end（或等价可组合事件）；
2. Completions / Responses / Anthropic 在收到首个工具相关 delta 时即发出 tool 意图事件；
3. 提供 `parse_streaming_json`（或等价）使 partial args 可渐进解析；
4. provider-trace 能对照 raw SSE 与 mapped 块事件（为后续 obs change 铺路）。

## What Changes

- 扩展或替换 `AiBridgeChunk`（推荐引入 `AiBridgeAssistantEvent` 流，映射层投影到 domain）
- `openai.rs`：`delta.tool_calls` 增量 → toolcall_*（禁止仅在 `finish_reason` 批量吐出）
- `openai_responses.rs`：`output_item.added`(function_call) → start；`function_call_arguments.delta` → delta；`output_item.done` → end
- `anthropic_messages.rs`：`tool_use` + `input_json_delta` 对齐同上
- `src/infra/provider/map.rs` + `XyChunk` / 新 domain 事件类型
- 包内 fixture 单测（录制或手工构造 SSE 片段）

## Capabilities

- `package-ai-bridge`（modify）
- `infra-provider`（modify，薄映射）
- 可能触及 `domain-*` 中 `XyChunk` 定义所在 capability（若拆出则注明）

## Out of scope

- Agent 意图/执行生命周期拆分 → **c1255**
- TUI 工具 chrome / 人类摘要 → **c1260**
- 包级 fastrace span 扩面 / 真机剧本闸 → **c1265**
- Cursor/XML 伪工具抽取器（**明确不做**；t0718 证明主路径是原生 tool API）
- 追平 pi 全部 provider（Bedrock/Google 等）

## Evidence (t0718)

| request_id (prefix) | fc_args deltas | FunctionCall mapped | lag (first delta → mapped) |
|---|---|---|---|
| `68c6ddb0` | 53 | 3× `ls` | ~1747ms |
| `04ef843e` | 47 | 2× `read` | ~1299ms |
| `b528b304` | 70 | `ls`+2×`read` | ~2029ms |

Session 落盘为结构化 `toolCall` parts，文本通道无 markup。

## Ethics

- risk_level: medium
- prohibited_actions: 在核心路径加入 XML/伪工具方言解析；把 Heuristic usage 标成 Api；破坏「包不依赖主 crate」边界
- required_evidence: Responses/Completions/Anthropic 各至少一条「首个 tool delta 即有 mapped 意图事件」单测；与 t0718 同类时序可复现
- escalation_policy: 若完整替换 `AiBridgeChunk` 导致主仓大面积破坏，升级确认是否分两步（先加事件旁路、再删旧 FunctionCall 整包变体）

## Depends

- []

## Downstream

```text
c1250-update-ai-bridge-assistant-stream-events
├── c1255-update-agent-tool-intent-lifecycle
│   └── c1260-update-app-tui-tool-stream-chrome
└── c1265-add-obs-fastrace-package-spans
```

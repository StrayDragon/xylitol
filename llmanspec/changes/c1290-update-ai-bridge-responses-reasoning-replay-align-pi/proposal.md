---
change_id: c1290-update-ai-bridge-responses-reasoning-replay-align-pi
title: "Responses：reasoning 捕获/回放 + strict/store + 工具 guidelines 装配（对齐 pi）"
status: purpose-draft
priority: 1290
depends_on:
  - c1270-update-ai-bridge-responses-input-align-pi
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: infra
---

# c1290-update-ai-bridge-responses-reasoning-replay-align-pi

## Why

1. eeee 等会话：多轮原生 `toolCall` 正常后，偶发「仅 `reasoning_text` + thinking 内伪 `<tool_call>`、无 native function_call」→ ReAct 停轮（与 pi **相同**停轮规则）。pi 同任务很少出现。
2. 对照 pi（`openai-responses.ts` / `openai-responses-shared.ts` / `agent-session._rebuildSystemPrompt`）：差异不在「无 tool 仍续跑 / 解析 XML」，而在 **请求与历史协议面**：
   - thinking 开时 `include: ["reasoning.encrypted_content"]` + `reasoning.summary`
   - 流式把 reasoning item 存进 `thinkingSignature`，下一轮原样回放
   - tools 显式 `strict: false`；常用 `store: false`
   - 系统提示装配 **汇总各工具 `promptGuidelines`**
3. c1270 已做 developer/system 注入与「无 signature 不并入 output_text」；**未**捕获/回放 encrypted reasoning，也未对齐 strict/store/guidelines——多步 + `enable_thinking` 时后几轮易漂。

## Purpose（延后；本文件仅 draft）

对齐 pi，**不做** XML 抢救、**不改**「无 native tool 则停」：

### A. Reasoning 生命周期（主）

1. Responses 请求：thinking 非 off 时注入 `reasoning.summary`（pi 默认 `auto`）与 `include: ["reasoning.encrypted_content"]`（兼容端忽略未知字段则文档化降级）。
2. 流式 / 非流式：在 reasoning `output_item.done`（或等价）把 **完整 reasoning item JSON** 写入 `AiBridgePart::Thinking.thinking_signature`（及 domain `AgentPart` 透传）。
3. 历史回放（c1270 已有钩子）：有 signature → 推入 input；无 → 省略（保持不并入 `output_text`）。

### B. 请求体其余字段

1. Responses tools：每项 `strict: false`（与 pi `convertResponsesTools` 默认一致）。
2. Responses body：`store: false`（与 pi 默认一致；若某兼容端拒绝再记例外）。

### C. 提示装配（结构对齐，非防呆文案）

1. 实现侧：从 `XyTool::prompt_guidelines()`（或 infra 工具定义）收集进 `SystemPromptOpts.prompt_guidelines`（`set_tools` / 初次装配）。
2. 为 bash/read/edit/write 等补齐与 pi 等价的 guideline 短句（内容对齐 pi tools，不是「禁止 XML」）。
3. 有 SYSTEM.md/`custom` 替换默认正文时：行为与 pi `customPrompt` 路径一致（不偷偷塞防呆）；可选后续再议「替换时是否仍附 Available tools」——**本 draft 默认跟 pi：custom 则整段替换**。

## What Changes（拟）

- `packages/xylitol-ai-bridge`：`openai_responses` build_body + stream map + signature 回放
- `domain` / infra map：Thinking signature 透传
- `src/agent/prompt` + `session`/`set_tools`：guidelines 收集
- `src/infra/tools/*`：补 `prompt_guidelines` / snippet 对齐
- live specs：`package-ai-bridge`（pab16+）、`agent-prompt` 或 `agent-runtime`（装配）
- 证据：单测 request JSON 含 include/strict/store；多轮 fixture 含 signature 回放；对照 pi 源码路径写入 design

## Capabilities

- `package-ai-bridge`（modify）
- `agent-prompt` 或 prompt 相关 capability（modify）
- 可能 `domain-message`（Thinking signature 字段若需契约）

## Out of scope

- 核心路径解析 `<tool_call>` XML / 自动 `go on`
- 改 ReAct「无 native tool 仍继续」
- c1280 安静 chrome（独立 draft）
- c1265 fastrace

## Ethics

- risk_level: medium（兼容端对 `include`/`store`/`summary` 行为不一）
- prohibited_actions: 双写 Partitioned SSOT；引入 XML 执行路径；默认把完整 reasoning 打进 stdout/TUI
- required_evidence: 单测锁定 body 字段；至少一条「有 signature 则 input 含 type=reasoning」；文档注明 llama.cpp/Ornith 拒字段时的降级
- escalation_policy: 某兼容端 4xx 则按 provider compat 开关省略 include/store，并升级确认

## Depends

- `c1270-update-ai-bridge-responses-input-align-pi`（developer/system + 不并入 output_text）
- 软依赖：真机 eeee 类剧本回归（人工）

## Status note

**purpose-draft / 延后**：先锁对齐范围；用户显式开闸后再 propose/apply。

---
change_id: c1290-update-ai-bridge-responses-reasoning-replay-align-pi
title: Responses：reasoning 捕获/回放 + strict/store + 工具 guidelines 装配（对齐 pi）
status: full
priority: 1290
depends_on:
- c1270-update-ai-bridge-responses-input-align-pi
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: infra
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: false
---

# c1290-update-ai-bridge-responses-reasoning-replay-align-pi

## Why

1. eeee 等会话：多轮原生 `toolCall` 正常后，偶发「仅 `reasoning_text` + thinking 内伪 `<tool_call>`、无 native function_call」→ ReAct 停轮（与 pi **相同**停轮规则）。pi 同任务很少出现。
2. 对照 pi（`openai-responses.ts` / `openai-responses-shared.ts` / coding-agent tools）：差异不在「无 tool 仍续跑 / 解析 XML」，而在 **请求与历史协议面**：
   - thinking 开时 `include: ["reasoning.encrypted_content"]` + `reasoning.summary`（默认 `auto`）
   - 流式在 `output_item.done`（reasoning）把完整 item JSON 写入 `thinkingSignature`，下一轮原样回放
   - tools 显式 `strict: false`；常用 `store: false`
   - 系统提示装配汇总各工具 `promptGuidelines`
3. c1270 已做 developer/system 注入与「有 signature 则回放 / 无则省略且不并入 output_text」；**未**捕获 signature，也未对齐 store/strict/summary/include/guidelines。Option 2 全链路证据：8/8 请求稳定缺口；session 有 Thinking、0 signature。

## Purpose

对齐 pi，**不做** XML 抢救、**不改**「无 native tool 则停」：

### A. Reasoning 生命周期（主）

1. Responses 请求：thinking 非 off 时注入 `reasoning.summary`（默认 `auto`）与 `include: ["reasoning.encrypted_content"]`（兼容端忽略未知字段则文档化降级）。
2. 流式 / 非流式：reasoning item 完成时把 **完整 item JSON** 写入 `Thinking.thinking_signature`（经新 chunk 透传到 ReAct / domain `AgentPart`）。
3. 历史回放：沿用 c1270——有 signature → 推入 input；无 → 省略（不并入 `output_text`）。

### B. 请求体其余字段

1. Responses tools：每项 `strict: false`（与 pi `convertResponsesTools` 默认一致）。
2. Responses body：`store: false`。

### C. 提示装配（结构对齐，非防呆文案）

1. `set_tools` / 初次装配：从 `XyTool::prompt_guidelines()` 收集进 `SystemPromptOpts.prompt_guidelines`。
2. 为 bash/read/edit/write 等补齐与 pi 等价的 guideline 短句。
3. SYSTEM.md / `custom` 替换默认正文时：跟 pi `customPrompt`——整段替换，不偷偷塞防呆。

## What Changes

- `packages/xylitol-ai-bridge`：`build_body`（store/strict/summary/include）；stream/non-stream 捕获 reasoning → signature；必要时新增 `ThinkingEnd`（或等价）chunk
- `domain` `XyChunk` + infra map；`react`：把 signature 写入最终 `AgentPart::Thinking`
- `src/agent/prompt` + `session::set_tools`：guidelines 收集
- `src/infra/tools/*`：补 `prompt_guidelines`
- live specs：`pab16`；`pt9`
- 单测 + BDD；design 对照 pi 路径

## Capabilities

- `package-ai-bridge`（modify）
- `agent-prompt`（modify）
- `agent-runtime`（modify，signature 入 history；若仅 chunk 透传可不增 req）

## Out of scope

- 核心路径解析 `<tool_call>` XML / 自动 `go on`
- 改 ReAct「无 native tool 仍继续」
- c1280 安静 chrome；c1265 fastrace
- 默认把完整 reasoning / encrypted_content 打进 stdout/TUI

## Ethics

- risk_level: medium（兼容端对 `include`/`store`/`summary` 行为不一）
- prohibited_actions: 双写 Partitioned SSOT；引入 XML 执行路径；默认把完整 reasoning 打进 stdout/TUI
- required_evidence: 单测锁定 body 字段；有 signature 则 input 含 `type=reasoning`；文档注明拒字段时的降级
- escalation_policy: 某兼容端 4xx 则按 provider compat 开关省略 include/store，并升级确认

## Depends

- `c1270-update-ai-bridge-responses-input-align-pi`（已归档）

## Evidence（升 full 前，2026-07-18）

产物：`/tmp/xylitol-c1290-evidence/`（不入库）。Option 2：`--print` + hooks 抓 8 轮 POST——`developer` 有；store/strict/summary/include **8/8 gap**；req-01…07 `input_reasoning_items=0`、session Thinking 无 signature。Ornith probe：补 pi 字段接受且 SSE 回 `encrypted_content`。详见 draft 期 `OPTION2-REPORT.md`。

---
change_id: c1310-update-infra-tool-result-quiet-align-pi
title: infra：write/edit tool result 对齐 pi（模型短成功句；diff 不进 LLM）
status: full
priority: 1310
depends_on: []
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: infra
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: true
checkpoint_sha: 543568701581015a48f5fcecfca0b73bedda83cd
---

# c1310-update-infra-tool-result-quiet-align-pi

## Why

1. pi：write/edit 的 **tool result → LLM** 仅为短成功句；`details.diff` 只服务 UI/session，不进 provider。
2. xylitol：edit `execute` 返回含 `diff`/`display_diff` 的整包 JSON 进入 history → `project_for_llm` → **重复烧 token**（调用参数里已有 edits）。
3. bash/read 截断 + 临时文件路径已基本对齐；本 change 收 write/edit result 面。

## Purpose

1. **write** 成功：LLM 可见结果 MUST 为短文本（如 `Successfully wrote N bytes to {path}` 或等价）；MUST NOT 依赖大 content 回灌。
2. **edit** 成功：LLM 可见结果 MUST 为短文本（如 `Successfully replaced N block(s) in {path}.`）；unified/`display_diff` MUST NOT 进入发往模型的 tool result 正文。
3. **TUI**：仍能取得 `display_diff`（经 ToolExecutionEnd / details / 并行通道）；与 c1300 合块消费对齐。
4. **bash/read**：保持既有 2000 行/50KB + spill；本 change 不改阈值，仅在 specs 钉「与 pi 同语义」若缺漏则补。

## What Changes

- `src/infra/tools/{write,edit}.rs`：成功返回短文本；diff 经可给 UI 的通道保留
- 若需：`runtime_protocol` / ReAct 区分 `content` vs UI details；或 End 事件带 details、history 只存短句
- live specs：`agent-tools` 修改 t6 + 新增 quiet-result req；BDD 场景更新
- 单测：edit 成功 history/project_for_llm 不含 display_diff 墙

## Capabilities

- `agent-tools`（modify）

## Out of scope

- 产品 scrollback 合块 / write viewport → **c1300**
- 改 bash/read 数值阈值
- MCP 工具统一 quiet 策略（可另议）

## Ethics

- risk_level: medium（改 tool_result 合约与 BDD）
- prohibited_actions: 让模型看不到失败原因；删除 diff 却不给 TUI 通道
- required_evidence: 单测/BDD — 成功 edit 的 LLM 投影无 display_diff；TUI 仍能渲染 diff
- escalation_policy: 与 c1300 并行；先定「UI 如何取 diff」再改 execute 返回形

## Depends

- 无硬依赖；与 c1300 同 wave，建议设计评审一次通道形状

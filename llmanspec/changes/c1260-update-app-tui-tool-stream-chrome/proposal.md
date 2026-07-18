---
change_id: c1260-update-app-tui-tool-stream-chrome
title: app-tui：工具意图流式 chrome + 人类可读摘要（对齐 pi coding-agent）
status: full
priority: 1260
depends_on:
- c1255-update-agent-tool-intent-lifecycle
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: app
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: false
---

# c1260-update-app-tui-tool-stream-chrome

## Why

1. pi：`AssistantMessageComponent` **只渲染 text/thinking**；`ToolExecutionComponent` 旁路，在 `message_update` 见到 `toolCall` 即挂载并用 `updateArgs` 刷新。
2. xylitol TUI：工具块主要在 `ToolExecutionStart` 出现；`args_preview` 为截断 JSON；roadmap「工具可读化」未兑现。
3. c1255 之后事件已具备意图流式；本 change 把 **用户可感知** 体验对齐到 pi，并兑现 `docs/roadmaps/TUI视觉与信息表达.md` M1。

## Purpose

固定产品行为：

1. 助手区永不把 toolCall 当 Markdown 正文渲染；
2. 首个 tool 意图事件 → 出现工具块（可缺完整 args）；
3. args 流式更新；默认摘要人类可读（bash → `$ cmd`；read/ls → 路径短形）；展开仍可看完整参数/输出；
4. 执行期：pending/success/error 视觉状态；输出可随 `ToolExecutionUpdate` 增长；
5. flush 规则：不得因工具开始而把「本应属于 tool 的内容」误冻进 Assistant 文本（原生路径下）。

## What Changes

- `src/app/tui/bridge`：`MessageUpdate` 消费 ToolCall → upsert Tool；`human_tool_args_preview`
- `ToolExecutionStart` upsert 同一行；session_tree 重建共用摘要
- live specs：`app-tui-bridge` atb10；`app-tui-transcript` att13
- bridge 单测：意图先于执行、args 流式、bash `$` 摘要

## Capabilities

- `app-tui-bridge`（modify）
- `app-tui-transcript`（modify）

## Out of scope

- XML 伪工具「抢救」显示
- xylitol-tui 引擎级大改
- fastrace → **c1265**
- 完整主题密度 M4

## Ethics

- risk_level: low
- prohibited_actions: 默认倾倒完整 JSON 墙；reach `agent/`/`infra/` 内部；在假树 stub 上扩活树
- required_evidence: bridge 单测证明意图帧先于完整执行；bash 摘要非纯 JSON
- escalation_policy: 摘要表放 TUI 本地（本 change）

## Depends

- `c1255-update-agent-tool-intent-lifecycle`（已归档）

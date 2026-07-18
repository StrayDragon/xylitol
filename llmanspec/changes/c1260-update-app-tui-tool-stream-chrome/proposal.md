---
change_id: c1260-update-app-tui-tool-stream-chrome
title: "app-tui：工具意图流式 chrome + 人类可读摘要（对齐 pi coding-agent）"
status: purpose-draft
priority: 1260
depends_on:
  - c1255-update-agent-tool-intent-lifecycle
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: app
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

- `src/app/tui/bridge`：消费 MessageUpdate 中的 tool 块；与 ToolExecution* 协同
- `widgets/scrollback`（及/或独立 tool widget）：旁路工具行 + 摘要渲染
- 内置工具摘要表（bash / read / edit / ls 等最小集）
- 快照 / 组件测试；必要时 BDD 应用面场景

## Capabilities

- `app-tui-bridge` / `app-tui-transcript` / `app-tui-chrome`（按实际落点 modify）
- 可能轻触 `agent-tools` 若摘要挂在工具元数据

## Out of scope

- XML 伪工具「抢救」显示
- xylitol-tui 引擎级大改（除非现有组件不够用的最小扩展）
- fastrace / 真机闸全文 → **c1265**
- 完整主题密度 M4

## Ethics

- risk_level: low
- prohibited_actions: 默认倾倒完整 JSON 墙；reach `agent/`/`infra/` 内部；在假树 stub 上扩活树
- required_evidence: 组件/桥接测试证明「意图帧先于完整 args」；bash 摘要非纯 JSON；Assistant 渲染路径跳过 toolCall
- escalation_policy: 若摘要表放 tool 定义 vs TUI 本地表有争议，升级确认 SSOT 位置

## Depends

- `c1255-update-agent-tool-intent-lifecycle`

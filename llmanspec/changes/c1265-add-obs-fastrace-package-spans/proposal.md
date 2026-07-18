---
change_id: c1265-add-obs-fastrace-package-spans
title: "观测：跨包 fastrace span + 真机流式工具剧本"
status: purpose-draft
priority: 1265
depends_on:
  - c1250-update-ai-bridge-assistant-stream-events
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: infra
---

# c1265-add-obs-fastrace-package-spans

## Why

1. 今日 fastrace 主战场在 `provider.request` + `provider-trace.jsonl`；app-tui / agent / xylitol-tui 缺少可关联的父子 span，排障仍靠猜。
2. t0718 证明：用 trace 可在 **分钟级** 判定「上游已流式 function_call_arguments.delta，mapped 过晚」——应把这类对照变成稳定、可脚本化的能力。
3. 真机剧本（hi → 工具 → 读文件）应可在开发者机器一键跑，默认 CI 不碰密钥。

## Purpose

1. 为关键包/层安装 **零成本可关** 的 fastrace span（命名稳定、可父子关联）：
   - `xylitol-ai-bridge`：`provider.request`（已有）+ `adapter.map_event`
   - `agent`：`react.turn` / `react.stream` / `tool.execute`
   - `app/tui`：`tui.host.event` / `tui.bridge.apply`（采样策略避免热路径爆炸）
   - 可选 `xylitol-tui`：`engine.frame`（低频或 debug-only）
2. 文档化：如何用 `provider-trace` + span 判定「无 tool_calls」vs「映射滞后」（引用 t0718 模式）。
3. 可选 `just` 目标 / 脚本：`XYLITOL_LIVE_MODEL=1` 时跑最小真机剧本并检查「mapped 工具意图不晚于首个 args delta 之后 N 事件」类断言（在 c1250 落地后有意义；完整 UI 断言可等 c1260）。

## What Changes

- span 埋点（遵守 ipt3：仅 fastrace + log）
- `infra-provider-trace` / 新 `infra-observability` 或文档 capability 增补
- 真机脚本（opt-in）+ AGENTS/skill 指针更新（`xylitol-inspect-runtime-logs`）

## Capabilities

- `infra-provider-trace`（modify）
- 可能 `package-ai-bridge`（span 点）
- `test-*`（live harness，若新增）

## Out of scope

- 引入 `tracing` crate（禁止）
- 默认 CI 调用付费/私有模型
- XML 抽取器

## Ethics

- risk_level: low
- prohibited_actions: span/log 写入 Authorization；stdout/stderr 打 trace 毁 TUI；release 默认昂贵序列化
- required_evidence: 关闸时零/近零开销测试或文档化证明；开闸时一次请求可见跨层 span / 同一 request_id
- escalation_policy: 热路径 span 若导致可测延迟回退，改为采样并升级确认

## Depends

- `c1250-update-ai-bridge-assistant-stream-events`（事件名与 map span 对齐）
- 完整「TUI 无裸块」真机断言 **软依赖** c1260（tasks 分阶段）

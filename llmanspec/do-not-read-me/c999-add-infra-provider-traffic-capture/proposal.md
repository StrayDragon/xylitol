---
change_id: c999-add-infra-provider-traffic-capture
title: "Provider 流量抓包 / 即时查看（外挂优先）"
status: paused
priority: 999
depends_on: []
author: agent
track: B
wave: observability-capture
paused_reason: "产品意向改为进程内、agent 可分析的 raw provider trace（debug 默认 / release 开关）；外挂 MITM/claude-tap 路径暂不推进"
paused_date: 2026-07-15
paused_location: llmanspec/do-not-read-me/c999-add-infra-provider-traffic-capture
---

# c999-add-infra-provider-traffic-capture

> **⏸️ PAUSED（2026-07-15）** — 外挂抓包方案与「agent 友好可分析 / 区分 provider vs 映射 bug」目标不符。
> 本目录已移出 `llmanspec/changes/`：`llmanspec/do-not-read-me/c999-add-infra-provider-traffic-capture/`。
> **后续主线不读此提案**；若恢复外挂路径，再移回 `changes/` 并重开 decision log。
> 替代方向（另开 change）：进程内 raw SSE + 映射后 `XyChunk` 对照落盘；`cfg(debug_assertions)` 默认开，release 经 env/settings 开关。

## Why（历史）

会话 JSONL / `XyChunk` 是归一化结果，无法单独证明网关推了 `reasoning_text` 还是 `output_text`。

## Purpose（历史草案 — 外挂优先）

见正文归档意图：mitmproxy / claude-tap PoC。**已否决为当前主路径。**

## Next（若重开）

1. 确认仍要外挂，而非进程内 trace。
2. 移回 `llmanspec/changes/`，更新 status，再 PoC / full。

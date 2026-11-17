---
depends_on: []
blocks: []
title: 工具门被动轮询与 MCP 视图读稳定
needs_specs_change: false
branch: sdd/c3347-fix-tool-gate-passive-poll
base_branch: main
base_sha: 286c34dd7754d8cc83dc1a9df8f65c0da6727309
---

# 工具门被动轮询与 MCP 视图读稳定

## Why

c3345 交付后真机复现两个残余症状:

1. **准备期提交后 tray 在 connecting↔connected 间抖动、右下角 mcp cue 消失、Assembling 异常**:c3345 的 `poll_tool_gate` 用 `hub_wait(100ms)` 分段等待,而 `McpPrepareHub::wait` = `timeout(window, self.ensure())`——每 50ms 轮询把整条 MCP prepare 管线重跑(连接打断重建、进度重置),且每次窗口超时写一次 `SESSION_PREPARE_TIMEOUT_NOTICE` 污染 cue 与 Assembling。
2. **空闲(不提交)时 tray 单帧闪回 `configured · 0 connected`**:端侧 `Command::LoadedResources` unary 不带 session_id,host 走「遍历 slot 取首个有 driver」兜底;历史 TUI 残留 slot 的 hub 永远未完成,其未完成视图被推给当前 TUI,与带 session 的刷新互相覆盖。

## What Changes

- `poll_tool_gate` 改纯被动:每轮只读 `hub.view()`(`configured == 0 || complete` 即 settle),**不得**调用 `hub.wait` / `ensure`;门限耗尽才一次性回落 `armed_result` 并写一次门限通知。
- bootstrap 本身由 materialize 时的 `begin_session_prepare`(`hub.fire()`)自启,被动轮询只观察进度,不驱动。
- 端侧 `loaded_resources_snapshot` 的 unary payload 经 `with_session` 注入 `session_id`(host 侧本就可选读取,零协议变更),使刷新始终命中本会话 slot。
- 诊断日志:`ensure_for` / `update_snapshot` / serve watch push / 端侧 downlink apply 各一行 debug 日志(本次定位依赖,保留为诊断面)。
- 无合约变更(`server-core` r1923 的「不持锁等待、有界收口、下行回推」契约不变);`needs_specs_change: false`。

## Impact

- crates:`xylitol-runtime`(prepare.rs 轮询 + 单测)、`xylitol-app-tui`(remote.rs payload 注入 + 诊断日志)、`xylitol-official-plugin-mcp`、`xylitol-serve`(诊断日志)。
- 复验:tmux 真机——全新 host 空闲 50s tray 仅启动期一次合法迁移零抖动;准备期提交全流程(Assembling + Follow-up 条 + cue 常驻 → 冻结后自动上行 → 流式)。

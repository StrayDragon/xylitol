---
depends_on: []
title: 修复 MCP pending cue 在冻结后残留
branch: sdd/c3221-fix-mcp-pending-cue-refresh-after-freeze-and-gate-timeout
base_branch: main
base_sha: 545361789b221ccba78f3ebed9a2393c8e459784
---

## Why

tmux 复现：配置一个 MCP server 后，首个 turn 已结束、`/mcp` 面板显示 `connected 1 · armed 1`，但 header 仍是 `mcp: connecting 0/1`，status 右侧仍显示 `mcp pending (see /mcp)`。挂死 server 时 cue 也会在对话结束后继续存在。现有 `refresh_mcp_short_cue` 依赖 loaded-resources 快照；该快照在 freeze / arm 后没有及时刷新，`mcp_tools_pending` 仍看到旧的 `connecting_label`。

## What Changes

- 在 tool freeze、MCP arm、gate timeout、reload、resume、切会话后刷新 loaded-resources 快照并重算 cue；不得只在 turn 前刷新。
- gate timeout 须收口为 settled 快照：标记 bootstrap complete / tools frozen（或等价状态），清 `mcp pending` cue；诊断留在 `/mcp`。
- connected / armed 后清掉 `connecting_label`；header、status cue、`/mcp` 面板读取同一份状态，不得一个显示 connected/armed、另一个显示 connecting。
- 挂死 stdio server 的 `connect_timeout_ms` 须真正生效；超时后收口为 Failed / timed-out，不得无限 `connecting`。
- 测试：pending → armed → cue 清除；gate timeout → cue 清除；hanging server → 超时收口；reload / resume 不粘旧 cue；tmux 真终端场景。

## Capabilities

- `app-tui-host`
- `infra-mcp`
- `agent-runtime`

## Impact

- TUI effects / host session ops / layout root cue。
- runtime `join_session_prepare` timeout 收口与 `SessionPrepareView`。
- MCP hub connect / initialize 超时与 snapshot。

## Further Notes

tmux 证据（fixture）：`mcp: connecting 0/1` + `mcp pending (see /mcp)` 与 `/mcp` 的 `connected 1 · armed 1` 同时存在。挂死 server 配置 `connect_timeout_ms: 2000`，40s 后仍显示 connecting。

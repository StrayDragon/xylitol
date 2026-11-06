---
depends_on:
  - c3075-add-mcp-diagnostics
title: "Add MCP capability probe and reconnect count"
---

## Why

c3075 已交付每 server 诊断（transport / scope / credential presence / counts / timeouts / last error），但 capability probe（`server/discover` 现代路径、legacy `initialize` + `tools/list`）与重连次数未实现。诊断只能看当前连接结果，不能主动探测能力或观察重连稳定性。

## What Changes

- capability probe：优先 `server/discover`；不支持时回退 legacy `initialize` + `tools/list`。
- probe 结果按 config fingerprint 缓存；只用于诊断，不判状态性。
- 重连次数：pool 记录每 connection key 的 connect 尝试 / 成功 / 失败计数。
- `/mcp` 面板显示 probe 结果、timeout 类别、重连次数。
- 错误分类：connect timeout / call timeout / legacy SSE / auth / protocol。

## Scope

- probe 不改变连接建立路径；不自动重试。
- 不做 statefulness 自动探测。
- 不实现新的 transport。

## 延后原因

- `server/discover` 在 rmcp 3.8 的可用性未验证；probe 时机（启动 / 首次使用 / 手动）与缓存 TTL 未定。
- 重连计数需要 pool 状态扩展；主线先交付静态诊断。
- probe 可能触发额外网络请求，需先明确成本与隐私边界。

## 启动前置条件

- c3075 已归档（当前已满足）。
- 验证 rmcp 3.8 是否支持 `server/discover` 与现代 capability 协商。
- 确定 probe 时机、缓存 TTL 与手动触发入口。
- 设计 pool 的重连计数与重置条件。
- 确定错误分类的稳定词表。

## Related

- c3075-add-mcp-diagnostics（已归档）
- c3085-add-mcp-transport-config（transport / timeout）

## Open Questions

- probe 是否在每次连接后自动执行。
- 缓存 TTL 与 config fingerprint 变化的关系。
- legacy 回退的判定条件。
- 重连计数是否按 session / workspace 分开。

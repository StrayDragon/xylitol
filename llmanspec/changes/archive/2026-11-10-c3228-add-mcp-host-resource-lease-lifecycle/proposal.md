---
depends_on: []
title: "MCP 连接作为 host 资源租约"
needs_specs_change: true
branch: sdd/c3228-add-mcp-host-resource-lease-lifecycle
base_branch: main
base_sha: 07e7f8a1339162f13bfcb0381a180121dab0ea84
---

# MCP 连接作为 host 资源租约

## Why

MCP 连接是 host 资源，不是 client 私有连接。现状已有 connection key、claims 引用计数与 `release_session`，但生命周期没有闭环：host 驱逐 session 时直接丢弃 driver，旧 claims 不释放；旧 workspace 的 MCP 连接会留在 pool，直到 host 退出。新 session 的快照又会聚合整个 pool，导致卡片出现重复连接。

用户目标：MCP 连接按引用计数删除；同 key 可复用；有状态 server 不被错误复用；client clean detach 立即释放，崩溃由 host 兜底。

## 现状证据

- 引用计数规则已写入 [infra-mcp.feature](../../llmanspec/specs/infra-mcp/infra-mcp.feature:94)：同 key 共用一个连接，懒连接 + 引用计数，最后一个 claim 释放才关闭。
- [hub.rs](../../plugins/xylitol-official-plugin-mcp/src/hub.rs:682) 已有 `release_session` / `release_claims`；切 session 时会调用（[session.rs](../../crates/xylitol-runtime/src/driver/in_process/session.rs:42)）。
- [host.rs](../../crates/xylitol-serve/src/occupancy/host.rs:197) 的 `evict_slot` 直接 `*slot.driver.lock().await = None`，没有释放 MCP claims。
- [host.rs](../../crates/xylitol-serve/src/occupancy/host.rs:1365) 已有 last-subscriber / linger 驱逐路径，可作为崩溃兜底。
- [loaded_resources.rs](../../crates/xylitol-app-tui/src/widgets/loaded_resources.rs:49) 的卡片直接聚合 `mcp_connected`；c3226 分支已临时改为按当前 session claims 过滤。
- 临时 workaround：`evict_slot` 调 `hub.release_session(slot.session_id)`；clean detach 立即释放，崩溃走 linger。它不是最终方案，后续由本 change 替换。

## What Changes

### Host 资源租约

- host 是租约所有者；新增通用 `ResourceKey` / `ResourceScope` / `ResourcePool` / `LeaseState`，MCP registry 使用 `ResourcePool<ResourceKey, PoolEntry>`；`PoolEntry` 记录 ref_count / last_used / idle_generation。
- session attach 获取 claim，clean detach / session evict 释放 claim；最后一个 claim 释放后按 `keep_alive_ms + idle_ttl_ms` 关闭连接。
- LSP / DAP 后续可直接复用同一租约语义。

### Session 生命周期

- 新增产品 unary `session_detach`；TUI clean exit 发送该 unary，host 在最后 client 且非 busy 时释放 session 租约。
- 崩溃 / detach 失败由 host 的 last-subscriber / linger 兜底；MCP 规范没有专用 shutdown 消息，关闭 transport 即终止信号。
- 同一 session 多 client attach 时 claims 保持；最后一个 detach 才释放。

### Scope 与状态

- `Shared`：跨 session 复用，适合无状态 server。
- `Workspace`：同 workspace 跨 session 复用；缺省。
- `Session`：session 结束即关，忽略 idle grace，适合有状态 server。
- 新增 `keep_alive_ms` / `idle_ttl_ms` 配置；有状态 server 显式设 `scope: session`。
- 不跨 workspace 复用；不把旧 workspace 连接显示到新 session 快照。

### 观测与测试

- 记录 pool entry、claims、last_used、reconnect；提供 host 诊断输出。
- 测试：同 workspace 复用、跨 workspace 隔离、clean detach 释放、崩溃 TTL 释放、stateful scope 关闭、最后一个 claim 关闭。

### Specs

- 更新 `infra-mcp`：资源租约、scope 策略、clean detach / crash 兜底、快照按 session 过滤。
- 更新 `server-core`：host-resource-lease 通用租约语义。
- 更新 `protocol-app`：`session_detach` unary 与 `Command::SessionDetach`。
- 单测覆盖租约状态机与 detach；端到端 BDD 覆盖 clean detach 与崩溃兜底。

## 非目标

- 本 change 不改 MCP 协议本身；不要求 MCP server 提供 shutdown 消息。
- 不在本 change 内实现 LSP / DAP 租约；只预留通用模型。
- 不跨 workspace 复用 MCP 连接；不做全局无界连接池。
- 不把有状态 server 默认设为跨 client 复用。

## 验收方向

- clean detach 后，最后一个 claim 释放并关闭连接；新 session 卡片只显示当前 session 的连接。
- 崩溃 / 无 clean detach 时，linger/TTL 到期后释放；不无限保留。
- 同 workspace 新 client 可复用连接；跨 workspace 隔离。
- 有状态 server 配 `scope: session` 后，session 结束即关。
- 临时 workaround 删除，由租约模型替代。

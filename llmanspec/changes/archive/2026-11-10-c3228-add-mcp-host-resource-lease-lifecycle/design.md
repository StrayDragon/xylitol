# Design — MCP 连接作为 host 资源租约

## 1. 目标与边界

- host 拥有 MCP 连接池；session 只持有 lease，不拥有 transport。
- 引用计数删除：同 key 共用一个连接，最后一个 lease 释放后按 `idle_ttl` 关闭。
- clean detach 立即释放；崩溃由 last-subscriber + linger/TTL 兜底。
- Scope 决定复用边界：Shared 跨 session，Workspace 同 workspace，Session 单 session。
- 不跨 workspace 复用；不把旧 workspace 连接显示到新 session 快照。
- 有状态 MCP 默认不跨 client 复用；显式 `scope: session`。

## 2. 决策摘要

| 决策 | 内容 |
|---|---|
| 资源模型 | 通用 `ResourceKey` / `ResourcePool` / `LeaseState`；MCP registry 用 `ResourcePool<ResourceKey, PoolEntry>` |
| 生命周期 | session attach → claim；clean detach / evict → release；最后一个 release → keep_alive + idle_ttl → close |
| Clean detach | TUI 发 `session_detach`；host 在最后 client 且非 busy 时释放；失败由 WS / mux / linger 兜底 |
| 崩溃兜底 | last-subscriber gone → linger → release；host 退出全部关闭 |
| Scope | Shared / Workspace / Session；缺省 Workspace |
| 配置 | `keep_alive_ms` / `idle_ttl_ms` / `scope`；有状态 server 显式 Session |
| 快照 | 只聚合当前 session 的 claims keys；跳过 idle entry |
| 观测 | pool entry / claims / last_used / reconnect |
| 临时补丁 | 删除 `TODO(c3228)`；`release_session` 作为最终释放入口保留 |

## 3. 资源模型

通用租约类型在 `xylitol-plugin-sdk::resource_lease`；MCP registry 直接使用：

~~~text
ResourceKey { kind, fingerprint, scope, credential, workspace, session }
ResourcePool<K, V> { entries, next_generation }
LeaseState { ref_count, idle_generation, idle_delay, touch }

ConnectionKey                 = ResourceKey (kind = Mcp)
claims[session_id]            = 该 session 持有的 lease set
ResourcePool<ResourceKey, PoolEntry> = MCP registry
PoolEntry.claims              = ref_count
PoolEntry.last_used           = 最近 acquire / release 时间
PoolEntry.idle_generation     = 失效令牌，用于取消未到期的 idle close
PoolEntry.config              = keep_alive_ms / idle_ttl_ms / scope
~~~

- `ResourceKey` / `ResourcePool` 对 LSP / DAP 开放；本 change 先落 MCP 实现。
- `ResourcePool` 只由 host 侧 hub 修改；client 不直接操作。
- key 不跨 workspace 合并。

## 4. 生命周期状态机

~~~text
session attach
  │
  ▼
claim acquired (entry.claims += 1, idle_generation += 1)
  │
  ├─ clean detach / session evict → release (entry.claims -= 1)
  │       └─ claims == 0 → keep_alive_ms + idle_ttl_ms → close transport
  │               └─ new claim during grace → reuse entry, cancel expiry
  │
  └─ crash / no clean detach
          └─ last-subscriber gone → linger → release → idle grace → close
~~~

- clean detach 由 host 的 last-subscriber / evict 触发；崩溃走 linger 兜底。
- 同一 session 多 client：claims 保持，最后一个 client detach 才 release。
- host 退出：drop 所有 transport。

## 5. 与 MCP hub 的关系

- `McpPrepareHub` 用 `ResourcePool<ResourceKey, PoolEntry>` 保存连接与 lease 元数据；`claims` 记录 session 持有的 key。
- host 通过 `release_session` 释放；`session_detach` 与 `evict_slot` 都是释放入口。
- `view_now` 只聚合当前 session 的 claims keys，并跳过 `claims == 0` 的 idle entry。
- `Shared` key 跨 session 共用 entry；`Workspace` key 带 workspace；`Session` key 带 session。
- 有状态 server 的 `scope: session` 保证 session 结束即关。

## 6. Session 生命周期

- 新增产品 unary `session_detach`，参数 `session_id`；TUI clean exit 在关闭 WS 前发送。
- host 在最后 client 且无 run inflight 时释放 session 租约；仍有其他 client 或 busy 时不释放。
- 失败 / 崩溃时由 WS / mux 关闭与 linger 兜底；MCP transport close 仍是最终关闭信号。
- 不新增 MCP 协议 shutdown 消息；host 是关闭动作的唯一权威。

## 7. 配置

~~~yaml
mcp_servers:
  - name: context7
    scope: workspace
    keep_alive_ms: 30000
    idle_ttl_ms: 300000
  - name: browser
    scope: session
    keep_alive_ms: 0
    idle_ttl_ms: 0
~~~

- `keep_alive_ms`：最后一个 claim 释放后的复用窗口。
- `idle_ttl_ms`：复用窗口到期后的额外 idle 时间；两者相加为总保留时间。
- 两者都为 0 时立即关闭；新 claim 在保留期内复用未关闭连接。
- `scope`：Shared / Workspace / Session；缺省 Workspace。
- Session scope 结束即关，忽略 keep_alive_ms / idle_ttl_ms。
- 有状态 server 必须显式 Session；文档给出示例。

## 8. 快照与观测

- `LoadedResourcesSnapshot.mcp_connected` 只来自当前 session lease keys。
- host 诊断输出：pool entry、claims、last_used、reconnect_count、scope。
- 日志：lease acquire/release、idle close、crash linger。
- 指标：pool size、claims、idle closes、reconnects。

## 9. 测试矩阵

| 场景 | 期望 |
|---|---|
| 通用 ResourcePool | acquire / release / idle generation 单测 |
| 同 workspace 两 session | 复用同一 entry；ref_count=2 |
| 跨 workspace 两 session | 两个 entry；互不显示 |
| clean detach | ref_count 减一；最后归零后 idle grace → 关闭 |
| session_detach | 最后 client 释放；多 client / busy 不释放 |
| idle grace 内新 claim | 复用未关闭 entry；过期任务失效 |
| 崩溃 / linger | linger 到期释放；不无限保留 |
| 多 client 同 session | 最后一个 detach 才释放 |
| stateful scope=session | session 结束即关 |
| host 退出 | 全部 transport 关闭 |
| 快照 | 只显示当前 session 连接；跳过 idle entry |

## 10. 风险与迁移

- 有状态 MCP 可能把状态绑在连接上；默认 Workspace 复用有风险。必须提供 Session scope 与文档。
- 并发 client 共享一个 MCP session 可能触发 server 并发限制；host 需串行化或按 scope 隔离。
- OAuth / credential 绑定 key；跨 workspace 不得复用 credential。
- 崩溃兜底依赖 last-subscriber；host 需保证 mux 生命周期可靠。
- 迁移：c3226 的 `evict_slot` 释放入口保留；删除 `TODO(c3228)`，idle grace / scope / last_used 由 hub 租约 registry 承担。
- 通用 ResourceKey / ResourcePool 已落；LSP/DAP 接线留待后续。

## 11. 调研结论

- MCP 规范：shutdown 无专用消息，transport close 即终止；生命周期是 initialize → operation → shutdown。来源：<https://modelcontextprotocol.io/specification/2025-03-26/basic/lifecycle>。
- 生产 MCP server：session-per-connection、SessionPool、evictStale；状态恢复需要外部持久化。来源：<https://www.sitepoint.com/building-production-mcp-servers-in-typescript/>。
- 有状态 MCP 的复用通常在同一会话内；跨 client 复用需谨慎。来源：<https://www.npmjs.com/package/n8n-nodes-mcp-session/v/0.1.30>。
- LSP 长驻资源用 idle timeout 收口，避免无限保留。来源：<https://github.com/NousResearch/hermes-agent/pull/64980>。
- 重连可重放 initialize handshake 并用稳定 session ID 续接；说明 stateful 连接需要显式生命周期。来源：<https://pkg.go.dev/github.com/plumbkit/plumb@v0.16.0/internal/sessionstate>。

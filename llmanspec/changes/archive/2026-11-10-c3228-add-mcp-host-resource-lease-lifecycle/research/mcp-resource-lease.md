# Research — MCP 连接生命周期与复用

## 问题

- MCP 连接是 host 资源还是 client 私有连接？
- 旧连接能否复用于下一个 client？
- 有状态 MCP 是否应跨 client 复用？
- 旧 TUI 断开前是否应发事件让 MCP 结束？
- 更根本的解法是什么？

## 一手资料

### MCP 规范

- 生命周期：initialize → operation → shutdown。
- shutdown 没有专用消息；由底层 transport 关闭来终止连接。
- 来源：<https://modelcontextprotocol.io/specification/2025-03-26/basic/lifecycle>。

结论：关闭动作应由持有 transport 的 host 执行；client 不需要发 MCP shutdown 消息。

### 生产 MCP server 的 session 管理

- session-per-connection；SessionPool 管理并发会话；evictStale 清理过期会话。
- 状态恢复需要外部持久化；断开时用 AbortController 取消执行。
- 来源：<https://www.sitepoint.com/building-production-mcp-servers-in-typescript/>。

结论：连接池 + 过期淘汰是成熟做法；状态恢复不能只靠连接复用。

### 有状态 MCP 的复用

- 连接池可复用有状态 server 的多次工具调用，但复用范围通常是同一会话。
- 来源：<https://www.npmjs.com/package/n8n-nodes-mcp-session/v/0.1.30>。

结论：跨 client 复用有状态 server 需要显式策略；默认不应跨 session。

### LSP 长驻资源

- 长驻 server 用 bounded idle timeout 收口，避免无限保留。
- 来源：<https://github.com/NousResearch/hermes-agent/pull/64980>。

结论：idle_ttl / linger 是长驻资源的通用兜底。

### 重连与状态续接

- 重连时重放 initialize handshake，并用稳定 session ID 让新 daemon 续接旧连接状态。
- 来源：<https://pkg.go.dev/github.com/plumbkit/plumb@v0.16.0/internal/sessionstate>。

结论：stateful 连接需要显式 session identity；不能把 transport 复用当成状态恢复。

## 本仓现状

- `infra-mcp` 已规定 connection key 池化 + 引用计数 + 最后一个 claim 关闭。
- `McpPrepareHub` 有 claims / `release_session`；切 session 时释放。
- `HostState::evict_slot` 丢弃 driver 时没有释放 claims。
- `view_now` 聚合整个 pool；c3226 临时改为按当前 session 过滤。

## 建议

- host 资源租约：ResourceKey + ref_count + last_used + idle_ttl。
- session attach/detach/close；clean detach 立即释放；崩溃 linger/TTL。
- Scope：Shared / Workspace / Session；有状态 server 用 Session。
- 同 key 复用；跨 workspace 不复用；快照按当前 session 过滤。
- 临时补丁：`evict_slot` 调 `release_session`；最终由租约模型替代。

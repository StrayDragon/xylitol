# Tasks — MCP 连接作为 host 资源租约

- [x] 1. 通用资源租约：`ResourceKey` / `ResourceScope` / `ResourcePool` / `LeaseState`；MCP registry 使用 `ResourcePool<ResourceKey, PoolEntry>`；ref_count / last_used / idle_generation；单测覆盖 acquire / release / idle close / reuse。seam：`crates/xylitol-plugin-sdk/src/resource_lease.rs`、`plugins/xylitol-official-plugin-mcp/src/hub.rs`。
- [x] 2. Host 释放路径：`evict_slot` 调 `release_session`；clean detach 走 last-subscriber / evict；崩溃走 linger；删除 `TODO(c3228)`。seam：`crates/xylitol-serve/src/occupancy/host.rs`、`crates/xylitol-kernel/src/occupancy/factory.rs`。 [blocked-by: 1]
- [x] 3. 配置与策略：`keep_alive_ms` / `idle_ttl_ms` / schema；`scope` 语义；idle 到期关闭与延迟期复用。seam：`plugins/xylitol-official-plugin-mcp/src/config.rs`、`configs/config.schema.json`。 [blocked-by: 2]
- [x] 4. 快照与观测：`LoadedResourcesSnapshot` 只聚合当前 session claims，跳过 idle entry；claims / last_used / reconnect 诊断。seam：`plugins/xylitol-official-plugin-mcp/src/hub.rs`。 [blocked-by: 3]
- [x] 5. Specs：更新 `infra-mcp` 的租约 / scope 规则、`server-core` 的 host-resource-lease、`protocol-app` 的 session-detach。 [blocked-by: 4]
- [x] 6. 回归与收口：`just fmt` / `lint` / `qa`；验证同 workspace 复用、跨 workspace 隔离、stateful session 关闭；真实终端复现 clean detach。 [blocked-by: 5]
- [x] 7. `session_detach`：协议 `METHOD_SESSION_DETACH` / `Command::SessionDetach`；host 在最后 client 且非 busy 时释放租约；TUI clean exit 发送；协议 / TUI 单测。seam：`crates/xylitol-protocol/`、`crates/xylitol-serve/src/occupancy/host.rs`、`crates/xylitol-app-tui/src/`。 [blocked-by: 5]

## 临时 workaround

- c3226 的 `evict_slot` 释放调用保留为最终入口；`TODO(c3228)` 已删除。
- idle grace / scope / last_used 由 `McpPrepareHub` 租约 registry 承担，不再只是立即释放缺口。

## 风险与 fallback

- 若 host 多 client 生命周期复杂，先保证单 client clean detach 与崩溃 linger；多 client 共享 session 后置。
- 若 MCP server 不支持并发 client，默认按 session 隔离，不做跨 client 复用。
- 若 `keep_alive` 与 `idle_ttl` 配置复杂，先固定缺省值，后续再暴露配置。

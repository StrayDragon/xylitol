# Design — MCP transport 配置

## 1. Canonical transport

- 核心只支持 `stdio` 与 `http`。
- `http` = MCP Streamable HTTP（现有 rmcp transport-streamable-http-client）。
- 配置字段 `transport` 只接受 `stdio` / `http`。

## 2. 破坏性拒绝 legacy SSE

- `transport: sse` 不再接受：反序列化失败并给出迁移提示（"sse 已移除；使用 http（Streamable HTTP）"）。
- 不提供 alias / 兼容别名，不做自动回退。
- URL 以 `/sse` 结尾或 server 返回 legacy SSE 语义时，产生可读诊断 "legacy HTTP+SSE is not supported"；不重试为其他 transport。

## 3. Per-server timeout

- `McpServerConfig` 增 `connect_timeout_ms`（缺省 8000）与 `call_timeout_ms`（缺省 120000）。
- connect 阶段超时按 server 独立，单 server 超时不影响其他 server。
- call 阶段超时按 server 覆盖全局 `MCP_CALL_TIMEOUT`；到期返回可分类 Timeout 错误。
- 0 或负数非法，配置校验失败。

## 4. 诊断

- 每 server 记录 last error / timeout 类别；供 c3075 面板显示。
- 失败不得静默当作成功连接。

## 5. 测试

- 配置：sse 拒绝、http 校验、timeout 缺省与非法值。
- 连接：per-server connect timeout。
- 调用：per-server call timeout。
- 诊断：legacy SSE 提示。

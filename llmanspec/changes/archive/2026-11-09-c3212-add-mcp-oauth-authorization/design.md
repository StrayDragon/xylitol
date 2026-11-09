# Design — MCP OAuth authorization flow

## 1. rmcp 3.8 接入

- 每个 `http` server 使用 `OAuthState` 管理授权生命周期；`OAuthState::new(url, Some(reqwest_client))` 后按配置走已有 credential、pre-registration、CIMD 或 DCR。
- 开启 rmcp `auth` feature，并给 MCP 插件加 `reqwest` 依赖；transport 从 `from_config` 改为 `StreamableHttpClientTransport::with_client(AuthClient::new(reqwest_client, manager), config)`。
- 无 token 时先发未认证请求；401 的 `StreamableHttpError::AuthRequired` 必须保留 `www_authenticate_header`，映射为 `McpError::AuthRequired`，再以 `resolve_metadata_from_challenge` + `AuthorizationRequest::with_challenge` 启动授权。
- 授权未完成时连接返回 auth-required 诊断，不阻塞 bootstrap / 首回合 / UI；授权完成后只重连该 connection key。
- `AuthClient` 负责带 token 与内部 refresh；refresh rejected 由 `CredentialStore::on_refresh_token_rejected` 清理并触发重授权；`InsufficientScope` 走 `request_scope_upgrade`；refresh 不得 reset 连接。
- revoke 使用 AS metadata 的 revocation endpoint（`additional_fields`）自定义请求；成功后清 credential store。
- 拒绝 `AuthorizationMetadataSource::LegacyEndpointFallback`：OAuth 2.1 须有真实 Protected Resource Metadata / AS metadata，不得静默合成端点。
- CIMD 用 `AuthorizationRequest::with_client_metadata_url`，仅在 AS 广告 `client_id_metadata_document_supported` 时启用；否则回退 DCR。只支持 public client，不配置 client_secret。

## 2. credential store / keychain

- `StoredMcpCredential` 扩展 `client_id` / `granted_scopes` / `token_received_at` / `token_type` / `issuer`；token 响应按可重建 `OAuthTokenResponse` 的形状保存，Debug 脱敏。
- `OAuthCredentialStore` 实现 rmcp `CredentialStore`，在 `StoredCredentials` 与 `StoredMcpCredential` 间映射；实现 `on_refresh_token_rejected` 与 `acquire_refresh_guard`。
- 后端：keychain 优先（`keyring`；service `xylitol-mcp`，account = credential_id）；不可用/报错回退 0600 文件（`McpCredentialStore`）；`oauth.keychain: auto|off`，默认 `auto`；keychain 调用走 `spawn_blocking`。
- `credential` 缺省时生成稳定默认账号 ID（server name），并进入 connection key；同 server 多账号按 credential ID 隔离。
- DCR 成功后持久化 client_id（即使尚无 token）；重启后优先复用，避免每次重新注册。
- `/mcp` credential set/none 从 token store 读取，不得只看 config。

## 3. callback / host flow

- `McpOAuthGateway`（plugin-sdk 端口）由 Host / 进程内实现提供：`start` 返回 `flow_id` / URL，`complete` 接收 callback URL，`cancel` 关闭 listener。
- MCP 插件持有 `OAuthState` 与 `StateStore`；gateway 只负责 localhost listener 与回调转发，不持有 token。
- Host 绑定 `127.0.0.1:0`，redirect URI 为 `http://127.0.0.1:<port>/callback`；DCR 注册该 URI。pre-registered client 配置固定 redirect URI 时绑定该端口；端口占用返回可操作错误。
- 一次性 listener，超时 5 分钟；校验 `state`、RFC 9207 `iss`；成功保存 credential 并 emit `mcp_oauth_completed`；失败 emit `mcp_oauth_failed`；取消关闭 listener 并清 state。
- 支持多个并发 flow（按 flow_id / CSRF 隔离）；不得把不同 server 的 callback 串线。
- 远程 TUI 不支持：TUI 与 Host 不同机器时 127.0.0.1 callback 不可达，须在 UI/文档明确失败与替代方式。
- 方法：`mcp_oauth_start` 返回 `flow_id` / `url`；`mcp_oauth_cancel` 取消；事件经 `xylitol-wire` 映射。code / token 不进事件、日志、错误。

## 4. TUI

- auth-required 事件或 `/mcp` 行内“Authorize”触发 Choice：打开 URL / 取消；显示 server、scope、过期提示；等待完成事件。
- 错误走滚动提示或通知条；不显示 token；不把回调 URL 中的 code 当可复制秘密；远程 TUI 明确不支持。
- `/mcp` 面板补 transport / scope / credential 是否存在 / 超时 / last error；credential 只显示 set/none。

## 5. 配置

- `mcp_servers[].oauth`：`client_id?`、`client_metadata_url?`、`scopes?`、`redirect_uri?`、`keychain?`；不提供 `resource` 与 `client_secret`。
- `oauth` 仅允许 `http` transport；`stdio` / `custom` 校验失败；redirect URI 仅允许 loopback http。
- `credential` 仍是账号 key；OAuth 要求非空或生成默认账号 ID。
- `fingerprint` 纳入 OAuth 配置（client_id / client_metadata_url / scopes / redirect_uri / keychain），配置变化须重建连接。
- schema、validate、配置文档同步；只支持 public client。

## 6. 安全

- PKCE S256、state、RFC 8707 resource、RFC 9207 iss。
- token 不进 config / secret.env / 日志 / 错误 / 导出 / 工具子进程 env；Debug 脱敏；`/mcp` 只显示 credential set/none。
- 错误、导出、provider trace 不出现 token；revoke 清 store；step-up 只请求所需 scope。
- callback 日志不得记录完整 URL / code / state；OAuth 事件只带 flow_id、server、scope、状态。
- MCP stdio 子进程沿用 `host_env::scrub_tokio_command`；不得把 OAuth token 放进任何 env。

## 7. 测试

- Mock AS fixture：metadata、authorize、token、refresh、revoke、DCR、CIMD、step-up。
- 单测：PKCE / state / iss / resource、challenge discovery、store/keychain fallback、默认账号 ID、fingerprint、脱敏。
- Host callback：成功 / state 错 / iss 错 / 超时 / 取消 / 多 flow；auth-required 不阻塞 prepare；完成后按 FROZEN 语义重定稿。
- BDD：host start → callback → complete；TUI 同意 / 取消；远程 TUI 失败路径。
- 门禁：`just qa`、`just qa-e2e`。

## 8. 边界

- 不做 WebSocket；legacy HTTP+SSE 仍拒绝。
- 不把 token 写进模型上下文。
- 不做 confidential client / client_secret；不做远程 TUI callback。
- 不提供显式 `resource` 覆盖；keychain 回退不改变连接语义。

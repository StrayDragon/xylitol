# Design — MCP OAuth authorization flow

## 1. rmcp 3.8 接入

- 每个 `http` server 在建立连接前构造 `AuthorizationManager::new(url).await`。
- 注入 credential store 与 `InMemoryStateStore`；`resolve_metadata().await` 后写回 manager。
- 有 `client_id` 用 `OAuthClientConfig`；无 `client_id` 用 `register_client` 走 DCR；CIMD 以 client_id URL 走预注册变体。
- `AuthClient::new(reqwest_client, manager)` 包装后交给 `StreamableHttpClientTransport::with_client`。
- 401 / `AuthorizationRequired` 触发授权事件；refresh 由 manager 内部处理；refresh rejected 走重授权提示。

## 2. credential store / keychain

- `StoredMcpCredential` 扩展 `client_id` / `granted_scopes` / `token_received_at` / `token_type`。
- `OAuthCredentialStore` 实现 rmcp `CredentialStore`，在 `StoredCredentials` 与 `StoredMcpCredential` 间映射。
- 后端：keychain 优先（`keyring`；service `xylitol-mcp`，account = credential_id）；不可用/报错回退 0600 文件（`McpCredentialStore`）。
- 配置 `oauth.keychain: auto|off`，默认 `auto`；`auto` 下 keychain 失败只 warn 并回退，不阻塞连接；`off` 强制文件。
- CI / headless 用文件或注入的 in-memory store，不依赖系统 keychain。

## 3. callback / host flow

- Host 绑定 `127.0.0.1:0`，redirect URI 为 `http://127.0.0.1:<port>/callback`；DCR 注册该 URI。
- pre-registered client 配置固定 `redirect_uri` 时绑定该端口；端口占用返回可操作错误。
- 一次性 listener，超时 5 分钟；校验 `state`、RFC 9207 `iss`，再 `exchange_code_for_token_with_issuer`。
- 成功后保存 credential 并 emit `mcp_oauth_completed`；失败 emit `mcp_oauth_failed`。
- 方法：`mcp.oauth.start` 返回 `flow_id` / `url`；`mcp.oauth.cancel` 取消；事件经 `xylitol-wire` 映射。
- code / token 不进事件、日志、错误。

## 4. TUI

- Choice：打开 URL / 取消；显示 server、scope、过期提示；等待完成事件。
- 错误走滚动提示或通知条；不显示 token；不把回调 URL 中的 code 当可复制秘密。

## 5. 配置

- `mcp_servers[].oauth`：`client_id?`、`scopes?`、`redirect_uri?`、`resource?`、`keychain?`。
- `oauth` 仅允许 `http` transport；`stdio` / `custom` 校验失败。
- `credential` 仍是账号 key；OAuth 要求非空或生成默认账号 id。
- schema、validate、配置文档同步。

## 6. 安全

- PKCE S256、state、RFC 8707 resource、RFC 9207 iss。
- token 不进 config / secret.env / 日志 / 工具子进程 env；Debug 脱敏；`/mcp` 只显示 credential set/none。
- 错误、导出、provider trace 不出现 token；revoke 清 store。

## 7. 测试

- Mock AS fixture：metadata、authorize、token、refresh、revoke、DCR。
- 单测：PKCE / state / iss / resource、store fallback、脱敏。
- BDD：host start → callback → complete；TUI 同意 / 取消。
- 门禁：`just qa`、`just qa-e2e`。

## 8. 边界

- 不做 WebSocket；legacy HTTP+SSE 仍拒绝。
- 不把 token 写进模型上下文。
- keychain 回退不改变连接语义。

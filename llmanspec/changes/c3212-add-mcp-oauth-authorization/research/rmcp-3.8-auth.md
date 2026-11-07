# rmcp 3.8 OAuth 接入证据

## 结论

- `AuthClient<C>` 实现 `StreamableHttpClient`，可直接交给 `StreamableHttpClientTransport::with_client`。
- `AuthorizationManager` 提供 metadata 发现、client 配置/DCR、授权 URL、code exchange、credential store 与 state store 注入。
- `CredentialStore` / `StateStore` 均为 async trait，可注入 0600 文件或 keychain 实现。

## 证据

- `rmcp-3.8.0/src/transport/common/auth/streamable_http_client.rs:74`：`impl<C> StreamableHttpClient for AuthClient<C>`。
- `rmcp-3.8.0/src/transport/streamable_http_client.rs:2087`：`with_client(client, config)`。
- `rmcp-3.8.0/src/transport/auth.rs:349`：`CredentialStore` trait（load/save/clear）。
- `rmcp-3.8.0/src/transport/auth.rs:517`：`StateStore` trait。
- `rmcp-3.8.0/src/transport/auth.rs:794`：`OAuthClientConfig`。
- `rmcp-3.8.0/src/transport/auth.rs:2306`：`exchange_code_for_token_with_issuer`。
- `rmcp-3.8.0/src/transport/auth.rs:273`：`StoredCredentials` 形状（client_id / token_response / granted_scopes / token_received_at / issuer）。

## 影响

- 不需要自定义 transport；只需在 MCP http 连接构造时换 client。
- token 刷新由 `AuthClient` 内部触发，MCP 层只需处理 `AuthorizationRequired` 与 refresh rejected。

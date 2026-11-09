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

## 补充证据（2026-11-07 深挖）

- `OAuthState` 是推荐的授权状态机；`AuthorizationSession::new` 负责注册选择与授权 URL，`handle_callback_url` 负责 code exchange / iss 校验：`auth.rs:3814`、`auth.rs:4172`、`auth.rs:4279`。
- `AuthClient` 只负责带 token 与 refresh，不启动授权：`auth.rs:573`；401 以 `StreamableHttpError::AuthRequired` 携带 `www_authenticate_header`：`streamable_http_client.rs:139`。
- 反应式发现入口为 `resolve_metadata_from_challenge`：`auth.rs:1692`；`AuthorizationRequest::with_challenge`：`auth.rs:967`。
- `AuthorizationMetadataSource::LegacyEndpointFallback` 表示端点由客户端合成，不是真实发现：`auth.rs:705`。
- `discovered_resource` 为私有字段，`oauth_resource()` 只用发现值或 base URL；没有公开的 resource 覆盖 setter：`auth.rs:1209`、`auth.rs:2052`。
- rmcp 没有公开 revoke API；只有 oauth2 revocation 类型别名：`auth.rs:990`。revoke 须自定义请求 AS metadata 的 revocation endpoint。
- `CredentialStore` 需要 `on_refresh_token_rejected` / `acquire_refresh_guard`：`auth.rs:349`；`StoredCredentials` 字段含 client_id / token_response / granted_scopes / token_received_at / issuer：`auth.rs:273`。
- `AuthorizationManager::initialize_from_store` 可在重启后复用 token：`auth.rs:1580` 附近。

## 影响

- 不需要自定义 transport；只需在 MCP http 连接构造时换 client。
- token 刷新由 `AuthClient` 内部触发，MCP 层只需处理 `AuthRequired`、refresh rejected 与 step-up。
- 授权流程用 `OAuthState` / `AuthorizationSession`；callback 须与同一 manager/state store 共享。
- `resource` 不提供配置覆盖；revoke 需自定义 HTTP 请求。

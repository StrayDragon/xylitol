# Tasks — MCP OAuth authorization flow

- [ ] 1. 配置：`mcp_servers[].oauth` schema / validate（仅 http、loopback redirect、无 resource/client_secret）；默认 credential ID；fingerprint 纳入 OAuth；单测。
- [ ] 2. credential store：扩展 `StoredMcpCredential`；`OAuthCredentialStore` adapter；keychain backend + 0600 fallback + `oauth.keychain`；`on_refresh_token_rejected` / refresh guard；单测。
- [ ] 3. MCP http 接入：rmcp `auth` feature + reqwest；`OAuthState` / `AuthClient` / `with_client`；typed `AuthRequired` challenge；refresh / revoke / step-up；auth-required 异步与单 key 重连；单测。
- [ ] 4. Host 流程：`McpOAuthGateway`；`mcp_oauth_start` / `cancel`；127.0.0.1 临时端口 callback；state / iss / code exchange；多 flow；超时 / 取消；远程 TUI 不支持；单测。
- [ ] 5. 协议 / wire / TUI：OAuth 方法与事件映射；host client 解码；TUI URL / 同意 / 取消 / 状态；`/mcp` Authorize；单测 / harness。
- [ ] 6. `/mcp` 诊断与脱敏：transport / scope / credential set/none / 超时 / last error；日志、错误、导出、工具子进程 env；审计字段；单测。
- [ ] 7. 测试：Mock AS fixture（metadata / authorize / token / refresh / revoke / DCR / CIMD / step-up）；PKCE / state / iss / resource；callback 成功/失败/超时/取消；auth-required 不阻塞 prepare；TUI / BDD；远程 TUI 失败路径。
- [ ] 8. 门禁：`just fmt` / `just lint` / `just qa` / `just qa-e2e`；`llman-sdd validate --strict` 全勾后 finalize。

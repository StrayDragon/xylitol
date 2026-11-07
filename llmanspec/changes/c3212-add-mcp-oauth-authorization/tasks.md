# Tasks — MCP OAuth authorization flow

- [ ] 1. 配置：`mcp_servers[].oauth` schema / validate；仅 `http` 允许；补 `infra-mcp` spec。
- [ ] 2. credential store：扩展 `StoredMcpCredential`；实现 rmcp `CredentialStore` adapter；keychain backend + 0600 fallback + `oauth.keychain` 开关；单测。
- [ ] 3. MCP http 接入：`AuthorizationManager` + `AuthClient` + `StreamableHttpClientTransport::with_client`；401 / refresh / refresh rejected 处理。
- [ ] 4. Host 流程：`mcp.oauth.start` / `cancel`；127.0.0.1 临时端口 callback；state / iss / code exchange；事件与 `xylitol-wire` 映射。
- [ ] 5. TUI：URL / 同意 / 取消 / 状态；不显示 token。
- [ ] 6. 脱敏：日志、错误、导出、`/mcp`、工具子进程 env；审计字段。
- [ ] 7. 测试：Mock AS fixture；PKCE / state / iss / resource；refresh / revoke；callback 成功/失败；BDD。
- [ ] 8. 门禁：`just fmt` / `just lint` / `just qa` / `just qa-e2e`；`llman-sdd validate --strict` 全勾后 finalize。

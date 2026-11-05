# Design — MCP credentials and secure token store

## 1. 已交付

- McpCredentialStore：agent 目录下 mcp_credentials.json，0600，原子替换（临时文件 + rename）。
- StoredMcpCredential 的 Debug 脱敏；access_token / refresh_token 不进入日志。
- credential ID 已进入 connection key（c3055）；同 server 不同 credential 独立连接与 catalog。
- token 不得进 config / secret.env / 工具子进程 env；store 只由 host 读写。

## 2. 未交付（明确边界）

- OAuth 2.1 授权流程未实现：PKCE、Protected Resource Metadata、AS metadata、DCR、RFC 8707 Resource Indicators、RFC 9207 iss 校验、scope step-up、refresh / revoke、TUI 同意页与 localhost callback 均未接。
- OS keychain 未接；当前只有 0600 文件 fallback。
- 上述未交付项不得在 specs / tasks 中写成已交付；后续 change 再实现。

## 3. 测试

- store round-trip / remove / 0600 权限 / Debug 脱敏。
- connection key 含 credential，同 server 多账号不串。

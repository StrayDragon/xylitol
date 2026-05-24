# c91-fix-security-enforcement Tasks

- [x] 修复 `check_file_tool` 同时检查 `file_path` 和 `path` 字段；空路径返回 Blocked
- [x] 为 MCP 工具添加专用安全策略分支（allowlist 配置 + default deny）
- [x] 将 `SecurityConfig.enabled` 默认值改为 `true`
- [x] Hook 超时路径改为 fail-closed（Block）；tokio 超时自动 drop 子进程
- [x] 实现 `network.allowed_domains`/`blocked_domains` 检查（bash 命令中的域名匹配）
- [x] bash timeout 取 `min(args.timeout, MAX_TIMEOUT_SECS)` 并校验正值
- [x] TUI `requires_approval` 扩展覆盖 bash 工具
- [x] 更新 SecurityConfig：新增 `mcp_allowlist` 字段 + `default_security_enabled` 默认值
- [x] SecurityEngine 暴露 `bash_timeout_secs()` 供上层使用
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c91-fix-security-enforcement --strict --no-interactive`

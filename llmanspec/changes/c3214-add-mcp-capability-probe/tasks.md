# Tasks — MCP capability probe and reconnect count

- [x] 1. probe：连接后异步 `Peer::discover`；按 connection key 缓存；TTL 取 `ttl_ms` 夹取 60s–1h；失败分类 unsupported / timeout / transport / auth / invalid_response。
- [x] 2. 计数：pool 按 connection key 记录重建/重连次数；`/mcp` 诊断展示；`/reload` 重置。
- [x] 3. 诊断：`server_details` 增加 `probe=` 与 `reconnect_count=`；`SessionPrepareView` 增加 probe 行。
- [x] 4. TUI：`/mcp` 面板经 `mcp_diag_short` 显示 probe 行；`/reload` 重建 pool 后重新探测。
- [x] 5. Specs：`infra-mcp` capability probe 与 reconnect count 规则；单测 / harness。
- [x] 6. 门禁：`just fmt` / `just lint` / `just qa` / `just qa-e2e`。

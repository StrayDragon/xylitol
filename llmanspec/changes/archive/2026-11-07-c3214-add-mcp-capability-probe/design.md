# Design — MCP capability probe and reconnect count

## 1. probe 时机

- 连接 initialize 成功后异步触发；不阻塞 bootstrap / 首回合 / UI。
- 每个 connection key 只探测一次；缓存命中直接返回。
- `/reload` 重建 pool 后重新探测；本 change 不新增单独 probe 命令。

## 2. `server/discover`

- 用 `Peer::discover(meta)`；返回 `DiscoverResult`。
- 记录 `supported_versions` / `capabilities` / `instructions` / `ttl_ms` / `cache_scope`。
- `ttl_ms` 夹取 60s–1h；`cache_scope` 只影响跨 auth context 复用，本进程按 connection key 隔离。
- 失败：保留连接，诊断记 `unsupported` / `timeout` / `transport` / `auth` / `invalid_response`；不自动重试。
- legacy：不支持 discover 时沿用 initialize + `tools/list` 结果，标记 `legacy`。

## 3. 重连计数

- pool 每个 connection key 维护 `reconnect_count`。
- 同一 connection key 的 pool 重建/重连递增；手动 `/reload` 重置。
- `/mcp` 展示 count 与 last error；不展示 token/credential 值。
- 计数只用于诊断，不改变重连策略。

## 4. 成本与隐私

- discover 只发一次请求；不触发工具调用；受 `connect_timeout_ms` 限制。
- 不向非本机地址额外探测；沿用 server URL 与 auth 配置。
- 不把 capabilities / instructions 写进模型上下文。

## 5. 测试

- 单测：probe 分类/排序；重连计数递增与重置。
- BDD：`/mcp` 显示 probe 行与 reconnect count。
- 门禁：`just qa`。

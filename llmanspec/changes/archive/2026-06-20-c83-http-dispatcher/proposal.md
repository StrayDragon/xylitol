---
depends_on: []
---

# c83-http-dispatcher: HTTP 代理 + 空闲超时配置

## Why
pi 的 `http-dispatcher.ts`（73 LOC）提供两件事：
1. `applyHttpProxySettings(proxy)` — 将用户配置的 httpProxy 应用到环境变量 `HTTP_PROXY`/`HTTPS_PROXY`
2. `configureHttpDispatcher(timeoutMs)` — 设置全局 HTTP 空闲超时（header/body idle timeout）

xylitol 使用 `reqwest::Client` 发送 LLM API 请求，但完全没有代理配置路径和超时控制，导致：
- 企业代理环境无法使用
- 长时间无响应的请求无法超时断开（浪费 token 配额）

## What Changes
- 新增 `src/agent/http_dispatcher.rs`：
  - `apply_http_proxy_settings(http_proxy: Option<&str>)` — 设置系统代理环境变量
  - `parse_http_idle_timeout_ms(value: &str) -> Option<u64>` — 从字符串/数字解析超时
  - `configure_http_client(client_builder, timeout_ms)` — 在 `reqwest::ClientBuilder` 上设置 `connect_timeout` / `read_timeout` / `pool_idle_timeout`
  - `HTTP_IDLE_TIMEOUT_CHOICES` 预设常量（30s, 1min, 2min, 5min, disabled）
  - `DEFAULT_HTTP_IDLE_TIMEOUT_MS`（300_000 = 5 分钟）
- `Settings` 新增 `httpProxy`/`httpIdleTimeoutMs`/`websocketConnectTimeoutMs` 字段（占位定义，完整接入在 c89）
- 集成到 `agent/provider/mod.rs`：构建 `reqwest::Client` 时调用 `configure_http_client`

## Capabilities
- network-config

## Impact
- 非破坏性：新增模块。Settings 字段增为 Option，默认 None 即不启用。
- 仅依赖 `reqwest`（已是项目依赖）。

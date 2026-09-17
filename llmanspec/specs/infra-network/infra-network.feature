# language: zh-CN
# capability: infra-network
# purpose: 网络配置 — HTTP 代理设置、idle 超时与 dispatcher 配置。
# scope: src/infra/config/, src/infra/settings/

功能: infra-network

  @req:r1455 @human
  场景: placeholder
    - System SHALL 支持可配置的 HTTP 代理与 idle 超时设置。

  @req:r1456 @human
  场景: proxy-config
    - System MUST 从 httpProxy 设置应用 HTTP_PROXY 与 HTTPS_PROXY 环境变量。

  @req:r1457 @human
  场景: idle-timeout
    - System MUST 用可配置的 connect_timeout、read_timeout 与 pool_idle_timeout 配置 reqwest Client。

  @req:r1458 @human
  场景: timeout-parser
    - System MUST 提供 parse_http_idle_timeout_ms，接受字符串 30 sec 或数值毫秒。

  @req:r1459 @human
  场景: timeout-choices
    - System MUST 提供 HTTP_IDLE_TIMEOUT_CHOICES 常量，含预设 30s、1min、2min、5min、disabled。

  @req:r1460 @human
  场景: settings-stubs
    - System SHALL 向 Settings 添加 httpProxy、httpIdleTimeoutMs、websocketConnectTimeoutMs 字段。

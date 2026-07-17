# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-network

  @req:nc0
  场景: placeholder
    假如 未配置网络
    当 无操作
    那么 无变化

  @req:nc1
  场景: proxy
    假如 'httpProxy 设为 "http://proxy.example:8080"'
    当 apply_http_proxy_settings 被调用
    那么 HTTP_PROXY 与 HTTPS_PROXY 环境变量被设置

  @req:nc2
  场景: timeout
    假如 httpIdleTimeoutMs 为 120000
    当 configure_http_client 被调用
    那么 client 具有 120s read timeout

  @req:nc3
  场景: parse-string
    假如 '值为 "5 min"'
    当 parse 被调用
    那么 返回 300000

  @req:nc3
  场景: parse-numeric
    假如 值为 30000
    当 parse 被调用
    那么 返回 30000

  @req:nc4
  场景: presets
    假如 访问 timeout choices
    当 读取 HTTP_IDLE_TIMEOUT_CHOICES
    那么 5 个预设均存在

  @req:nc5
  场景: stub-settings
    假如 settings 中设置 httpProxy 与 httpIdleTimeoutMs
    当 Settings 序列化为 JSON
    那么 两字段均以 camelCase 出现

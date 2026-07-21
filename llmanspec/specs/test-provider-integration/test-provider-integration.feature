# language: zh-CN
# managed by llman sdd partition-migrate
功能: test-provider-integration

  @req:pi0
  场景: config-api
    假如 model config 含 api 字段
    当 factory 构建 provider
    那么 正确 adapter 被注入 provider

  @req:cv1
  场景: literal
    假如 config 值为纯字符串
    当 解析该值
    那么 返回 literal 字符串

  @req:cv2
  场景: env-var
    假如 config 值为 MY_KEY 且 MY_KEY 在 env 中已设置
    当 解析该值
    那么 返回 env var 值

  @req:cv2
  场景: missing-env
    假如 config 值为 MISSING 且变量未设置
    当 解析该值
    那么 返回 None

  @req:cv3
  场景: shell-cmd
    假如 'config 值为 "!echo hello"'
    当 命令在 10 秒内执行
    那么 返回 hello

  @req:cv3
  场景: shell-cache
    假如 同一命令解析两次
    当 值被缓存
    那么 第二次调用不重新执行

  @req:pi2
  场景: provider-moved
    假如 检查 src/agent/provider 是否存在
    当 检查该路径
    那么 不存在，且 src/infra/provider/ 存在且 XyModel 从 crate::protocol 导入

  @req:pi3
  场景: no-concrete-provider
    假如 agent 引用 provider
    当 检查引用类型
    那么 为 XyModel trait object，非 OpenAIProvider

# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-ai-bridge-accounting

  @req:paa1
  场景: prefer-api
    假如 消息集含可信 last Api usage 且其后无新消息
    当 调用 estimate_context
    那么 provenance 为 Api 且 tokens 反映该 usage

  @req:paa11
  场景: api-trailing-after-anchor-only
    假如 消息集含长历史与带 usage 的末条 assistant 及之后少量新消息
    当 调用 estimate_context
    那么 provenance 为 Api 且 tokens 约等于 usage 加锚点后增量而非 usage 加全量 heuristic

  @req:paa1
  @req:paa10
  场景: fallback-local
    假如 无 Api 锚点且 LocalTokenizer 闸为 on 且 registry 命中已加载词表
    当 调用 estimate_context
    那么 provenance 为 LocalTokenizer

  @req:paa10
  场景: local-off-skips-tokenizer
    假如 无 Api 锚点且 LocalTokenizer 闸为 off 且 registry 已映射可用词表
    当 调用 estimate_context
    那么 provenance 不是 LocalTokenizer 且估计路径未调用本地 encode

  @req:paa1
  场景: fallback-heuristic
    假如 无 Api、RemoteCount 不可用、无 LocalTokenizer
    当 调用 estimate_context
    那么 provenance 为 Heuristic 且仍返回估计值

  @req:paa2
  场景: abort-not-anchor
    假如 最近 assistant 为 abort 且带非零 usage
    当 估计当前上下文
    那么 不得仅因该 usage 将 provenance 标为 Api 锚点

  @req:paa3
  场景: no-per-delta-encode
    假如 正在流式生成且开启本地 tokenizer
    当 仅处理 TextDelta 热路径
    那么 不因每个 delta 触发全文 encode

  @req:paa4
  场景: heuristic-provenance
    假如 估计结果来自 chars/4
    当 检查 ContextTokenEstimate
    那么 provenance 为 Heuristic

  @req:paa5
  场景: remote-fail-degrade
    假如 RemoteCount 已启用但网络失败
    当 估计上下文
    那么 降级到 LocalTokenizer 或 Heuristic 且 provenance 不是 Api

  @req:paa6
  场景: hf-opt-in
    假如 模型映射到 HuggingFace tokenizer 且本地无缓存且未 opt-in 下载
    当 估计上下文
    那么 不静默下载大文件并降级到 Heuristic 或明确错误策略后降级

  @req:paa8
  场景: cache-list-and-remove
    假如 缓存目录中已有 tokenizer.json 条目
    当 列举并删除该缓存键
    那么 列举曾包含该条目且删除后不再包含

  @req:paa8
  场景: download-atomic
    假如 opt-in download 中途失败
    当 再次 encode_count_if_cached
    那么 不把不完整文件当作可用缓存

  @req:paa9
  场景: hf-endpoint-mirror
    假如 环境变量 HF_ENDPOINT 为 https://hf-mirror.com
    当 拼装某 repo 的 tokenizer.json resolve URL
    那么 URL 以 https://hf-mirror.com/ 为前缀且含 resolve/main/tokenizer.json

  @req:paa9
  场景: hf-endpoint-default
    假如 未设置 HF_ENDPOINT 与 HF_HUB_ENDPOINT
    当 拼装 resolve URL
    那么 基址为 https://huggingface.co

  @req:paa7
  场景: openai-input-tokens-success
    假如 Responses 路径 RemoteCount 启用且 input_tokens 返回 123
    当 调用 estimate 且无 Api 锚点
    那么 provenance 为 RemoteCount 且 tokens 为 123

  @req:paa7
  场景: openai-input-tokens-http-fail
    假如 Responses RemoteCount 启用但 HTTP 非 2xx
    当 估计上下文
    那么 provenance 不是 RemoteCount 也不是 Api

  @req:paa7
  场景: completions-no-fake-remote
    假如 仅 Completions adapter 且未声明 input_tokens
    当 未注入 remote_count_tokens
    那么 不声称 OpenAI RemoteCount 成功

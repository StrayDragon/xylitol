# language: zh-CN
# capability: package-ai-bridge-accounting
# purpose: "xylitol-ai-bridge 多源上下文 token 计量与 TokenProvenance。"
# scope: packages/xylitol-ai-bridge/, src/agent/compaction/

功能: package-ai-bridge-accounting

  @req:paa1 @human
  场景: 计量优先级
    - 上下文 token 估计 MUST 按固定优先级解析来源：Api，然后 RemoteCount，然后 LocalTokenizer，然后 Heuristic；结果 MUST 携带 TokenProvenance（或等价枚举）标明实际采用的来源。

  @req:paa2 @human
  场景: Api-锚点规则
    - 当存在适用于当前会话 prefix/leaf 的可信厂商 usage 时 MUST 优先用作 Api 锚点；因 abort 或 error 结束的回合 usage、以及 compaction 之后不再描述当前 prefix 的旧 usage，MUST NOT 作为锚点。

  @req:paa11 @human
  场景: Api-trailing-范围
    - 当 Api 锚点有效时，trailing_tokens MUST 仅计入 last usage 消息之后的消息（对齐 pi estimateContextTokens）；MUST NOT 对锚点及之前的消息再叠加 heuristic；last_usage_index MUST 为该锚点消息下标；若消息列表中找不到带 usage 的 assistant，可将整段列表视为 trailing 且 last_usage_index 可为 null。

  @req:paa3 @human
  场景: 禁止流上全量 encode
    - generate_stream 热路径 MUST NOT 在每个文本 delta 上对累计全文调用完整本地 tokenizer.encode；本地精确计数 MUST 仅在显式 estimate 调用、流结束补洞或经文档化的节流策略下发生。

  @req:paa4 @human
  场景: Heuristic-诚实标注
    - 当最终采用 Heuristic 时，ContextTokenEstimate 的 provenance MUST 为 Heuristic；下游展示层若使用该结果 MUST 能区分于 Api/LocalTokenizer。

  @req:paa5 @human
  场景: RemoteCount-可降级
    - RemoteCount（Anthropic count_tokens 与 OpenAI Responses input_tokens，及等价注入 stub）MUST 可配置关闭；调用失败或超时时 MUST 降级到 LocalTokenizer 或 Heuristic，且 MUST NOT 将失败结果标为 Api。

  @req:paa6 @human
  场景: LocalTokenizer-注册与缓存
    - LocalTokenizer MUST 经 registry 将 model_id 映射到 Builtin、HuggingFace tokenizer.json 或本地 path 源（用户配置优先于 builtin 启发式）；HuggingFace 文件下载 MUST 为 opt-in 或显式 prefetch，默认 MUST NOT 静默拉取大文件；缓存命中或 local path 可用后 MUST 可离线加载。

  @req:paa7 @human
  场景: openai-responses-remote-count
    - 当模型路径为 OpenAI Responses（或声明兼容 input_tokens 的端点）且 RemoteCount 已启用时，估计上下文 MUST 可经 POST /v1/responses/input_tokens（或配置的 base_url 等价路径）取得 input_tokens 并标 provenance 为 RemoteCount；非 Responses 路径 MUST NOT 伪造该远程调用成功。

  @req:paa8 @human
  场景: tokenizer-cache-manage-api
    - HfTokenizerCache（或等价）MUST 提供可测的缓存根查询、条目列举与按键删除；opt-in download MUST 在成功前不把不完整文件当作可用缓存；CLI 与其它面 MUST 经此 API 管理缓存，MUST NOT 在估计路径调用 download。

  @req:paa9 @human
  场景: hf-endpoint-base
    - 拼装 HuggingFace resolve URL 时 MUST 以环境变量 HF_ENDPOINT 为优先基址（去尾斜杠），未设置时可回退 HF_HUB_ENDPOINT，再默认 https://huggingface.co；URL 形状 MUST 为 {base}/{repo}/resolve/main/{file}（本波 revision 固定 main）；opt-in download MUST 使用该基址，MUST NOT 在已设 HF_ENDPOINT 时仍硬编码仅官方域。

  @req:paa10 @human
  场景: local-tokenizer-gate
    - LocalTokenizer 档 MUST 仅在显式允许时参与估计（配置 on|off，默认 off）；off 时即便 registry 已映射或缓存可用，估计路径 MUST NOT 调用本地 encode，并 MUST 继续按 paa1 降级到其后档；本波 MUST NOT 引入 every-N 或 idle 等其它本地计数策略。

  @executable @req:paa8
  场景: cache-list-and-remove
    假如 缓存目录中已有 tokenizer.json 条目
    当 列举并删除该缓存键
    那么 列举曾包含该条目且删除后不再包含

  @executable @req:paa8
  场景: download-atomic
    假如 opt-in download 中途失败
    当 再次 encode_count_if_cached
    那么 不把不完整文件当作可用缓存

  @executable @req:paa9
  场景: hf-endpoint-mirror
    假如 环境变量 HF_ENDPOINT 为 https://hf-mirror.com
    当 拼装某 repo 的 tokenizer.json resolve URL
    那么 URL 以 https://hf-mirror.com/ 为前缀且含 resolve/main/tokenizer.json

  @executable @req:paa9
  场景: hf-endpoint-default
    假如 未设置 HF_ENDPOINT 与 HF_HUB_ENDPOINT
    当 拼装 resolve URL
    那么 基址为 https://huggingface.co

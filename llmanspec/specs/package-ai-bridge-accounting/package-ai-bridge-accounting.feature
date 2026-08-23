# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-ai-bridge-accounting

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

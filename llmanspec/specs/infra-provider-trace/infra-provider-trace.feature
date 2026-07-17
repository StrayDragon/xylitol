# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-provider-trace

  @req:ipt1
  场景: raw-mapped-correlate
    假如 Responses 流且 provider tracing 已激活并产生 reasoning 与 text
    当 流结束
    那么 provider-trace.jsonl 中存在共享同一 request_id 的 raw 与 mapped 记录且覆盖 ThinkingDelta 与 TextDelta

  @req:ipt1
  场景: no-secrets-in-dump
    假如 provider tracing 已激活
    当 发出带 Authorization 头的请求
    那么 trace 文件 MUST NOT 含 bearer 或 API key 材料

  @req:ipt2
  场景: release-off-by-default
    假如 release 构建且未设 XYLITOL_PROVIDER_TRACE
    当 跑一次 provider 流
    那么 不应因该流产生可归因的 provider-trace 增长

  @req:ipt3
  场景: no-tracing-crate
    假如 变更完成后
    当 rg tracing 于 Cargo.toml 与 src
    那么 无 tracing / tracing-subscriber 依赖与 import

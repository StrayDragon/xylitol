---
depends_on: []
---

## Why

批 3 遗留（c2829 triage）：4 条 (c) naked 全部卡在同一断言面——`llm.request` 仅由 native HTTP 适配层导出，fake provider 无 HTTP 层。一个可控 mock HTTP 上游 harness 可一次性吃掉 4 条并沉淀流式回归基建。

## What Changes（设想）

- mock 上游（借鉴 tests/inner/provider_http_stream_abort.rs 的 wiremock/本地 listener 雏形）：可脚本化 SSE 帧序列、可控慢速 chunk-gap、abort 注入。
- r1462：gate-on 下 raw 与 mapped 事件在 llm.request 上成对。
- r1473：io=full 时 llm.request 携带完整请求体 input（截断上限内）。
- r1474：未见 Done 提前断开 → llm.request 携带 ERROR/aborted 且按档 flush。
- r1556：慢上游触发 chunk-gap idle 上界 → 可分类超时错误。

## Evidence

llmanspec/specs/infra-otel、infra-observability、package-ai-bridge 对应规则；triage 记录 c2829 research/triage.md。

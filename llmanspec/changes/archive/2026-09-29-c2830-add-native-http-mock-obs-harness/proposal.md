---
depends_on: []
branch: sdd/c2830-native-http-mock-obs-harness
base_branch: main
base_sha: c12c3618f67b9f14638c8b1132da536b1b83422e
---

## Why

批 3 遗留（c2829 triage）：4 条 (c) naked 全部卡在同一断言面——`llm.request` 仅由 native HTTP 适配层导出，fake provider 无 HTTP 层。`ProviderRequestTrace`（capture_request_input / emit_raw / emit_mapped_chunk / aborted finalize）机制层单测已齐，缺的是**真适配器在真实 HTTP 流上的端到端接线证据**。一个可控 mock HTTP 上游 harness 一次性吃掉 4 条并沉淀流式回归基建。

## What Changes

- **BDD mock 上游 harness**（`tests/bdd/steps_c2830.rs`）：TcpListener 脚本化 SSE 服务（借鉴 `tests/inner/provider_http_stream_abort.rs` 雏形）——可编排 Anthropic SSE 帧序列（message_start/content_block_*/message_delta/message_stop）、半程后 Stall；真 `AnthropicMessagesAdapter` 以 `base_url` 指向 mock，`SpanCollectScope` 收集断言（复用 `OtelBdd` 闸具，io=truncated）。
  - r1462：同一 `llm.request` span 上 raw 与 mapped 事件成对（`SpanRecord.events` 断言）。
  - r1473：io=truncated 下 `langfuse.observation.input` 来自适配器 HTTP 前捕获的请求体（含 model 与 stream 旗标）——流式路径。
  - r1474：首个 TextDelta 后 drop 流 → `level=ERROR` + `status_message=aborted`、输入输出按档 flush、无 usage_details。
- **r1556（bridge cap，规则本体在 wait_bounds）**：SSE idle 上界为 90s 程序权威常量，真实时间等不起——以 bridge 包内 `#[tokio::test(start_paused)]` 单测 + stalled mock 上游（auto-advance 瞬时烧完 90s）断言可分类的 `provider SSE idle` 错误；规则加 `# verified-by: fn` 锚（组件边界，单测承载比 BDD 场景更诚实）。
- bridge dev-deps tokio 补 `test-util` feature（start_paused 需要）。

## Evidence

- llmanspec/specs/infra-observability r1462、infra-otel r1473/r1474；packages/xylitol-ai-bridge/llmanspec r1556。
- c2829 research/triage.md；matrices：naked 6→2（余 r1415、r1790 归 c2831/c2832）。

# c1590 Design

## Seam

- Unit：`ProviderRequestTrace`（capture request JSON → Done/Drop → span properties）
- Adapter 调用点：`generate` / `generate_stream` 在 HTTP 前 `capture_request_input`
- Feature：`infra-otel` otel10 / otel16 / otel17（文档 + `@req`；实现以 bridge 单测为主）

## Flow

```text
build_body / serialize request
  → ProviderRequestTrace::capture_request_input(json)   # tier ≠ none
  → HTTP stream…
  → TextDelta → output_buf
  → Done { usage? } → finalize(Completed): usage? + attach I/O
  → drop without Done → finalize(Aborted): ERROR + status_message=aborted + attach I/O
```

## Attribute keys（Langfuse OTEL）

- `langfuse.observation.input` / `output`（既有）
- `langfuse.observation.level` = `ERROR`
- `langfuse.observation.status_message` = `aborted`

## Non-goals

- turn 根 ERROR
- ThinkingDelta 并入 observation output
- 整份 session JSONL 上云
- CancellationToken 贯穿 adapter（Drop 即 abort 语义即可）

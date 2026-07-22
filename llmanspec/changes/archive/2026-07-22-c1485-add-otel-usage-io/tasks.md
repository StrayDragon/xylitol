# Tasks: c1485-add-otel-usage-io

## Phase A — Usage

- [x] `ProviderRequestTrace::emit_mapped_chunk`：`Done`+usage → `add_property` gen_ai.usage.* + usage_details JSON
- [x] 单测：inactive 无分配；active + usage 写入属性键名

## Phase B — I/O 档

- [x] `OtelConfig.observation_io`（none/truncated/full，默认 none）+ schema/文档片段
- [x] bridge 静态闸 `set_observation_io_tier` / `observation_io_tier`；logging/bootstrap 接线
- [x] truncated/full 时在 generation span 上写 `langfuse.observation.input`/`output`
- [x] 默认 none 时单测确认不写 I/O 属性

## Docs / verify

- [x] 更新 `docs/roadmaps/OTEL与Langfuse观测.md` M3 行
- [x] `llman sdd validate c1485-add-otel-usage-io --strict --no-interactive`
- [x] `cargo test -p xylitol-ai-bridge` + config 相关单测

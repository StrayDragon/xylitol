# Design: c1485-add-otel-usage-io

## Decisions

### Usage on existing `provider.request` span

- Keep a single fastrace root for the HTTP stream (`ProviderRequestTrace`).
- On `AiBridgeChunk::Done { usage: Some(u), .. }`, call `Span::add_property` **before** the span drops so OTLP export sees usage.
- Prefer GenAI semantic attrs (`gen_ai.usage.input_tokens` / `output_tokens`) plus Langfuse-friendly `langfuse.observation.usage_details` JSON (`{"input":N,"output":N,"total":N}` + optional cache fields when non-zero).
- Missing usage → write nothing (do not emit zeros).

### Observation I/O tier without infra→bridge type coupling

- Config lives on `OtelConfig.observation_io` (`none` | `truncated` | `full`, default `none`).
- Composition root maps enum → bridge `ObservationIoTier` atomic/OnceLock (same pattern as `provider_trace_active`).
- **truncated**: reuse `PROVIDER_TRACE_TEXT_MAX` (4096 Unicode scalars) for input/output strings.
- **full**: still cap at a higher hard limit (e.g. 64Ki chars) to protect OTLP batch size.
- Capture strategy (minimal): accumulate mapped text deltas + thinking into small buffers only when tier ≠ none; on Done, set `langfuse.observation.input` (best-effort request summary if available) and `langfuse.observation.output` (concatenated assistant text). If request body is not already buffered for raw emit, output-only is acceptable for v1 of this change; document gap.

### Non-goals

- Pricing tables / cost_details from model catalog.
- Parent/child span trees.
- Tool observation I/O in this slice (generation first).

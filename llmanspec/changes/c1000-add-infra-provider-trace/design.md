# Design — c1000-add-infra-provider-trace

## 问题

「正文出现在 thinking」可能是：

1. **上游**把正文放进 `reasoning_*` / `thinking` 事件；或
2. **本仓适配器**把 `output_text` 错映成 `ThinkingDelta`。

只看 session / UI 无法证伪。需要 **同一 request 上 raw 与 mapped 对照**，且 **agent 能直接读文件**（非外挂 GUI）。

## Crates 调研结论（基础设施选型）

| 选项 | 结论 |
|------|------|
| **`tracing` + `tracing-subscriber`（已有）** | **采用**。组合根已有 file-only subscriber（c460/ath3）；库内 emit + EnvFilter target 是 Rust 事实标准。 |
| 自定义 `ProviderTraceSink` port | **薄封装可有**（测试注入），但默认实现应走 tracing Layer，避免第二套总线。 |
| **`fastrace` / minitrace** | **本 change 不引入**。适合「库级零开销 timeline + OTel」；本仓已绑 tracing，双栈会碎基础。记入 future：若 SSE 全量 dump 在压测下不可接受再评估。 |
| OpenTelemetry | **不做**。目标是本地 agent 分析，不是分布式后端。 |
| 把 raw 塞进 script hook | **禁止**（与 c735/c999 边界一致）。 |

### tracing 高性能惯例（本 change MUST 遵守）

1. **稳定 `target`**：`xylitol::provider::trace`（EnvFilter 可单独开关，不淹没 `xylitol=debug`）。
2. **Span 管因果**：`request_id` / `model` / `api` 进 span；raw 与 mapped 为 **Event**。
3. **先门闩再分配**：昂贵 `String`/`Value` 克隆前用 `tracing::enabled!(target: …, Level::TRACE)`（或等价 helper）。
4. **关闭近零开销**：无 Layer / filter 不感兴趣时，callsite 短路；可选后续 `release_max_level_*`（另 change）。
5. **高流量专用文件**：SSE 不灌进 `xylitol.log`；独立 JSONL + **同步 append**（对齐 logging：`panic=abort` 不丢尾）。
6. **Layer 组合**：`Registry` + 现有 fmt Layer +（可选）provider-trace Layer；per-layer `EnvFilter` / 专用 env。

参考：tokio-rs/tracing（`enabled!` / `STATIC_MAX_LEVEL`）、tracing-subscriber Layer 组合与 JSON fmt。

## 架构

```text
Adapter SSE loop
  │  raw event (type + data snippet)
  ├──────────────► tracing event  target=xylitol::provider::trace  kind=raw
  │  map → XyChunk
  ├──────────────► tracing event  same request_id                 kind=mapped
  ▼
XyStream → agent

Subscriber (composition root)
  ├─ xylitol.log          (现有人类/通用 debug)
  └─ provider-trace.jsonl (本 change；agent 友好对照)
```

### JSONL 行形状（示意）

```json
{"ts":"...","request_id":"…","kind":"raw","api":"openai-responses","event":"response.reasoning_text.delta","delta":"…"}
{"ts":"...","request_id":"…","kind":"mapped","variant":"ThinkingDelta","text":"…"}
```

截断：超长 delta MAY 截断并标 `truncated:true`（实现选合理上限，设计不定死字节数）。

### 闸门

| 构建 | 默认 | 打开方式 |
|------|------|----------|
| debug (`debug_assertions`) | **开**（与 ath3 文件日志一致） | `XYLITOL_PROVIDER_TRACE=0` 可关（若实现） |
| release | **关** | `XYLITOL_PROVIDER_TRACE=1`；或 `RUST_LOG=xylitol::provider::trace=trace` |

### 安全

- 永不 dump `authorization` / `x-api-key` / `api-key` 头值。
- 文件权限对齐 `0o600`。
- 永不写 stdout/stderr。

### 与隔离原则

Hook 三缝仍只认 `HeaderBag` / body Value。Trace 是 **旁路观测**，不经 hook，不耦合 reqwest（在适配器已解析的字符串/JSON 上 emit）。

## Future

- fastrace 或采样（只保留含 Thinking+Text 混用的 turn）
- 与 session turn id 更强关联
- OTel 导出（若产品需要）

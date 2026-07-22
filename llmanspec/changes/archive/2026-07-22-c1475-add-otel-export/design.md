# Design: c1475-add-otel-export

## 决策摘要

| 决策 | 选择 | 理由 |
|---|---|---|
| 时间线栈 | 保持 fastrace | `src/AGENTS.md` 禁止 tracing 双栈 |
| OTEL 桥 | `fastrace-opentelemetry` + `opentelemetry-otlp` HTTP | 官方 companion；Langfuse 仅 HTTP |
| 分包 | 主仓 `infra/observability` + feature `otel` | 不为分层再拆 crate；重依赖可关 |
| 默认 | `exporter=none` / 未配置 | 对齐 `ipt4` no-default-otel；Langfuse 未就绪 |
| 配置面 | `[otel]` in `config.yaml` | 远程出口需 endpoint/凭证；与本地 file 闸（env/build）正交 |
| 失败策略 | 降级不收集 | 观测不得阻断对话 |

## 装配流

```text
app::cli::init_logging / 观测装配
  ├─ 本地 FileReporter？ ← XYLITOL_PROVIDER_TRACE / debug（既有）
  └─ OTEL？ ← feature otel + AppConfig.otel
        ├─ none / 缺配 / 构建失败 → 跳过
        └─ otlp-http → OpenTelemetryReporter
  └─ fan-out Reporter（0..2 下游）→ fastrace::set_reporter
```

## 配置草图

```toml
[otel]
exporter = "none"           # none | otlp-http
# endpoint = "http://localhost:3000/api/public/otel"
# protocol = "http-json"   # 默认；部分 Langfuse self-host 对 protobuf 吞掉不出 trace
# environment = "dev"
# service_name = "xylitol"
# headers 经 secret / env 注入（勿把 secret 明文写进可分享 config）
```

Langfuse Basic Auth：`Authorization=Basic base64(pk:sk)`，可选 `x-langfuse-ingestion-version=4`。
OTLP HTTP 使用 **async reqwest**（禁止在 app Tokio 内构建 `reqwest::blocking`，会 panic）。导出 `block_on`：有 Handle 则 `block_in_place`；否则私有 `xylitol-otel` runtime。
客户端强制 **HTTP/1.1**。`with_endpoint` 须带 `/v1/traces`（装配侧规范化）。`ForceSampledExporter` 强制 SAMPLED。

## 与 Codex 的差异

- Codex：`codex-otel` + tracing layers；`OtelExporter::None` → `Ok(None)`
- xylitol：同「None → 不装」语义，但桥是 fastrace reporter，不是 tracing-subscriber

## 测试策略

- 单测：config 解析；`exporter=none` 不创建 exporter；坏 endpoint 构建失败 → None
- 可选：loopback HTTP 收 span（feature `otel`）；无 backend 时跳过
- BDD：默认不出口（文档/结构场景可 `feature: false` 或轻量 harness）

## 非目标提醒

属性语义（generation / tool / session）留给 c1480；本刀只保证「能导出已有 span 壳」。

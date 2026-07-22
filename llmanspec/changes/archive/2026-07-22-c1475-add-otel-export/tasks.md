# Tasks: c1475-add-otel-export

## 1. 配置与 feature

- [x] Cargo feature `otel` + 依赖（`fastrace-opentelemetry`、`opentelemetry`、`opentelemetry_sdk`、`opentelemetry-otlp` HTTP）
- [x] `AppConfig` / schemars：`OtelConfig`（exporter/endpoint/protocol/environment/service_name/headers）
- [x] 加载：缺省 ≡ none；非法枚举 fail load；凭证从 secret/env 解析（设计对齐 MCP 密钥习惯）
- [x] 单测：rc24 形状与默认

## 2. Reporter 装配

- [x] `infra/observability`：fan-out `Reporter`（File ± OTEL）
- [x] `OpenTelemetryReporter` 仅在 `feature = "otel"` + `exporter=otlp-http` + 合法 endpoint 时构建
- [x] 构建失败 / 缺凭证 → log warn（file）+ 跳过 OTEL
- [x] `flush_observability` 仍 `fastrace::flush()`（覆盖两侧）
- [x] 单测：none 路径；构建失败降级（mock/错误注入）

## 3. 组合根接线

- [x] `app/cli/logging.rs`（或紧邻）：读 `LoadedAppConfig.otel` 装配；TUI 安全（无 stdout/stderr）
- [x] 无 `otel` feature 时配置字段可存在但 MUST 视为 none（或编译期忽略导出）

## 4. 合约与文档

- [x] live `infra-otel` spec + feature（本 change）
- [x] `runtime-config` 增 rc24；`infra-provider-trace` 澄清 opt-in
- [x] roadmap / architecture 已替换 Inspect（本分支 docs）
- [x] 更新 `xylitol-inspect-runtime-logs` skill 一句：OTLP 出口见 roadmap / `[otel]`

## 5. 校验

- [x] `llman sdd validate c1475-add-otel-export --strict --no-check`
- [x] `just lint` / 相关单测（apply 阶段全绿）

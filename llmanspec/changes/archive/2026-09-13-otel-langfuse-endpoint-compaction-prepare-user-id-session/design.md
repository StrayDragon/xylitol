# Design — OTEL / Langfuse 采集盲区

## (a) endpoint 未生效的产品可见性

- **捕获**：`otel.rs` install 路径两处设置进程级诊断（`OnceLock<String>`）：
  resolve 失败（exporter=otlp-http 且缺 endpoint/env）→「missing [otel].endpoint /
  LANGFUSE_* env」；build 失败 →「exporter build failed」（不含错误详情以防密钥经
  错误链泄漏）。exporter=none 不设置。纯函数 helper 便于单测（OnceLock 进程级不可重置）。
- **传输**：复用 `LoadedResourcesSnapshot`（in-process 组装直读；remote 经 unary
  `Command::LoadedResources` / `session/resources` 推送自动透传，serde 缺省兼容）。
  不新增 XyEvent 变体 / RPC 命令（无既有事件可载，新事件面变更过大）。
- **呈现**：loaded-resources 卡 mcp 块之后 `obs: <diag>` 单行（warning 色，与 mcp
  success 色区分；卡换行规则沿用 field_lines）。启动先画卡、obs 行随后出现不违反
  atc18 时序（mcp connecting label 先例）。

备选否决：`/session` stats 加键——按需命令非启动可见；footer/status——atc26 全禁。

## (b) prepare 早退可观测

- **形状**：`agent.compaction.skipped` 独立轻量 span（token.estimate 模板）：
  `Span::root` + turn parent（可用时）+ `langfuse_session_properties_from`（会话身份）
  + `skip_reason`；**不用** `langfuse_observation_properties_from`（其含 llm lane）；
  单 lifecycle event 后 drop。
- **发射点**：orchestrator 两处 prepare 门闸——`run_auto_compaction`（`Ok(false)` 早退前）
  与 `compact`（manual 早退、发 CompactionEnd 错误之外补 span）。`NoActiveSession`
  （compact_ops guard，无会话身份）不发射。`compaction disabled` 配置闸不算早退，不发射。
- **命名合规**：otel19 保留 `agent.compaction` 予过 prepare 尝试；otel22 禁门闸早退伪装
  LLM 语义 span——新名 + 无 lane 同时满足。

## 测试 seam

- otel.rs：纯函数诊断 reason 单测（exporter=none → None；缺 endpoint → Some；build 失败 → Some）。
- obs.rs：`SpanCollectScope` 单测——skipped span 名/skip_reason/session id/无 lane；闸关 → 无 span。
- loaded_resources.rs：obs_diag Some → 卡含 obs 行；None → 无该行（沿用既有渲染单测风格）。
- 复用既有 harness（SpanCollectScope / 渲染单测），不新发明 seam。

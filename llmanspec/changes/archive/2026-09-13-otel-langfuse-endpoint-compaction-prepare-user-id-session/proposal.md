---
depends_on: []
needs_specs_change: true
branch: obs/otel-blindspots
base_sha: 2c9cfec6224b6a80a9c636dc39cace21f64845d4
base_branch: main
---

# OTEL / Langfuse 采集盲区修复

## Why

2026-09-13 排查「langfuse 收集对不对」时发现两类盲区：

**配置层（最痛）**：`[otel]` 配了 `exporter: otlp-http` 但缺 endpoint 与认证、环境亦无
`LANGFUSE_*` 兜底时，OTLP 通道**静默关闭**（`otel.rs` resolve 失败连 warn 都没有，
build 失败仅文件日志 warn）。用户在 Langfuse UI 侧「什么都没有」，无从知道通道没开。

**代码层**：`agent.compaction` 只在通过 `prepare_compaction` 后导出（otel19 明文钉死早退
MUST NOT 用该名导出）——prepare 门闸早退（Already compacted / 无可摘要历史等）在 Langfuse
完全无痕。排查「为什么没压缩」时观测通道无证据；auto 路径早退更是完全静默（不发事件）。

## What Changes

- **otel3 扩句**：exporter 请求 otlp-http 而通道未生效时，该事实 MUST 经
  `LoadedResourcesSnapshot.obs_diag` 在 loaded-resources 卡以单行 obs 诊断呈现
  （无密钥、无完整 env）；exporter=none 不提示。
- **atc18 扩句**：snapshot 携带 `obs_diag` 时卡 MUST 追加一行 obs 诊断（与 mcp 失败诊断
  同形）；absent 不占行。
- **新 otel27（otel-compaction-skipped-span）**：prepare 门闸早退且会话已知时导出
  `agent.compaction.skipped` 轻量 span（skip_reason + 会话身份）；MUST NOT 用
  `agent.compaction` 名、不带 llm lane、不发 CompactionEnd（auto）、闸关零开销。
  由单测覆盖（capability 惯例：MUST NOT 为静态存在性扩 BDD step）。

## 延后（不在本 change 范围）

- `langfuse.user.id` 引入与否：产品决策（个人 harness 的 user 维度价值、来源），另立 change。
- `session_name` 进程槽多 writer 串名验证：需真实多会话 writer 场景证据，另立调查。
- `observation_io` 默认 none 的显式说明：文档性，随需要再做。

## Capabilities

- `infra-otel`（otel3 扩句、otel27 新增）
- `app-tui-fixed-zone`（atc18 扩句）

## Impact

- 代码：`src/infra/observability/otel.rs`（未生效原因捕获 + 进程级诊断槽）、
  `src/app/core/driver/types.rs`（`LoadedResourcesSnapshot.obs_diag` 字段）、
  `src/app/core/driver/in_process/mcp.rs`（snapshot 组装读取诊断槽）、
  `src/app/tui/widgets/loaded_resources.rs`（obs 行渲染）、
  `src/agent/compaction/obs.rs`（`export_skipped`）、
  `src/agent/compaction/orchestrator.rs`（auto/manual prepare 早退两处发射）。
- 行为：OTLP 配错时用户启动即见「obs: …未生效」一行；Langfuse 中压缩早退可查。
- 兼容：snapshot 新字段 serde 缺省，remote 旧客户端安全；无 wire 命令变更。

## Open Questions

- obs 诊断行颜色（warning 色与 mcp success 色区分）——实现时按 palette 既有 token 定。

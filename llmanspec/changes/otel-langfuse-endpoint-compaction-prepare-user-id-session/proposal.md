---
depends_on: []
needs_specs_change: true
---

# OTEL / Langfuse 采集盲区修复

## Why

2026-09-13 排查「langfuse 收集对不对」时发现，当前采集存在配置层与代码层两类盲区：

**配置层（最痛）**：用户配置 `~/.config/xylitol/config.yaml` `[otel]` 段只有
`exporter: otlp-http`，**缺 endpoint 与 headers**，环境亦无 `LANGFUSE_*` 变量兜底 →
OTLP 通道**静默关闭**（仅一条 warn 日志，`src/infra/observability/otel.rs:26-33`、
`otel.rs:273-279`）。用户在 Langfuse UI 侧看到「什么都没有」，误以为是采集实现坏了。

**代码层**（对照 `llmanspec/specs/infra-otel/infra-otel.feature`）：

1. 全仓从不写 `langfuse.user.id`（resource 只建 `service.name` / `deployment.environment`，
   `otel.rs:207-225`），Langfuse 侧无 user 维度。
2. `agent.compaction` 只在通过 `prepare_compaction` 后导出（`src/agent/compaction/obs.rs:27-33`）；
   prepare 早退（already-compacted / 无 active session 等）在 Langfuse 无痕——排查
   「为什么没压缩」时观测通道无证据。
3. `agent.turn` 的 `session_name` 取自进程槽（`react/mod.rs:886-891`），host 服务端多 writer
   交错时可能串到「最后 bind 的 writer 会话」的名字（id 不串、name 可能串；otel24/otel26 只约束 id）。
4. idle 路径 `token.estimate` 等 span 走观测槽回退，槽从未被 writer bind 时整条 span 无
   `langfuse.session.id`。

另：`observation_io` / `tool_observation_io` 默认 none（无 prompt/completion 内容）是产品约定
（otel4/otel7/otel8），非 bug，但常被误判为「采集缺失」，值得在欢迎卡 / 文档里更显式。

## What Changes

- **endpoint 缺失显式呈现**：`[otel]` exporter 已配置但 endpoint 与环境变量均缺失时，
  在 TUI 内以一次性轻提示呈现（文案落点遵循 `docs/architecture/TUI信息呈现与固定区词汇.md`），
  不再只有文件日志 warn。
- **compaction prepare 早退可观测**：prepare 早退时导出轻量标记（span 或 span event，
  带 skipped reason），MUST NOT 携带摘要全文（遵守既有 otel-compaction 隐私边界）。
- **user id（可选，待决策）**：若引入，确定来源（config 字段 / git user.email / 拒绝引入）与
  属性键（`langfuse.user.id`），补 infra-otel scenario。
- **session_name 槽位语义复核**：验证多 writer 交错下 `agent.turn` name 串号是否真实可达；
  可达则 turn span 的 name 改取 run 绑定快照。

## Open Questions

- `langfuse.user.id` 是否引入：个人 harness 场景 user 维度价值多大；若引入来源取什么。
- endpoint 缺失提示的落点（welcome 卡 / footer / status 条）与触发时机（启动一次 vs 每次 turn）。
- 早退标记的形状：独立短 span（parent 挂活跃 turn）vs 既有 `agent.compaction` span 的
  属性变体——前者查询友好，后者不增加 span 量。

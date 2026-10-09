---
depends_on: []
needs_specs_change: true
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# 观测身份 per-session 所有权（Phase B）：移除 obs_slot_writes 折衷，多会话各归各会话

## Why

观测「会话身份」在 materialized 路径上仍依赖**进程级槽**（`set_obs_session`/`obs_session_context`）：`bind_session` 每绑定一次都写全局槽（reader 需要 `obs_slot_writes(false)` 折衷防止 stomp，r1483/otel25）。实测代码（c2590 / otel24）已将 **generate/turn/tool 的归因改为显式快照**——进程槽只剩三处真消费：
1. `react` turn 起点的 `session_name`（react/mod.rs:899 直接读槽）；
2. `session_ops::obs_session_snapshot()` 的 name（session_ops.rs:72）；
3. optionless/unknown 请求的默认归因回退（`provider::trace::ProviderRequestTrace::start`、`attribution::merge_opencode_attribution` 的 None 臂、remote_count）。

事实：**name 是唯一跨会话从槽窃取的字段**（多会话 Host 下 last-writer-wins 把名字贴到别的会话 span 上）；`obs_slot_writes` 布尔门是 hotfix 而非结构。

## What Changes

把「会话身份事实」收敛为**运行时（capabilities）自持的所有权**，全局槽降级为「默认身份」（optionless 回退专用，仅由显式 writer 事件更新）：

1. `AgentCapabilities` 增自有 `session_name` 事实 + setter；`set_session`/`switch_session` **不再写全局槽**；`obs_session_snapshot()` 与 turn 起点用自有事实（不再读槽 name）。
2. **删除 `obs_slot_writes` 布尔门**（capabilities 字段/方法、react passthrough、host `new_reader_driver` 的 `set_obs_slot_writes(false)`、session.rs 的 gate 条件）。
3. 全局槽仅由**显式 writer 事件**更新：rename（`set_session_name` / active 分支 `set_session_name_for`）与 bootstrap 会话恢复——不设布尔门，靠调用点结构保证（reader 绑定/切换不触达这些路径）。
4. 槽成为纯「默认身份」：文档标注仅 optionless/remote-count/测试兜底消费，materialized 路径零依赖 → 多会话 span 各归各会话。
5. 收集侧 per-owner 隔离：`SpanCollectScope::records_for(session_id)`（按 `xylitol.session.id` 过滤）；otel BDD fixture 断言只针对本场景 SESSION_UUID 的记录 → 并行 CI 下外来 span 不再污染断言（与 #1 相同不变式）。

## Capabilities

- `xylitol-ai-bridge`（`provider::trace` 收集过滤、`provider::obs_session` 语义收窄）
- `agent`（capabilities session 事实、react turn 归因）
- `app`（driver session 改名路径、host reader 装配、bootstrap 恢复）
- `test-infra`（otel BDD fixture 按 owner 断言）

## Impact / 风险

- **不倒退**：generate/turn/工具归因不受影响（早已快照化）；optionless 回退语义「最近显式 writer 事件」与 otel25 前相当（不再被 reader 反复 stomp，反而更稳）。
- 单会话嵌入（desktop/库）仍正确（writer 路径不变）。
- 步骤化小提交 + 全量门禁；BDD 并行稳定性为本 change 显式验收。
- 交界测试需随迁：`reader_driver_switch_session_does_not_stomp_obs_slot`（otel25 测试）改写为「switch 根本不写槽」的结构性断言；otel7 改名步骤改走产品改名路径。

---
depends_on: []
needs_specs_change: true
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# 观测身份与激活的去全局态（per-session 所有权）逐步重构 + BDD 收集隔离

## Why

低频观测（fastrace/otel 族）的「会话身份、激活开关、collect sink」当前是**进程级全局 slot**（`set_obs_session` / `set_provider_trace_active` / `set_observation_io_tier` / `SPAN_SINK`）。三处真实代价：

1. **多会话 Host 语义错误**：served 拓扑下多 session 共享进程，`bind_session` 各写者轮流写全局 obs 槽（r1483 已用「reader 不得 stomp」打补丁证明这是已知痛点）；跨 session 的观测符号混在一条时间线。
2. **BDD 并行干扰（实证）**：`SpanCollectScope` 的独占锁只串行「同为取 scope 的测试」，串行不了「enable 观测但不取 scope」的测试（`steps_app_tui_host` 即时文件日志场景）。并行时外来 span 写入当前 scope 的 sink + 全局会话槽被并发改写 → `test_otel_r1484_dual_identity` 偶发失败（`--test-threads=1` 全绿）。
3. **未来任何 enable 观测的测试都会复发**（维护性差）。

## What Changes（分阶段，每阶段独立可校验）

### Phase A（near-term，本次）：per-turn 身份归因 + scope 按会话过滤
- `SpanCollectScope` 增 `records_for(session_id)`：按 `xylitol.session.id`/`langfuse.session.id` 过滤收集记录——先验条件：**turn 身份不再从进程槽现读，而是经显式快照**（`AgentTurnSpan::start_with_session(&ObsSessionContext)` 已存在，检查 `bind_session` 之后的 obs 槽读点全部收敛到绑定时的快照，杜绝并发改写导致的张冠李戴）。
- OTel BDD fixture 改用它，断言只对自身 SESSION_UUID 的记录生效——并行干扰在 content 层被隔离，无需串行化、无需枚举污染者。
- 单测 + BDD：并发放大（多测试并行污染窗口）下 `dual_identity` 稳定绿。

### Phase B（follow-up，独立 change）：激活与身份收敛为 per-session 所有权
- `xylitol-ai-bridge` 观测 slot 从进程单例改为**会话/driver 显式持有**（obs context 注入 run/tool/compaction 链），进程级 getter 仅保留为兼容回退。
- Host reader 的 `set_obs_slot_writes(false)` 补丁可随之移除（r1483 语义升级为 per-session 事实，不再是「不 stomp」折衷）。
- 多会话观测各归各会话（Langfuse 会话归属正确）。

## Capabilities

- `infra-otel` / `test-infra`（BDD 观测隔离）；Phase B 触及 `xylitol-ai-bridge`（provider trace slot API）。

## Impact / 风险

- Phase A：产品行为零变（只改观测归因路径与测试断言），低风险。
- Phase B：触及 product obs 槽 API 与 Host 组合根，中等风险；分独立 change（**intra 谨慎处理**：先 A 立基线，B 单独走 propose→apply→verify）。

## 背景事实（c2841/c2842 调研期收集）

- 干扰机制实证与分层方案：`llmanspec/changes/c2842-…/research/bdd-otel-parallel-interference.md`。
- 本次调研补充：turn 已有 `ObsSessionContext` 快照（`AgentTurnSpan::start_with_session`），Phase A 的收紧是局部且可验证的；`set_obs_slot_writes`（react/mod.rs otel25）即 r1483 折衷点，Phase B 的移除判据。

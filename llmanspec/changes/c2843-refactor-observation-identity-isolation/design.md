# 设计：观测身份与激活的 per-session 化（分阶段）

## 问题本质

观测链路有**三类全局态**，干扰测试并行性与多会话正确性：
1. 身份槽 `obs_session_context()`（进程单例）；
2. 激活闸 `provider_trace_active()` / `observation_io_tier()`（进程单例）；
3. collect sink `SPAN_SINK` + `SPAN_COLLECT_EXCL`（进程单例，已有一把只互斥 scope 持有者的锁）。

「独占锁只串行 scope 持有者」不是 bug，是设计遗漏：未取 scope 的 enable 者会把 span 写进当前活跃 sink。串行化（P2「枚举所有 enable 者」）要维护名单 → 维护性差；唯一根治方向是**让观测内容自带归属**，使收集可按归属过滤（P1+P3 合并），并对产品多会话语义正确。

## Phase A（本次）：turn 身份归因收紧 + scope 按会话过滤

择取点（已存在，非新机制）：
- `AgentTurnSpan::start_with_session(_, _, &ObsSessionContext)` 已支持显式快照；`react/mod.rs` 的 `set_obs_slot_writes` 证明身份来源点可开关；
- `SpanCollectScope.records()` 无过滤，加入 `records_for(session_id)` 是纯增量。

关键不变量：**在 A 完成后，本场景产生的观测 span 必带本场景 SESSION_UUID，外来 span 必带其他会话 UUID**；由此 `records_for(SESSION_UUID)` 的断言是确定性的，与并行调度无关。

本轮实施范围：
- 检查并收紧 `bind_session` 后各 span 起点的身份读取，使 turn/iteration/compaction 关联到**绑定时的快照**而非进程槽现读（若已如此则仅补断言面）。
- `SpanCollectScope::records_for(session_id)` + OTel BDD 迁移到它。
- 新增并行放大探针（多个 enable-观测场景并发）在 CI 下对 A 生效性给出可重复证明。

跳过项：激活闸与 sink 的 per-session 化 → Phase B。

## Phase B（后续独立 change）：激活与身份 per-session 所有权

- ai-bridge 观测槽 API 改为「显式 holder」：driver/组合根在 bind_session/装配时向运行链注入 `ObsSessionContext` 与激活档，进程 getter 仅作兼容回退。
- 移除 `set_obs_slot_writes(false)`（r1483 折衷）——per-session 事实而非「不 stomp」。
- 多会话 Langfuse 归属正确（当前是最后的写者赢）。

## 验收

- A：`cargo test --all-features --test bdd`（并行）稳定全绿（尤其
  `dual-identity`），无需 `--test-threads=1` 逃生；产品观测输出（记录形状）零变化。
- B：多 session 并发观测各归各会话；BDD 依旧全绿。

## 风险与缓解

- A 若发现 turn 身份仍依赖进程槽现读，改动落在 `obs.rs` 起点注入——面小、单测覆盖（otel6/7/8/10/12 已有断言）。
- B 属「intra」重构：分独立 change、保留 A 基线、逐步替换而非一次性重写。

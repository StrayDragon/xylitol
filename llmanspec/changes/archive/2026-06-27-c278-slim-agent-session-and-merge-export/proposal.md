---
depends_on:
  - c276-hoist-shared-vocabulary-to-core
  - c277-sink-assembly-to-composition-root
---

# c278-slim-agent-session-and-merge-export

> **状态**：draft 提案（2026-06-27）。c275 路线图第三项（收尾）。依赖 c276（export 类型已上提 core）
> 与 c277（SessionManager 已改 port 注入）。

## Why

c275 review 发现两个结构性问题：
1. **export 逻辑跨层重复**——`infra/session/export/mod.rs`（render_html/render_jsonl/parse_jsonl 真实现）
   + `agent/session/export.rs`（纯转发包装，8 处生产 import 全是 `crate::infra::session::export::*`
   的一跳中转）。agent 层这个 wrapper 无存在价值。
2. **`agent/session` 过载**（1,372 LOC / 8 文件）——AgentSession 同时持有 model registry、tool registry、
   持久化、compaction、stats、steering、bash exec、export，正是 HC-2 警告的"agent 绑死 Session 生命周期"。

## What Changes

### P1 — 合并 export 转发层

- 删除 `agent/session/export.rs`（5 处 `infra::session::export::*` 转发）。
- 调用方（AgentSession）直接调用 `infra::session::export`，或由组合根装配时注入。
- 对应 5 项白名单条目删除。

### P2 — SessionManager/EventBus 具体持有 -> port（从 c277 retag 过来，7 项）

- `SessionStore` port 扩容：新增 `load`（原始 entries）/`build_session_context`/
  `get_tree` 等方法（compaction/export 当前用 SessionManager 特有 API）。
- `AgentSession.session_manager` + `facade` + `compaction/{mod,orchestrator}` +
  `session/export` 的 `SessionManager` 具体持有 -> `Arc<dyn SessionStore>`。
- `EventBus` 具体持有（`session/{mod,events}`）：emit 流从 sync
  `event_bus.emit_lifecycle` 迁到 async `sink.emit`；移除死 API `subscribe()`/
  `event_bus()`/`unsubscribe()`（零外部调用者）。
- 对应 7 项白名单条目删除。

### P3 — AgentSession 瘦身

- 评估将非编排职责（stats / bash_exec handler）从 `agent/session` 剥离到 infra 或组合根。
- 保留 AgentSession 为"编排状态持有者"（当前 model/tools 选择 + 生命周期事件），符合 HC-2。

## Capabilities

- `agent-session`（modify）：界定 AgentSession 的职责边界（编排状态为主，不持后端实现）

## Impact

- **白名单收缩 12 项**（5 export + 7 SessionManager/EventBus）——arch_guard 趋近“全量扫描、零豁免”终态。
- **agent/ 瘦身**：`agent/session` LOC 下降，职责单一化。
- **零行为变更**：export 函数签名不变，仅去转发层；BDD 不受影响。

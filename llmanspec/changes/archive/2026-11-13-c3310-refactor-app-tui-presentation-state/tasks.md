---
depends_on: []
title: "Consolidate app-tui presentation state"
needs_specs_change: false
---

# Tasks — c3310 app-tui 呈现层状态收敛

每条完成即勾选并跑验收命令；全绿后 validate --strict + just qa。T1-T4 串行；T5→T6 有依赖；其余可并行。

- [x] T1 D6 块按 id 定位 + Cancelled 收窄：Compaction 补 id；model.rs 改写仅 Pending。验收：`cargo nextest run -p xylitol-app-tui` bridge 相关绿。
- [x] T2 D7 压缩块单一 upsert 入口（复用 T1 块 id，废 tokens_before 判重）。blocked_by: [T1]
- [x] T3 D8 mcp_tools_pending 改 ReadyState 分解 + 真值表单测。blocked_by: []
- [x] T4 D9 Cue 枚举 + status_busy 从 UiPhase + recalc_busy 去字面量。blocked_by: [T3]
- [x] T5 D11 ActivityFold 单一真相：VisibleItems 派生化、删 effective_level 别名。blocked_by: []
- [x] T6 D10 ScrollbackFold defaults+overrides 单一结构，visible 字段由 ActivityFold 单向流入。blocked_by: [T5]
- [x] T7 D12 PanelSubstate 枚举 + set_loading 复位子状态。blocked_by: []
- [x] T8 D13 EXIT_REQUESTED 收进 session 生命周期或补复位。blocked_by: []
- [x] T9 D18 侧栏拖拽补收尾路径 + Drag 边界检查。blocked_by: []
- [x] T10 D20 StageNamePhase 枚举，aborted 改从 AgentEnd reason 取。blocked_by: []
- [x] T11 D22 附件单槽 PendingSubmit，gated 为状态。blocked_by: []
- [x] T12 D24 脏位查询/消费分离，消费收敛 paint 单点。blocked_by: []
- [x] T13 D25 AskOutcome 类型化，非 JSON 显式 Unknown。blocked_by: []
- [x] T15 收口：`llman-sdd validate c3310 --strict` 绿、`just fmt && just lint && just test && just qa` 绿（WASI_SDK_PATH 导出）。

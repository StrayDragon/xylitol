---
title: 线协议载荷断言补齐与测试接缝可见性收口
depends_on: []
needs_specs_change: true
branch: sdd/c3402-refactor-wire-payload-and-test-seam-hygiene
base_branch: main
base_sha: 3a866eccb5a63ee13854115e6d218e583b46b6b8
---

# 线协议载荷断言补齐与测试接缝可见性收口

## Why

三项来自架构审查（llman-sdd-arch-review）的低风险卫生项，各自有可定位证据：

- **C15 · wire 载荷锚测缺口**：`every_domain_variant_round_trips` 的 23 个样本不含 `XyEvent::WorkflowUpdated`（`crates/xylitol-wire/src/event.rs:374-459`，`assert_eq!(cases.len(), 23)` 于 :459）。该变体走 `serde_json::to_value`/`from_value` 通用 reshape（:612-620、:646-652），载荷是嵌套结构（`Workflow` 含 `Vec<WorkflowTask>`，任务含可选 `sub_workflow`，见 `crates/xylitol-plugin-sdk/src/session/workflow.rs:76-114`），无逐字段断言时字段漂移不会被发现。`protocol-app` 对该事件零条款（深挖二次更正：生产方合约在 `agent-workflow` r2966 与 `agent-llm-projection`，非「specs 全树零提及」）。
- **C15 · settlement 载荷**：wire 侧 `Event::ContextTokenSettlement` 携带 `generation: u64`（`crates/xylitol-protocol/src/event.rs:110`），但 `xylitol-wire` 序列化时硬编码 `generation: 0`（`crates/xylitol-wire/src/event.rs:100`）。该字段在 `XyEvent::ContextTokenSettlement` 与 `ContextTokenEstimate` 中均不存在（`crates/xylitol-plugin-sdk/src/settlement.rs:30-34`、`crates/xylitol-plugin-sdk/src/dto/chunk.rs:72-80`），runtime 无写入点，反序列化侧亦不消费——即全链路恒为 0 的死字段。
- **C16 · bridge 家族路由**：`ToolExecutionEnd` 到屏幕的路径跨 8 个文件（wire 映射 → serve 分派 → remote driver → host 事件扇入 → `bridge::apply_xy_event` 四家族依次尝试 → `handlers/tools.rs` → `tool_projection.rs` → layout 渲染）。`bridge::apply_xy_event` 以四路 `||` 短路尝试定位家族（`crates/xylitol-app-tui/src/bridge/mod.rs:27-36`），读者须逐文件确认事件归属。
- **C17 · 测试接缝长在生产 pub 面**：`crates/xylitol-runtime/src/driver/in_process/mod.rs:448/460/473` 三个 `pub fn *_for_test` 无 `#[cfg(test)]` 门；`crates/xylitol-serve/src/occupancy/host.rs:535/539` 两个 `pub fn *_for_test` 同样无门，且仓内无任何调用点。

## What Changes

- **C15-a**：为 `WorkflowUpdated` 增加逐字段锚测（嵌套任务、可选字段、子工作流），并纳入 `every_domain_variant_round_trips` 样本集；同步在 `protocol-app` 增加该事件载荷的往返条款。
- **C15-b**：删除 wire `Event::ContextTokenSettlement.generation` 死字段，并同步 `crates/xylitol-protocol/src/event.rs` 定义与既有测试样本；`protocol-app` 条款改为不依赖该字段。
- **C16**：在 `bridge` 家族路由处补一张事件→家族→处理函数→渲染落点的注释导航表；不改任何运行时行为。
- **C17**：五个 `*_for_test` 方法全部保留（实测均有仓内调用点），只补 doc 注释注明其测试接缝性质与调用方；不改可见性、不挂 `#[cfg(test)]` 门、不删除。

## Capabilities

### Modified Capabilities

- `protocol-app`：`Event 变体完整`（r1698）与新增 `WorkflowUpdated` 载荷条款；settlement 载荷描述去除死字段。

## Impact

- 代码：`crates/xylitol-wire/src/event.rs`、`crates/xylitol-protocol/src/event.rs`、`crates/xylitol-app-tui/src/bridge/mod.rs`、`crates/xylitol-runtime/src/driver/in_process/mod.rs`、`crates/xylitol-serve/src/occupancy/host.rs`。
- 线协议：删除一个恒零字段属载荷收缩。按 `llmanspec/AGENTS.md`「破坏性合约变更」，须评估升级路径（字段为必填 `u64`，旧客户端载荷缺失将解析失败）。
- 测试：wire 单测、protocol 单测、serve/runtime 相关套件。

## 事实更正（相对审查条目）

审查条目与代码/spec 现状存在三处出入，本提案按代码事实记录：

1. C15 称「未知 cache 值降级策略未写明」——**不成立**。`protocol-app` r1698 已写明「未知字符串 须按 unknown，不得 panic」（`llmanspec/specs/protocol-app/protocol-app.feature:37`），代码 `prompt_cache_from_wire`（`crates/xylitol-wire/src/event.rs:342-348`）已实现，单测 `settlement_prompt_cache_tri_state_round_trips` 已覆盖 `"mystery"`（:516-532）。本项**不纳入**改动。
2. C15 称「generation: 0 硬编码语义待确认」——**已确认**：无生产者、无消费者，为死字段。
3. C17 称「小卫生」——**不成立**。实测五个 `*_for_test` 全部有仓内调用点，无一项是零调用：
   - `agent_cwd_for_test`（runtime `in_process/mod.rs:448`）：经 `OccupancyEngine` trait 被外部集成测试 `tests/occupancy/assembly_fixture_tests.rs:1584` 调用。
   - `tool_names_for_test`（`:460`）：被 `crates/xylitol-kernel/src/composition.rs:202,221,240,272,282,338,358,380` 调用。
   - `system_prompt_for_test`（`:473`）：被 `crates/xylitol-kernel/src/bootstrap_tests.rs:48,63,87,90,109,394` 调用。
   - `mark_run_inflight_for_test`（serve `host.rs:535`）：被 `tests/bdd/steps_server.rs:3419,3444,4133` 与 `tests/occupancy/assembly_fixture_tests.rs:1823,1841,1849,1866` 调用。
   - `flush_pending_runtime_for_test`（serve `host.rs:539`）：被 `tests/occupancy/assembly_fixture_tests.rs:1842` 调用。

   结论：`#[cfg(test)]` 对跨 crate 调用不生效（cfg 只在被测 crate 自身测试编译时激活），且这些方法被外部集成测试与 BDD 步骤实际依赖。C17 范围收窄为**保留并注明测试接缝性质**，不做删除、不挂 cfg 门。

## Open Questions

### 已定（2026-11-20 深挖）

- **C15-b 破坏性评估（已定）**：`generation: u64` 无 `#[serde(default)]`（`crates/xylitol-protocol/src/event.rs:110`），全仓无 `deny_unknown_fields`（`rg deny_unknown_fields crates/xylitol-protocol/src/ crates/xylitol-wire/src/` 零命中）。因此：
  - 旧载荷携带 `generation` → 新代码反序列化**成功**（未知字段忽略）。
  - 新载荷缺失 `generation` → 旧客户端反序列化**失败**（必填缺失）。

  但本仓无已发布跨版本客户端；协议版本经 `PROTOCOL_VERSION` 与 `host.describe` 协商，不承诺旧载荷兼容。按 `llmanspec/AGENTS.md`「版本化命名」，字段级演进走可选字段与 `Unknown` 降级。删除恒零必填字段属净收缩，**接受**。
- **C16 注释导航（已定）**：按 `llmanspec/AGENTS.md`「spec 约束层级」，注释属代码组织，不改变产品可观察行为，不得写成 spec 条款。**仅作代码注释，不改 spec。**
- **C17 范围（已定）**：见「事实更正」第 3 条，收窄为保留并补 doc 注释。

### 已全部定案

- **D1 `WorkflowUpdated` 载荷是否钉进 `protocol-app`？**（2026-11-20 深挖补充证据）
  - r1698 枚举已漂移：其「须覆盖全部 XyEvent 变体」列举 23 个变体，缺 `WorkflowUpdated`；`protocol-app` 全文对该事件零提及。**枚举修复两案共通**，均在本 change 落 spec（事件变体清单是产品合约非组织条款，不适用维护性直接编辑豁免）。
  - 当前零消费方：TUI bridge 吞掉该事件不渲染（`crates/xylitol-app-tui/src/bridge/handlers/tools.rs:15`）；生产方合约为 agent-workflow r2966。
  - 载荷无护栏：wire 侧 `serde_json::Value` 透传 reshape（`crates/xylitol-wire/src/event.rs:612-652`），字段漂移静默；持久化侧有插件 DAG 校验兜底，线协议侧无。
  - 选项 A（推荐）：枚举修复 + 新增载荷往返条款（对齐 r1718 形态）+ BDD 场景。零消费方时钉形状零成本；等有消费方再钉则形状已冻结。
  - 选项 B：只修 r1698 枚举（spec 仍要改，`needs_specs_change` 保持 true），载荷护栏交给 wire 单测（T1）。符合「有消费方才钉 spec」先例，避免提前固化无消费方的形状。

  **决议（已确认，用户 2026-11-20）**：选项 A。理由：载荷已是线上事实——wire 双向映射存在（`event.rs:117/:254/:612-652`）、生产方合约 r2966 要求发布事件，「零消费方」仅指端侧渲染未接；现在钉护栏成本最低（bridge 吞掉事件期间形状未冻结）；T1 锚测无论 A/B 都要做，A 边际成本低；条款形态对齐 r1718，不逐字段钉 serde 细节。

---
title: runtime 装配面与门协议同源（XyDriver 形状单一真值）
depends_on: []
needs_specs_change: false
---

# c3401 — runtime 装配面与门协议同源

## Why

「谁能驱动一个会话、以什么形状驱动」目前有三份独立真值，且已经实质分叉：

- **`XyDriver` trait 双份**：`crates/xylitol-runtime/src/driver/proto.rs:32` 与 `crates/xylitol-app-tui/src/driver/proto.rs:54`。**深挖更正方法数**：runtime `proto.rs` 62 个 `fn`（含 4 个 `run_turn` 测试），app-tui 51 个；共同方法 **48** 个。`comm` 实测分歧：
  - runtime 独有真实方法仅 `run_turn`（`proto.rs:51`；深挖二次更正——原列 10 个中其余 9 个 app-tui trait 全有：`abort` :154、`current_model` :156、`available_models` :166、`thinking_level` :168、`session_id` :170、`estimate_context_tokens` :183、`get_commands` :209、`leaf_entry_id` :211、`loaded_resources_snapshot` :225）。
  - app-tui 独有：`detach_session`、`exposes_session_store`、`bind_debug_scene`。
  - **结构差异（提案漏记）**：app-tui trait 声明为 `pub trait XyDriver: crate::dispatch::SessionCommandExecutor + Send`（`proto.rs:54`），runtime trait 只 `Send`（`proto.rs:32`）。这不是方法增删，是**继承关系不同**，收敛时须决定 sdk trait 是否带该 supertrait。
  - `LinkPhase` 枚举只在 app-tui 侧（:21-43）。
- **配套类型双份**：`driver/types.rs` runtime 16 项公开声明、app-tui 19 项；MCP 就绪判定两侧算法不同——runtime `mcp_tools_pending() -> bool`（types.rs:111-138）对 app-tui `mcp_ready_state() -> McpReadyState` 六态（:110-149）。
- **错误类型双份**：`XyDriverError` 在 `runtime/driver/error.rs` 与 `app-tui/driver_error.rs` 各一份（c3370 实测 7 变体 + 9 个共享 From 逐字相同；深挖二次更正，原称 8）。
- **翻译合约双份**：`SessionCommandExecutor` 与 `DispatchOutcome` 在 `kernel/src/dispatch.rs:13` 与 `app-tui/src/dispatch.rs:13` 各一份（c3370 实测 25 变体逐字零差异；实测行数 kernel 563 / app-tui 262）。**c3385 已定落点为 plugin-sdk，但未落地**：`rg 'DispatchOutcome|SessionCommandExecutor' crates/xylitol-plugin-sdk/src/` 零命中。

装配面同样薄（实测核对）：`crates/xylitol-kernel/src/occupancy/factory.rs:191-354` 的 `OccupancyDriver` 共 **38 个转发方法**（`XyDriver` 25 + `OccupancyEngine` 13，每方法恰 1 处 `self.inner`），接口复杂度 ≈ 实现复杂度；`crates/xylitol-runtime/src/driver/in_process/` 有 **19 个** `set_*` / `attach_*` 注入器（`mod.rs` 10 个、`prepare.rs` 9 个；含 `enable_reload_state`（prepare.rs:71）则 20），靠调用方按顺序打补丁。装配链 `assemble_driver`（`kernel/driver/in_process/mod.rs:82`）→ `assemble_driver_with_features_and_context`（`:91`）→ `assemble_driver_full`（`:133`）三层传参接力后才落地；`assemble_session_driver`（`:109`）再补 4 处 `set_*`（:118/:119/:126/:129，后两处条件式）。

**新增一个驱动方法目前要改 3 处**（runtime trait + app-tui trait + OccupancyDriver 转发），漏一处即静默漂移。c3370 research 已记录同目录 `types.rs` / `proto.rs`「放任复制 → 实质分叉」的先例。

## 范围裁定（2026-11-20 深挖）

本 change 的范围经用户裁定**收窄**：

- **做**：c3385 已定但未落地的部分——把 `DispatchOutcome` / `SessionCommandExecutor` / `DispatchRejection` 及 5 个数据类型（`ModelInfo` / `SessionState` / `CommandInfo` / `RuntimeReloadReport` / `LoadedResourcesSnapshot`）落到 plugin-sdk，删除 kernel 与 app-tui 的本地副本。这是**前置落点**，后续任何形状收敛都依赖它。
- **另开**：`XyDriver` trait 双份收敛（含 `LinkPhase` / supertrait 差异 / `McpReadyState` 六态 / `XyDriverError` 词表）与装配面加厚（`DriverSpec` / `OccupancyDriver` 删除）**不在本 change**，须另立 change。

理由：trait 收敛触及 runtime + app-tui + kernel 三 crate 的公开接口与 8695 行 `ScriptedDriver` 镜像，与前置落点混做会让回归面无法二分定位。

## 与既有决策的关系

- `c3370-refactor-dispatch-twins-shape`（已归档）：给出双胞胎全量差异与「推荐 plugin-sdk」落点结论，但范围只到 dispatch 翻译层。
- `c3385-refactor-dispatch-contract-home`（已归档）：把 `Command→Outcome` 翻译合约落点定为 plugin-sdk，Q1/Q2/Q3 已定稿。**但其决策未落地**——`rg DispatchOutcome|SessionCommandExecutor|DispatchRejection crates/xylitol-plugin-sdk/src/` 零命中，sdk 也缺 ModelInfo / SessionState / CommandInfo / RuntimeReloadReport / LoadedResourcesSnapshot 五件。本 change 必须先把该落点真正建起来。
- 本 change 是 c3385 的**相邻缺口**：c3385 管「命令怎么翻译成结果」，本 change 管「驱动本体以什么形状存在、谁来装配它」。两者共用同一落点（plugin-sdk）与同一条 `sdk → protocol` 依赖边。

## What Changes

本段已随 2026-11-20 范围裁定收窄（原 5 项中的 4 项移出、另立 change；见上方「范围裁定」），现行范围为两条：

1. **翻译合约落 plugin-sdk**：`DispatchOutcome` / `SessionCommandExecutor` / `DispatchRejection` 及 3 个辅助函数（`validate_nonempty_thinking_level` / `transport_variant_error` / `variant_name`）落到 sdk 单点定义；kernel 与 app-tui 删本地双胞胎、只 `pub use` 再导出（kernel 保留 `outcome_to_value`）。
2. **5 个数据类型落 plugin-sdk**：`ModelInfo` / `SessionState` / `CommandInfo` / `RuntimeReloadReport` / `LoadedResourcesSnapshot` 下沉 sdk；runtime（`driver/types.rs`）与 app-tui（`driver/types.rs`）删本地副本。`XyDriverError` 本体留两端（design Q3 已定暂不实施，sdk 只落极小拒绝类型）。

移出项（另立 change）：`XyDriver` trait 双份收敛（含 supertrait / `LinkPhase` / `McpReadyState` 六态）、装配面加厚（`DriverSpec` / `OccupancyDriver` 删除）、MCP 就绪状态机统一。

## 非目标

- 不改 crate 图、不新增 crate、不改依赖方向（`src/AGENTS.md` normative）。
- 不拆 `react/mod.rs`、session manager、driver 子树（AGENTS 明文「默认不为行数大拆」）。
- 不改 `xylitol-wire` 逐变体显式映射规则。
- 不动产品可观察行为：无 spec 行为变更，`needs_specs_change: false`。

## 风险

| 风险 | 处置 |
|---|---|
| 触及 4 个 crate 的公开接口，回归面大 | 分阶段落地，每阶段可独立编译与测试（见 tasks） |
| `XyDriver` 方法迁移会改动 app-tui harness `ScriptedDriver`（8695 行假件镜像全接口） | 迁移后假件只实现 sdk trait 一份，字段收敛为剧本数据 |
| c3385 决策未落地，本 change 依赖其落点 | 显式列为 tasks 前置步骤，先建 sdk 落点再收敛两端 |
| 与 r1535「无 Deref 包装」冲突 | 用 `DriverSpec` 装配期产出而非运行时 Deref；r1535 不重开 |

## Further Notes

（逐问深挖决策记录见 `design.md`；调研证据见 `research/`）

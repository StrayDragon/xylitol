---
title: runtime 装配面与门协议同源（XyDriver 形状单一真值）
depends_on: []
needs_specs_change: false
---

# c3401 — runtime 装配面与门协议同源

## Why

「谁能驱动一个会话、以什么形状驱动」目前有三份独立真值，且已经实质分叉：

- **`XyDriver` trait 双份**：`crates/xylitol-runtime/src/driver/proto.rs:32` 与 `crates/xylitol-app-tui/src/driver/proto.rs:54`。**深挖更正方法数**：runtime `proto.rs` 62 个 `fn`（含 4 个 `run_turn` 测试），app-tui 51 个；共同方法 **48** 个。`comm` 实测分歧：
  - runtime 独有真实方法：`abort`、`available_models`、`current_model`、`estimate_context_tokens`、`get_commands`、`leaf_entry_id`、`loaded_resources_snapshot`、`session_id`、`thinking_level`、`run_turn`。
  - app-tui 独有：`detach_session`、`exposes_session_store`、`bind_debug_scene`。
  - **结构差异（提案漏记）**：app-tui trait 声明为 `pub trait XyDriver: crate::dispatch::SessionCommandExecutor + Send`（`proto.rs:54`），runtime trait 只 `Send`（`proto.rs:32`）。这不是方法增删，是**继承关系不同**，收敛时须决定 sdk trait 是否带该 supertrait。
  - `LinkPhase` 枚举只在 app-tui 侧（:21-43）。
- **配套类型双份**：`driver/types.rs` runtime 16 项公开声明、app-tui 19 项；MCP 就绪判定两侧算法不同——runtime `mcp_tools_pending() -> bool`（types.rs:111-138）对 app-tui `mcp_ready_state() -> McpReadyState` 六态（:110-149）。
- **错误类型双份**：`XyDriverError` 在 `runtime/driver/error.rs` 与 `app-tui/driver_error.rs` 各一份（c3370 实测 7 变体 + 8 个共享 From 逐字相同）。
- **翻译合约双份**：`SessionCommandExecutor` 与 `DispatchOutcome` 在 `kernel/src/dispatch.rs:13` 与 `app-tui/src/dispatch.rs:13` 各一份（c3370 实测 25 变体逐字零差异；实测行数 kernel 563 / app-tui 262）。**c3385 已定落点为 plugin-sdk，但未落地**：`rg 'DispatchOutcome|SessionCommandExecutor' crates/xylitol-plugin-sdk/src/` 零命中。

装配面同样薄（实测核对）：`crates/xylitol-kernel/src/occupancy/factory.rs:191-354` 的 `OccupancyDriver` 中 **38 处** `self.inner` 转发，接口复杂度 ≈ 实现复杂度；`crates/xylitol-runtime/src/driver/in_process/` 有 **19 个** `set_*` / `attach_*` 注入器（`mod.rs` 10 个、`prepare.rs` 9 个，提案原称 20 个），靠调用方按顺序打补丁。装配链 `assemble_driver`（`kernel/driver/in_process/mod.rs:82`）→ `assemble_driver_with_features_and_context`（`:91`）→ `assemble_driver_full`（`:133`）三层传参接力后才落地；`assemble_session_driver`（`:109`）再补两次 `set_*`。

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

1. **`XyDriver` 形状单一真值落 plugin-sdk**：trait 本体、`EventStream` / `BashRun` / `LinkHealth` / `AttachObserve` / `LinkPhase` / `CommandInfo` / `ModelInfo` / `SessionState` / `TurnRequest` / `RuntimeReloadReport` / `ProjectTrustMode` / `ProjectTrustPersistReport` / `ClipboardCopyOutcome` / `LoadedResourcesSnapshot` / `McpReadyState` 全部下沉 sdk 单点定义；runtime 与 app-tui 只 `pub use` 再导出。
2. **分歧收敛为显式能力声明**：`detach_session` / `bind_debug_scene` / `run_turn` 三处差异改为 sdk 内带默认实现的方法，默认实现即「不支持」；不再靠两份 trait 各自增删方法表达差异。
3. **`XyDriverError` 与 `DispatchOutcome` / `SessionCommandExecutor` 随 c3385 决策一并落 sdk**：错误类型下沉为 sdk 单一词表 + 两端各自 `From` wrap（保留 runtime 独有 `From<RuntimeControlError/HookBlockedError>` 与 app-tui 独有 `From<ClipboardError>`）。
4. **装配面加厚，不引入 Deref**：删除 `OccupancyDriver` 的 25 行纯转发，改为由装配期一次性产出的**已装配驱动**（`DriverAssembly`）承担；`layer-architecture` r1535「进程内驱动无 Deref 包装」保持成立。20 个 `set_*` 注入器收敛为一个具名装配描述（`DriverSpec`），装配顺序由类型而非调用方记忆保证。
5. **MCP 就绪判定收敛为单一状态机**：`mcp_ready_state()` 六态为唯一真值，`mcp_tools_pending()` 改为由其派生。

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

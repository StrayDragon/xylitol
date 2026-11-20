# c3401 design — 装配面与门协议同源

本文件是 `llman-sdd-arch-review` 逐问深挖的决策记录。

**范围裁定（2026-11-20，用户决定）**：本 change 只做 **c3385 已定未落地的部分**（翻译合约与 5 个数据类型落 sdk + 双胞胎删除）。原 design 中 Q1/Q2/Q4 涉及的 `XyDriver` trait 双份收敛与装配面加厚（`DriverSpec` / `OccupancyDriver` 删除）**移出本 change**，须另立 change。

理由：trait 收敛触及三 crate 公开接口与 8695 行 `ScriptedDriver` 镜像，与前置落点混做会让回归面无法二分定位。

下文 Q1–Q4 保留为**已定但暂不实施**的设计输入，供后续 change 引用。

## 术语与形状

| 词 | 本 change 中的含义 |
|---|---|
| **门协议（gate protocol）** | 端与宿主之间的驱动接口形状：`XyDriver` trait 本体 + 其配套数据类型 + 错误类型 + `SessionCommandExecutor` 翻译合约 |
| **装配面（assembly surface）** | 把 `AgentRuntime` + store + 端口注入器 + 工具集组合成可用驱动的那一层：`assemble_*` 链、`set_*` 注入器、`OccupancyDriver` 转发壳 |
| **同源** | 形状只在一处定义，其余位置是 `pub use` 再导出，不存在第二份可独立漂移的定义 |

## Q0 前置落点：c3385 决策落地（本 change 唯一实施范围）

**事实**：c3385 已归档并定稿落点为 plugin-sdk（Q1/Q2/Q3），但 `rg 'DispatchOutcome|SessionCommandExecutor|DispatchRejection' crates/xylitol-plugin-sdk/src/` **零命中**，sdk 也缺 `ModelInfo` / `SessionState` / `CommandInfo` / `RuntimeReloadReport` / `LoadedResourcesSnapshot` 五件。kernel 仍持 `dispatch.rs` 563 行本地副本，app-tui 持 262 行副本。

**决策（2026-11-20）**：按 c3385 已定结论实施，不重开决策。

- `DispatchOutcome` / `SessionCommandExecutor` / `DispatchRejection` 落 `crates/xylitol-plugin-sdk/src/`（具体模块名实施期定，候选 `dispatch.rs`）。
- 5 个数据类型落 sdk；runtime 与 app-tui 的 `driver/types.rs` 改 `pub use` 再导出。
- kernel 删本地 `DispatchOutcome` / `SessionCommandExecutor` / `validate` / `transport_variant_error` / `variant_name`，只保留独有 `outcome_to_value`（RPC 结果序列化）。
- app-tui 删本地副本，改 import sdk。
- 错误：sdk 持 `DispatchRejection` 小类型，两端各自 `From` wrap 到本地 `XyDriverError`；`log_failure` / `detail_kind` / flatten 行为零变化。

**验证判据**：`rg 'pub enum DispatchOutcome' crates/` 仅命中 sdk 一处；`rg 'pub trait SessionCommandExecutor' crates/` 仅命中 sdk 一处。

**依赖边**：sdk → protocol 单向边（`Command` 词表）已被 `src/AGENTS.md` 允许，c3385 已确认无环。须在 `crates/xylitol-plugin-sdk/Cargo.toml` 加 `xylitol-protocol` 依赖（实测当前无此依赖）。

## Q1 落点：为什么是 plugin-sdk，不是 protocol（已定，暂不实施）

**已定（2026-11-20）**：门协议形状下沉 `xylitol-plugin-sdk`。

证据与理由：

1. `xylitol-protocol` 是信封 crate，`src/AGENTS.md` 明文「不认识端口；不认识 `XyEvent`；不得承接 plugin-sdk」。`XyDriver` 方法签名大量引用 `XyEvent` / `McpCatalog` / `CapabilitySnapshot` / `ContextTokenEstimate`，放进 protocol 会同时违反两条禁令。
2. `xylitol-plugin-sdk` 已是词表 + 端口 crate，`XyModel` / `XyTool` / `XySessionStore` / `XyEventSink` 等 16 个 port 都在 `crates/xylitol-plugin-sdk/src/ports/`。`XyDriver` 是同一族的应用协议端口，落点一致。
3. `sdk → protocol` 单向边已被 `src/AGENTS.md` 允许（Command 词表），c3385 已确认该边无环。本 change 复用该边，不新增依赖边。
4. sdk 现有依赖已含 `tokio` / `tokio-util` / `async-trait` / `futures` / `serde_json`，`XyDriver` 的异步签名与 `EventStream` 所需全部就绪，零新增重依赖。

**否决**：放 protocol（撞信封定位 + 拉 `XyEvent` 违反禁令）；放 runtime（app-tui 不得依赖 runtime，`src/AGENTS.md` crate 表明文）；两端各自保留（漂移已实测，见 Q5）。

## Q2 分歧处置：三处差异怎么收敛

`diff` 实测的 3 处实质分歧，逐项定处置：

| 分歧 | 现状 | 决策 |
|---|---|---|
| `run_turn` | 只在 runtime（proto.rs:51-56） | 进 sdk 单一 trait，带默认实现（context 为空时回落 `run`，否则 `unsupported`）；app-tui 客户端不需要实现它，默认实现即正确语义 |
| `detach_session` | 只在 app-tui（:130-132），默认 `Ok(())` | 进 sdk 单一 trait，默认 `Ok(())`；runtime 侧实现覆盖 |
| `exposes_session_store` | 只在 app-tui（:221-223），由 `session_store().is_some()` 派生 | **删除**，调用方直接查 `session_store()`；这是纯派生方法，删除验证为「复杂度直接消失」 |
| `bind_debug_scene` | 只在 app-tui（:295） | 进 sdk 单一 trait，默认空实现；属 TUI 客户端专属能力，默认空实现使 runtime 侧零改动 |
| `LinkPhase` | 只在 app-tui（:21-43） | 进 sdk；`LinkHealth` 保留并标注为「进程内驱动用的简化视图」，两者并存但都单点定义 |

**原则**：sdk trait 是**并集**，每处差异以「默认实现 = 该端原语义」表达，而非两份 trait 各自增删。这样新增方法只改 sdk 一处，两端零改动。

## Q3 错误类型：只下沉词表，不下沉 `XyDriverError` 本体

**已定（2026-11-20，沿用 c3385 Q2）**：`XyDriverError` 本体留两端，sdk 只提供 `DispatchRejection` 这类极小拒绝类型。

理由：`src/AGENTS.md`「三层契约与 `Xy*`」明文 `XyDriverError` 属「应用协议 → `xylitol-runtime`」。c3370 实测两侧 7 变体 + 8 个共享 `From` 逐字相同，但 runtime 独有 `From<RuntimeControlError/HookBlockedError>`、app-tui 独有 `From<ClipboardError>`——这是**本质差异**，强行统一会逼 sdk 拉 runtime 内部错误类型。

处置：sdk 提供 `XyDriverErrorKind` 词表（`kind()` / `detail_kind()` 已存在的分类值），两端各自保留 `XyDriverError` 结构体并实现 `From` 到该词表。`log_failure` / `detail_kind` / flatten 行为零变化。

## Q4 装配面：怎么加厚而不撞 r1535

`layer-architecture` r1535 明文：「进程内驱动必须自身实现端口，不得经 Deref 包装把调用绕过端口」。`OccupancyDriver`（factory.rs:191-354）用 25 行纯转发实现 `XyDriver`，**它没有 Deref**，所以 r1535 当前成立；但它的存在本身就是「接口 ≈ 实现」的薄模块。

**删除验证**：删掉 `OccupancyDriver`，25 行转发消失，但 `OccupancyEngine` 需要 `XyDriver` 超 trait——复杂度会以「`XyInProcessDriver` 必须同时实现 `OccupancyEngine`」的形式在 1 处重新冒出来，且 `OccupancyDriver` 还持有 `tool_features` / `driver_context` 两个装配期状态。**结论：它在扛事，值得加厚，不是纯透传。**

**决策（2026-11-20）**：改为**装配期一次成型**，分两步：

1. **`DriverSpec`**：把 20 个 `set_*` / `attach_*` 注入器收敛为一个具名结构（含 features / context / mcp ids / profile / ctx services / reload hook / slash commands / tool set）。装配顺序由 `DriverSpec::assemble()` 内部保证，调用方不再记顺序。
2. **`OccupancyEngine` 由 `XyInProcessDriver` 直接实现**，删除 `OccupancyDriver` 转发壳；`tool_features` / `driver_context` 两个状态迁入 `XyInProcessDriver` 字段（它们本就属于驱动实例）。

这样：25 行转发归零；`XyInProcessDriver` 自身实现端口，r1535 不破；装配知识集中到 `DriverSpec` 一处（局部性）；新增方法只需改 sdk trait 一处（杠杆）。

**否决**：`Deref<Target = XyInProcessDriver>`（违反 r1535）；宏生成转发（隐藏了 `OccupancyEngine` 额外方法的来源，AI 可导航性下降）；把 `OccupancyEngine` 也下沉 sdk（serve 专属，sdk 不认识会话槽）。

## Q5 MCP 就绪判定：单点六态

runtime `mcp_tools_pending() -> bool`（types.rs:111-138）与 app-tui `mcp_ready_state() -> McpReadyState`（:110-149）是同一套条件的两份实现，且算法顺序不同——runtime 先判 `mcp_connecting_label` 再判 servers，app-tui 合并为一个 `||` 后返回 `Connecting`。这是**漂移**，不是本质差异。

**决策**：`mcp_ready_state()` 六态为唯一真值，下沉 sdk；`mcp_tools_pending()` 改为 `matches!(self.mcp_ready_state(), Connecting | BootstrapPending | Arming)` 派生。runtime 侧原调用点行为保持不变（由派生式逐态核对，见 tasks T5 验证）。

## Q6 边界：装配面与门协议怎么切

打包后两者仍各有归属，不混为一谈：

| 关注点 | 归属 | 本 change 后形态 |
|---|---|---|
| 驱动**形状**（有哪些方法、什么类型） | plugin-sdk 单点 | `ports/driver.rs` |
| 驱动**实现**（ReAct、会话绑定、工具冻结） | `xylitol-runtime` `XyInProcessDriver` | 自身实现 sdk trait + `OccupancyEngine` |
| 驱动**装配**（从基线到可运行实例） | `xylitol-kernel` | `DriverSpec::assemble()` |
| 驱动**会话槽托管**（占用 / 驱逐 / journal） | `xylitol-serve` | `OccupancyEngine` / `OccupancyFactory` 不变 |
| 驱动**客户端镜像**（JSON-RPC 客户端） | `xylitol-app-tui` | 只 `pub use` sdk 形状，只写传输 |

**分界判据**：凡是「两端必须一致」的进 sdk；凡是「一端专属」的留该端并以默认实现表达。

## 测试存活判定

| 测试 | 处置 |
|---|---|
| runtime `proto.rs` 内 `Stub` + 3 个 `run_turn` 测试（:272-363） | 迁到 sdk 侧，验证默认实现语义 |
| kernel `dispatch.rs` `StubDriver` | 改为实现 sdk trait，删除本地 `DispatchOutcome` 镜像 |
| app-tui `harness.rs` `ScriptedDriver`（8695 行） | 保留，但只实现 sdk trait 一份；`Mutex` 字段收敛为剧本数据 |
| kernel `occupancy/factory.rs` 6 个 `McpSession::reload` 测试 | 全存活；`OccupancyDriver` 删除后测试改用 `XyInProcessDriver` |
| BDD `layer-architecture` r1535 | 存活（`XyInProcessDriver` 自身实现端口） |
| BDD `server-core` / `agent-runtime` 装配群 | 存活，行为零变化 |

## 与 design.md 的冲突

仓库当前无任何 `llmanspec/specs/*/design.md`（`find llmanspec/specs -name design.md` 零命中），无既有 ADR 可冲突。唯一的规则冲突是 r1535，已在 Q4 处置且不重开。

## 未决项

- `McpReadyState` 六态中 `Arming` 与 `BootstrapPending` 的精确边界以代码为准（types.rs:110-149），tasks T5 需逐态核对后定稿。
- sdk 新增 `ports/driver.rs` 后是否触发 sdk 收纳准则第 1 条（纯数据/纯函数无新增依赖）复核——本 change 零新增依赖，判为满足。

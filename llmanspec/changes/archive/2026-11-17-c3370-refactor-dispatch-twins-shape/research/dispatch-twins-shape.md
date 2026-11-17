# kernel↔app-tui dispatch 双胞胎形状对比调研

调研问题来源：本 change proposal「查证问题」1–5。方法：全文读取 + `diff -u` 机械对比，每条断言附 `路径:行号`。

## 0. 事实基线与行号漂移

- kernel `crates/xylitol-kernel/src/dispatch.rs` 全文 563 行；app-tui `crates/xylitol-app-tui/src/dispatch.rs` 全文 262 行。与 proposal 记载一致。
- proposal 记载的三写行号已漂移：`SessionCommandExecutor` impl 现值 kernel `command_exec.rs:11`、app-tui `driver/remote.rs:1601-1696`、app-tui `harness.rs:1001-1348`（proposal 写 remote.rs 1582-1671 / harness.rs 1003-1347）。双方独立演化的旁证。

## 1. 查证问题 1：两份 DispatchOutcome 与两枚 XyDriverError

### 1.1 DispatchOutcome：逐变体零差异

两份 enum（kernel `dispatch.rs:22-60`、app-tui `dispatch.rs:22-60`）经 `diff -u` 无 hunk，25 个变体逐字相同：`Aborted` / `State` / `Model` / `Models` / `ThinkingLevel` / `Bash` / `Compacted` / `SessionStats` / `ExportedPath` / `NewSession` / `SwitchedSession` / `Messages` / `EstimateContext` / `SessionTree` / `SessionTreeTravel` / `Sessions` / `SessionEntries` / `SessionName` / `Reload` / `LoadedResources` / `Capabilities` / `Empty` / `Commands` / `PluginCommandOutput` / `QueueStats`。

两文件的全部差异仅 4 处 hunk：

1. 错误导入路径：kernel `dispatch.rs:3` `pub use crate::driver::XyDriverError`；app-tui `dispatch.rs:7` `pub use crate::driver_error::XyDriverError`。
2. kernel 独有 wire 序列化层 `model_data` / `outcome_to_value` / `session_outcome_to_value`（kernel `dispatch.rs:62-160`）；app-tui 无。
3. `dispatch()` 接收者：kernel `dispatch.rs:176` `&mut dyn SessionCommandExecutor`；app-tui `dispatch.rs:76` `&mut dyn XyDriver`——app-tui 的 `XyDriver` trait 以 `SessionCommandExecutor` 为 supertrait（`driver/proto.rs:51`）。同一函数已出现两种接收者约定，属已发生的形状漂移。
4. import 排版（`BashOutcome` 具名导入 vs 全路径）。

测试模块中 `StubDriver` 与 4 个测试函数逐字双份（kernel `dispatch.rs:209-368` ≈ app-tui `dispatch.rs:102-261`）；kernel 另有 `outcome_to_value_covers_every_variant` 序列化全变体测试（`dispatch.rs:380-562`）。

### 1.2 XyDriverError：runtime 单一定义 vs app-tui 本地双胞胎

- **kernel 侧源** = `xylitol-runtime` 单一定义：`crates/xylitol-runtime/src/driver/error.rs:11`；kernel 经 `driver/mod.rs:14-16` re-export `xylitol_runtime::{XyDriver, XyDriverError, ...}`，并 re-export driver 数据类型（`driver/mod.rs:8-13`）。kernel 侧**不是**双胞胎。
- **app-tui 侧源** = 本地 `crates/xylitol-app-tui/src/driver_error.rs:10`（`driver/mod.rs:7` re-export）；app-tui 另有本地 `driver/types.rs`、`driver/proto.rs`（见 §4.3）。
- 两枚 error 经 `diff -u` 仅 3 处 hunk，**逐字共享面**：7 变体本体（`NotFound`/`Unsupported`/`InvalidInput`/`Io`/`Remote`/`Agent(XyError)`/`Message`，error.rs:12-50 == driver_error.rs:11-49）、`kind`/`detail_kind`/`log_failure`、6 个构造器、`from_opaque` 分类器（error.rs:159-208 == driver_error.rs:159-208）、`From` impl × 8（`String`/`&str`/`XyError`/`XyToolError`/`XySessionError`/`XySessionStoreError`/`XyExportError`/`XyTrustError`，全部源类型来自 `xylitol_plugin_sdk::error`——两文件 import 一致，error.rs:6-8 == driver_error.rs:5-7）。
- **差异面**（各自专属 From）：runtime 独有 `From<RuntimeControlError>`（error.rs:317-326）、`From<HookBlockedError>`（error.rs:328-332），且其 `CompactionError` 来自 crate 内部路径（error.rs:4）；app-tui 独有 `From<ClipboardError>`（driver_error.rs:316-325）。两边的 `From<CompactionError>` 实现体逐字相同且**是同一个 sdk 类型**：runtime `agent/compaction.rs:1-2` `pub use xylitol_plugin_sdk::{CompactionError, ...}`，定义在 `xylitol-plugin-sdk/src/compaction_error.rs:4`（`lib.rs:53` re-export）。

## 2. 查证问题 2：两份副本函数 diff

`transport_variant_error`（kernel `dispatch.rs:189-194` == app-tui `dispatch.rs:89-94`）与 `validate_nonempty_thinking_level`（kernel `dispatch.rs:162-169` == app-tui `dispatch.rs:62-69`）经 diff **逐字相同**（含错误文案）。`variant_name` 同逐字（kernel:171-173 == app-tui:71-73），依赖 protocol enum 的 `strum::IntoStaticStr` derive（`xylitol-protocol/src/command.rs:6`）。

生产路径调用（修正 proposal 一处）：`validate_nonempty_thinking_level` 生产调用在 kernel `driver/command_exec.rs:180`（`exec_set_thinking_level`）；proposal 引的 kernel `dispatch.rs:261` 是测试桩 `StubDriver` 内的调用。`transport_variant_error` 生产调用：`command_exec.rs:91,136,157` 三处兜底。

## 3. 查证问题 3：Command 归属与共享先例

- `Command` 定义在 `crates/xylitol-protocol/src/command.rs:8`（serde tagged + `strum::VariantNames`/`IntoStaticStr`）。protocol 依赖仅 `base64`/`serde`/`serde_json`/`strum`（`protocol/Cargo.toml:11-15`）。
- **protocol 与 plugin-sdk 互不依赖**（`protocol/Cargo.toml` 无 sdk；`plugin-sdk/Cargo.toml:11-31` 无 protocol）。runtime 也不依赖 protocol（`runtime/Cargo.toml:14-35`）；runtime 全仓 grep 无 `execute_session_command`——Command→Outcome 翻译层只存在于 kernel 与 app-tui，两者均已同时依赖 sdk + protocol（app-tui `Cargo.toml:15,18`；kernel `dispatch.rs:8-10`）。
- 共享先例（sdk 已承担跨端合约的既有证据）：
  1. `plan_message_history_travel`：定义 `plugin-sdk/src/session/tree.rs:118`、re-export `session/mod.rs:54`；runtime `driver/in_process/session.rs:156` 与 app-tui `harness.rs:1171` 双端消费。
  2. `SessionListEntry`：唯一定义 `plugin-sdk/src/ports/session.rs:20`；runtime 与 app-tui 的 types.rs 各自 `pub use xylitol_plugin_sdk::SessionListEntry`。
  3. `CompactionError`：唯一定义在 sdk（compaction_error.rs:4），runtime re-export（agent/compaction.rs:1-2）。
  4. `XyError` 错误词表族：`plugin-sdk::error`，两枚 error 文件的全部共享 From 源都在此（§1.2）。

## 4. 查证问题 4：三写的本质差异 vs 漂移

三写 = `SessionCommandExecutor::execute_session_command` 的三个 impl：

| impl | 性质 | 本质（不可合并）部分 |
|---|---|---|
| kernel `driver/command_exec.rs:11` | in-process 直调 runtime 能力 | 方法直调（`select_model`/`compact`/`export_html`…）；`SessionDetach` 无 detach 语义直接 `Ok(Empty)`（command_exec.rs:80） |
| app-tui `driver/remote.rs:1601` | unary RPC + JSON 字段重组 | `unary_cmd`（remote.rs:948）+ 约 30 个 `exec_*` helper（remote.rs:1702-2161+）；`SessionDetach` 走 `detach_session()`（remote.rs:1662-1665） |
| app-tui `harness.rs:1001` | ScriptedDriver 剧本注入 | 调用记录钩子（`models_calls` 1018、`compact_calls` 1065、`fork_calls` 1117、`travel_calls` 1164、`steer/follow_up/clear_calls` 1298-1321 等）、错误注入 `get_messages_error`（1137-1139）、`travel_overrides`（1168-1170）、`SetThinkingLevel` 支持 Level 校验（1038-1053） |

漂移（本质相同的逐字/等价重复，可合并）：

1. **transport-only 11 变体名单逐字三份**（proposal 记双份，实为三份）：`Prompt`/`Quit`/`Subscribe`/`ApproveTool`/`AnswerQuestion`/`McpCatalog`/`McpResourceRead`/`McpPromptGet`/`McpOAuthStart`/`McpOAuthCancel`/`McpOAuthStatus` → `command_exec.rs:81-91`、`remote.rs:1683-1693`、`harness.rs:1335-1345`。
2. `Abort`/`GetState`/`GetAvailableModels`/`GetCommands` 直映射三份（command_exec.rs:17-18,63-65；remote.rs:1613-1622,1669-1671；harness.rs:1010-1021,1285-1287）。
3. fork `position` 解析两种拼写：kernel `fork_position` fn（command_exec.rs:432-437）vs harness 内联 match（harness.rs:1113-1116）。
4. `session_path`→`file_stem` 解析两种拼写：kernel `session_id_from_path`（command_exec.rs:424-430）vs harness 内联（harness.rs:1123-1128）。
5. session stats JSON 形状两份：kernel `session_stats_json`（command_exec.rs:439-450）vs harness（harness.rs:1076-1085）。
6. queue stats 形状三份：kernel `queue_stats_outcome`（command_exec.rs:410-416）、harness（1278-1284）、remote `exec_get_queue_stats`（2143）。

### 4.3 双胞胎已实质分叉的先例（同构文件的现状）

- `driver/types.rs`：runtime 284 行 vs app-tui 396 行。app-tui 独有 `McpReadyState` 枚举 + `mcp_ready_state()`（runtime 侧仍是布尔 `mcp_tools_pending()`）；`QueueStats`/`SessionStats` app-tui 本地定义、runtime re-export 自 `crate::agent`；`TurnRequest` 仅 runtime 有；`estimate_from_session_entries` 签名已分叉（runtime 带 `fixed_context` 经 crate::agent::compaction，app-tui 忽略之、走 sdk 实现）。
- `driver/proto.rs`：runtime 363 行 vs app-tui 296 行。`run_turn`/`TurnRequest` 机制仅 runtime 有；`LinkPhase`/`detach_session`/`exposes_session_store`/`bind_debug_scene` 仅 app-tui 有；app-tui trait 加了 supertrait `SessionCommandExecutor`（proto.rs:51）。

结论：dispatch.rs / driver_error.rs 这两对当前仍是「冻结双胞胎」，而同目录 types.rs / proto.rs 已给出「放任复制 → 实质分叉」的实测轨迹。

## 5. 查证问题 5：候选落点形状与最小面

### 候选 A：xylitol-protocol（不推荐）

需要把 `DispatchOutcome` 全部 payload 类型 + 翻译函数 + 错误类型搬进 protocol。两条实现路径都不成立：protocol→sdk 新依赖边会把 sdk 的 rutis/tokio/libc/fastrace 等重依赖拉进信封 crate（sdk Cargo.toml:11-31）；或把 sdk 词表搬进 protocol——改动面横跨 sdk 全部 session/model/bash/capability 类型与全部消费方。且与 protocol 定位「JSON-RPC 2.0 product wire protocol」相悖（protocol Cargo.toml:7 description）。

### 候选 B：xylitol-plugin-sdk（推荐）

先例链已备（§3 四条）。所需最小形状：

1. **新依赖边 sdk→protocol**：protocol 不依赖 sdk，无环；protocol 轻（4 个依赖）。
2. **数据类型下沉 5 个**：`ModelInfo`、`SessionState`、`CommandInfo`、`RuntimeReloadReport`、`LoadedResourcesSnapshot`（`SessionListEntry` 已在 sdk，ports/session.rs:20）——这同时是 runtime/app-tui 两份 types.rs 的收敛方向。
3. **错误抽象最小面**：合约函数只需要 `InvalidInput` 形状构造；七 kind 无源版本已是两枚 error 的逐字共享面，且共享 From 源已全部在 `sdk::error`。sdk 只承载 kind 化构造（或轻量合约错误类型），`XyDriverError` 本体与其专属 From 链（`RuntimeControlError`/`HookBlockedError`/`ClipboardError`）留在 runtime / app-tui 各自 wrap。
4. **翻译合约本体进 sdk**：`DispatchOutcome`、`SessionCommandExecutor`、`dispatch()`、`validate_nonempty_thinking_level`、`transport_variant_error`、`variant_name`。
5. **两端保留**：kernel 保留 `outcome_to_value` 序列化层（dispatch.rs:62-160，仅 kernel 消费）；app-tui `XyDriver` supertrait 关系（proto.rs:51）或相应 re-export。两端现有依赖集合零变化（均已依赖 sdk + protocol）。

### 候选 C：各自保留（现状）

零改动。维持成本即 §4 的漂移清单：11 变体名单三份、直映射三份、等价逻辑双拼写两处、JSON 形状双份一处；每新增一个 transport-only `Command` 变体需同步改三处 tail。

## 推荐

候选 B。理由：跨端共享合约的四条既有先例（travel 计划器、SessionListEntry、CompactionError、错误词表）全部落在 sdk；唯一新增依赖边 sdk→protocol 方向向下、无环、代价最小；kernel 的 wire 序列化层与 app-tui 的剧本注入不被迫上移；T4（Command→Outcome 翻译合约归属）落 sdk 后，新增变体的同步点从三处收敛到一处。

## 未确认

- 无。全部断言来自上述文件的全文读取与 `diff -u` 实际输出。

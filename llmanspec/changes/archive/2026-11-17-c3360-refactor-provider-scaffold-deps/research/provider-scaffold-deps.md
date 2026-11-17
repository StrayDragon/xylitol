# provider 重复件依赖面盘点（c3360-refactor-provider-scaffold-deps / research）

查证人：research-provider-deps；方法：cmp -s 逐字节比对 + diff -u + grep 全仓消费面，全部断言带 路径:行号。

## 0. 事实基线

- 6 文件两插件**逐字节相同**（`cmp -s` 全部通过）：`provider/trace.rs` 559 / `provider/attribution.rs` 177 / `provider/obs_session.rs` 166 / `provider/reqwest_headers.rs` 49 / `dto/json_parse.rs` 45 / `wire_policy/defaults.rs` 9，合计 1005 行/份。
- 近似重复：`thinking.rs` 372 vs 353（diff +2/−21）；`wire_policy/mod.rs` 145 vs 129（diff −16）。
- 外部依赖面：两插件均直接依赖 `fastrace`、`reqwest`、`partial-json-fixer`（23:reqwest = { workspace = true, features = ["stream", "json"] }; 33:fastrace = { workspace = true, features = ["enable"] }; 37:partial-json-fixer = { workspace = true }；anthropic：22:reqwest = { workspace = true, features = ["stream", "json"] }; 24:fastrace = { workspace = true, features = ["enable"] }; 27:partial-json-fixer = { workspace = true }）；**sdk 已有** `fastrace`、`serde_json`，**没有** `reqwest`、`partial-json-fixer`（16:fastrace = { workspace = true, features = ["enable"] }; 23:serde_json = { workspace = true, features = ["unbounded_depth"] }）。

## 1. 逐件 use 图与外部依赖分层

| 文件（两份同） | 外部 crate | 插件本地依赖（实为 sdk 转发） | 直接 sdk 依赖 |
|---|---|---|---|
| provider/trace.rs:1-13 | fastrace::prelude（:4，全文 9 处，Span/SpanContext/Span::root:51）；serde_json 仅测试 | `crate::dto::{XyChunk,XyUsage}`（:6，= sdk dto glob re-export，见 §3）；`super::obs_session_context()`（:34，= sdk re-export，见 §2） | obs_gate 8 项 re-export（:8-13） |
| provider/attribution.rs:1-4 | serde_json::Value（:1）；reqwest::Url（:14,:16，仅 `is_opencode_host`） | `crate::hooks::HeaderBag`（:3）；`crate::provider::obs_session::{ObsSessionContext,obs_session_context}`（:4）——均 sdk 转发 | 无直接 |
| provider/obs_session.rs:1-7 | 无 | 无 | 全量 re-export（:1-7）+ 测试 ：9-166 |
| provider/reqwest_headers.rs:1-4 | reqwest::header::{HeaderMap,HeaderName,HeaderValue}（:1，全文 6 处）；serde_json（:2） | `crate::hooks::HeaderBag`（:4） | 无直接 |
| dto/json_parse.rs:1,11 | serde_json（:1，全文 5 处）；**partial-json-fixer**（:11 `fix_json`） | 无 | 无 |
| wire_policy/defaults.rs:1 | 无（`use super::Compat` = sdk Compat 转发） | 同左 | Compat |
| thinking.rs:1-5 | serde_json（:3，json! 宏在测试） | openai 份另依赖 `crate::provider::dialect`（:147）；anthropic 份无（crate:: 仅测试内） | ObsSessionContext/ThinkingBudgets/XyGenerateOptions（:5 re-export；定义 sdk generate.rs:30、model/thinking.rs:72） |
| wire_policy/mod.rs:1-4 | 无 | 无 | Compat（:3）；XyPromptCacheRetention（:4，**仅 openai 份**） |

## 2. sdk obs_session vs 插件 provider/obs_session

结论：**分层 facade + 消费，非重复实现**。sdk `crates/xylitol-plugin-sdk/src/obs_session.rs:1-104` 是唯一实现（slot/SCOPED/set_obs_session/obs_session_context/langfuse_*）；两插件 `provider/obs_session.rs:1-7` 是 7 行 re-export，:9-166（158 行）是插件本地单测，两份逐字节相同。kernel 经 openai 插件壳消费：`crates/xylitol-kernel/src/provider/hooks_port.rs:5`、`crates/xylitol-kernel/src/bootstrap.rs:186`、`crates/xylitol-kernel/src/driver/in_process/tests/mod.rs:2`、`crates/xylitol-kernel/src/driver/in_process/tests/session.rs:499-507`；另 re-export trace 项 10:    pub use xylitol_official_plugin_provider_openai::provider::trace::{。

## 3. XyChunk / XyUsage 与 spec 依赖分界

- 定义**只在 sdk**：`XyChunk` = crates/xylitol-plugin-sdk/src/dto/chunk.rs:12（enum）；`XyUsage` = crates/xylitol-plugin-sdk/src/dto/message.rs:234（struct）。插件 `dto/mod.rs:4` = `pub use xylitol_plugin_sdk::dto::*;`——插件 `crate::dto::{XyChunk,XyUsage}` 即 sdk 同一类型，形状**零差异**（薄留 re-export）。
- spec 分界条款原文（llmanspec/specs/agent-llm-projection/agent-llm-projection.feature:14 @req:r1559「规则: 主仓映射层」）：「主仓 agent 须经 project_for_llm（或等价）将 AgentMessage 投影为 Vec<XyLlmMessage>：Llm 臂 须 passthrough；Env 折叠策略留在 agent。XyModel 调用方 须只传递投影后的 LLM DTO。agent MAY 依赖 bridge DTO 以组合 AgentMessage::Llm；**agent 不得直接依赖 xylitol_official_plugin_provider_openai 的 HTTP/vendor SDK 类型**。」——迁 sdk 后该条款的 provider 插件路径指称需随删除清理（llmanspec/AGENTS.md「spec 维护」：删除对象的引用条款随删除一并清理）。

## 4. hooks::HeaderBag

- **唯一定义**：crates/xylitol-plugin-sdk/src/ports/http.rs:8 `pub type HeaderBag = serde_json::Map<String, serde_json::Value>`；经 sdk ports/mod.rs:30、lib.rs:115 再导出。插件 hooks.rs（两份各 3 行）= 纯 re-export 壳。
- 消费方：openai 插件 8 文件共 9 处 `crate::hooks::`（factory.rs、native/{openai_hooks_mw,openai_completions,openai_responses,openai_client,openai}.rs、provider/{reqwest_headers,attribution}.rs）；anthropic 插件 4 文件共 9 处（factory.rs、native/anthropic_messages.rs、provider/{reqwest_headers,attribution}.rs）；跨 crate 1 处：crates/xylitol-kernel/src/provider/hooks_port.rs:5。
- 另注：`plugins/xylitol-official-plugin-hooks/src/http.rs:8` 有**独立定义**的本地同名别名（形状相同、不经 sdk），不属于本盘点 6+2 件。

## 5. 两份近似重复的 diff 摘要

- `thinking.rs`（372 vs 353，+2/−21）：唯一差异 = `apply_thinking_openai_completions_with_compat` 函数体。openai 份 ：147 委托 `crate::provider::dialect::apply_completions_thinking`（定义 plugins/...-openai/src/provider/dialect/mod.rs:7 → deepseek.rs:5，DeepSeek 兼容分派）；anthropic 份同函数（:142）改 `let _ = compat;` 直调。另 openai 份多一个 ~19 行 DeepSeek thinking 格式测试（diff hunk @@ -332,26 +333,6）。性质：**方言分派差异**，非实现漂移。
- `wire_policy/mod.rs`（145 vs 129，−16）：anthropic 份缺 `XyPromptCacheRetention` re-export（openai:4）、`ExtraPolicy.prompt_cache_retention` 字段（openai:10）、Default None（openai:19）、`prompt_cache_retention()` accessor（openai:64-66）及 3 处测试断言。性质：**openai 独有 prompt-cache retention 能力位**，其余逐行相同。

## 6. 壳调用点计数

- `hooks.rs`（3 行 ×2）：`crate::hooks::` 引用 openai 9 行/8 文件、anthropic 9 行/4 文件，加 kernel 1 处（§4）。删除壳 = 改 10 文件 import 路径。
- `provider/tool_wire.rs`（1 行 ×2，`pub use xylitol_plugin_sdk::tool_wire::*;`）：限定路径调用 openai 6 处（native/openai_completions.rs:711、native/openai_responses.rs:172,927,1601、native/openai.rs:416,534）+ 两边 provider/mod.rs `pub use tool_wire::{canonical_tool_names, from_wire_tool_name, is_provider_safe_tool_name, to_wire_tool_name}`（openai:32-34 / anthropic:26-28）；anthropic 3 处（native/anthropic_messages.rs:256,922,1411）。

## 7. 迁移判定（每件：带走类型 / 阻塞依赖）

| 件 | 判定 | 需带走 | 阻塞依赖 |
|---|---|---|---|
| trace.rs | **可迁 sdk** | `GenerationFinishKind`、`ProviderRequestTrace`（持 fastrace Span）；实现仅 ：1-235，测试 :237-559（322 行）可随迁 | **无新依赖**：fastrace sdk 已有；其余引用全是 sdk 内（dto/obs_gate/obs_session） |
| attribution.rs | **可迁，1 处例外** | 无类型（5 个自由函数） | `reqwest::Url`（:14,:16，仅 is_opencode_host）→ sdk 加 reqwest dep，或该函数换手写解析，或留插件 |
| obs_session.rs | **无需迁**（已是分层） | 无 | 无；可做的是删壳，kernel 4 文件改指向 sdk |
| reqwest_headers.rs | **不可迁（现状）** | 无类型（2 个转换 fn） | `reqwest::header` 全部（:1，6 处）；sdk 无 reqwest dep |
| dto/json_parse.rs | **可迁** | 无类型（1 fn） | `partial-json-fixer`（:11）；sdk 无此 dep |
| wire_policy（defaults+mod） | **可迁** | `ExtraPolicy`、`WirePolicy`、defaults 4 const | 无；anthropic 缺 retention 位 → 统一时字段置 None 即可（行为不变，§5） |
| thinking.rs | **可迁核心** | `ThinkingAdapterKind`、`ResolvedThinking`、`BuiltinThinkingLevels` + resolve/apply fns | openai 份 compat 钩子调 `provider::dialect`（:147，dialect/mod.rs:7、deepseek.rs:5）→ sdk 持核心 + compat 分派参数化，dialect 留 openai 插件 |

## 待确认

- 无未确认事实项。决策项（非事实）：sdk 是否接受 `reqwest` / `partial-json-fixer` 两个新依赖（影响 attribution / reqwest_headers / json_parse 三件去向）。

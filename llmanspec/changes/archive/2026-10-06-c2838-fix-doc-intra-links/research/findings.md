# Research — c2838 doc intra-doc 链接治理

## 背景

c2837 将 `agent/infra` 公开化后，rustdoc 扫描面扩大，28 处既有 intra-doc 链接缺陷暴露。
c2837 以两个 crate-level `#![allow(...)]` 临时兜底；本 change 逐处修复并移除 allow。

## 实测（临时移除 allow 后 `cargo doc --all-features --no-deps`）

共 **28** 处：`broken_intra_doc_links` ×18、`private_intra_doc_links` ×10。

## 修复方案表（逐处，调研依据=源码定义定位）

### BROKEN ×18 —— 补全正确路径 / 修正

| 位置 | 链接 | 调研结论 | 修复 |
|---|---|---|---|
| agent/mod.rs:8,47 | `[AgentCapabilities]` | 短名作用域缺失；真身在 `capabilities`（pub mod） | `[capabilities::AgentCapabilities]` |
| agent/builder.rs:1,8 | `[AgentRuntime]` | 短名缺失；真身 `runtime`（pub） | `[runtime::AgentRuntime]` |
| agent/builder.rs:21 | `[crate::agent::AgentCapabilities]` | 路径缺 `capabilities` 链 | `[crate::agent::capabilities::AgentCapabilities]` |
| agent/model/mod.rs:7 | `[ModelRegistry]` | 真身 `registry` 子模块 pub struct | `[registry::ModelRegistry]` |
| agent/runtime/react/mod.rs:284 | `[HookBlockedError]` | 真身 `capabilities::hook_bus`（pub struct） | `[super::super::capabilities::hook_bus::HookBlockedError]`（以实际层级为准） |
| agent/mod.rs:47 | `[AgentCapabilities]`（同上） | — | `[capabilities::AgentCapabilities]` |
| app/core/bootstrap.rs:2 | `[AgentRuntime]` | 短名缺失 | `[crate::agent::runtime::AgentRuntime]` |
| app/core/bootstrap.rs:22 | `[BootstrapOutput::warnings]` | **真 bug：`BootstrapOutput` 不存在**；意图类型=`ResolvedAssembly`（含 `pub warnings: Vec<BootstrapWarning>`，已核对 290-330 行） | `[ResolvedAssembly::warnings]` |
| driver/types.rs:38,52,214 | `[XyDriver::…]` | trait 顶层 re-export（crate 根） | `[crate::XyDriver::…]` |
| driver/types.rs:184 | `[XySessionStore::list_sessions]` | 顶层 re-export | `[crate::XySessionStore::list_sessions]` |
| driver/types.rs:187 | `[ContextTokenEstimate]` | 经 protocol::model::meta re-export | `[crate::protocol::model::ContextTokenEstimate]` |
| app/tui/commands.rs:287 | `[id]` / `[scene]` | **markdown 误解析字面量**（非链接意图） | 转义 `[id]` / `[scene]` → `\`[id]\`` 或 code 字体 |
| layout/root/mod.rs:322 | `[render_status_slot]` | 子模块 `render` 内 `pub(super) fn` | super 可达后 `[render::render_status_slot]`（若 render 模块 pub）否则文本 |
| infra/process/shell.rs:138 | `[join_paths]` | 应为 std 路径 | `[std::env::join_paths]` |

### PRIVATE ×10 —— 策略：改文本（`code` 字体，不扩公开面）

| 位置 | 链接 | 结论 |
|---|---|---|
| compaction/mod.rs:108 | `SUMMARY_PLACEHOLDER_TOKENS` | 内部实现常量 → 文本 |
| llm_project.rs:13 / session_env.rs:6,20,42 | `status_bar` / `STATUS_BAR_XML_ROOT` | prompt 子模块/常量 crate-internal → 文本（保留语义，不扩模块面） |
| app/core/mod.rs:15 | `mcp_spec` | core 内部项 → 文本 |
| app/server/wire_v3.rs:4 | `rpc_module::dispatch_raw` | 内部函数 → 文本 |
| root/mod.rs:1036 | `TOAST_NOTICE_TTL` | commands 子模块 pub const（模块内可达）→ 文本 |
| infra/config/loader.rs:62 | `ConfigPaths::discover_with` | paths 内部方法 → 文本 |
| infra/config/resolver.rs:45 | `SHELL_CACHE` | 内部 static → 文本 |

**不做**：本次不进行「私有项升 pub 公开化」——避免再次扩大 API/doc 面引发二次涟漪；语义公开化属于独立 API 决策。

## 验证计划

1. 逐处修复后**移除两个 allow**
2. `cargo doc --all-features --no-deps` 零 warning（rustdoc lints 全隐）
3. `just qa quiet` 全绿（doc-check / doc-test 主收口）
4. `llman-sdd validate --strict` + gateChecks 全绿

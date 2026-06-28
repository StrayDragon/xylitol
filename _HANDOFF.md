# Xylitol 架构解耦路线图 — Handoff Review

> 当前日期：2026-06-27
> 最近提交：c280-wire-session-commands（实现中，尚未 commit）
> 状态：c275–c280 路线图全部完成，arch_guard 零豁免终态达成

---

## 1. 已完成路线图（c275–c280）

| Change | 提交 | 核心成果 |
|---|---|---|
| c275 tighten-arch-guard | `bf7aa96` | arch_guard 全量扫描，21 项基线 |
| c276 hoist-shared-vocabulary | `34a0714` | 共享类型上提 core |
| c277 sink-assembly-to-composition-root | `11a8b4a` | model_builder/sandbox 注入（3 项） |
| c279 relocate-executor | `a235358` | BashExecutor/SecretResolver/ResourceLoader/TrustStore port 化（7 项） |
| c278 slim-agent-session (P0) | `814e329` | **白名单 12→0**：SessionManager→SessionStore port、EventBus 删除、export 上提 core、5 死方法删除 |
| **c280 wire-session-commands** (P1) | *(本 commit)* | **兑现架构红利**：RPC SwitchSession 真实化、GetMessages 实现、Export/Import JSONL 补齐、AGENTS.md/系统提示接入 live prompt、死 slash 命令系统删除 |

### c280 详细成果

- **P1 — RPC 会话命令补齐**：`SwitchSession` 从改字符串 → `store.exists` 校验 + 缓存自动失效；`GetMessages` stub → `store.load_entries` 实现；`Command` 枚举新增 `ExportJsonl`/`ImportJsonl`（对称 `ExportHtml`）
- **P2 — prompt 接入**：CLI 从 `DefaultResourceLoader` 取 context_files/system_prompt/append_system 填入 `AgentSession.prompt_opts`；删除硬编码 fallback `"You are a helpful AI assistant."`；loader > config > None 优先级
- **P3 — 死 slash 系统删除**：`process_prompt` 的 `/command`→`Handled` 分支 + `PromptResult::Handled` 变体 + `commands.rs` 的 `is_slash_command`/`get_command_args` 删除；保留 `BUILTIN_COMMANDS` 表（RPC `GetCommands` 数据源）

---

## 2. 当前架构状态

### 分层纯净度
- **arch_guard allowlist: 0**（零豁免终态，c278 达成，c280 保持）
- **agent/ 生产区零 `crate::infra::*` import**（测试区 legal）
- **AgentSession**: 7 个 port trait object，零具体 infra 持有

### 代码健康度
- **clippy `--lib -- -D warnings`: 0**（c280 后清理完毕）
- **`just lint`**: `cargo clippy -- -D warnings`（闸门，卫生不退化）
- **lib 测试**: 503 passed, 1 skipped
- **BDD**: 88 scenarios, 0 失败
- **arch_guard**: 3/3 全绿

### 未提交工件
- `llmanspec/changes/c280-wire-session-commands/`（active change，待归档）

---

## 3. 关键技术决策记录

### 命令通道：RPC 是唯一活路径
项目有两条命令体系，但只有 RPC `Command` 枚举 → `rpc.rs dispatch` 是活的。
slash `BUILTIN_COMMANDS` → `process_prompt` → `Handled` 是**完全断开的并行死系统**
（process_prompt 在 interactive/runtime/server 零调用方）。c280 明确放弃复活死系统，
全部新能力走 RPC 通道。

### abort() 的 lifecycle 通知
c278 删除 `EventBus` 时确认：旧 EventBus 零订阅者，abort 的 AgentEnd emit 是死操作
→ 直接移除。`core::ports::LifecycleEvent` 当前仅有 CompactionStarted/Ended，无 AgentEnd/TurnEnd。
若未来需要 abort 通知，需先扩 LifecycleEvent + 在 sink.emit 中实现。
`Driver::abort` 是 sync trait method → abort async 化需单独提案。

### SessionManager 去留
c278 上提 `fork`/`create`/`load_entries`/`append_session_entry`/`build_session_context` 到
`SessionStore` port。其余管理操作（`navigate_tree`/`switch_session`）对应的 AgentSession 方法
在 c278 中被删除（零调用）。SessionManager 只作 store 后端在组合根构造。

### system_prompt 默认
c280 删除 CLI 的硬编码 `"You are a helpful AI assistant."` fallback，改为：
1. Loader SYSTEM.md（若有）
2. Config profile 的 system_prompt
3. None → `build_system_prompt` 的内置默认（"You are an expert coding assistant..."）

### ResourceLoader trait
`core::ports::ResourceLoader` 的 `dyn` 调用方在 c278 卫生清理后归零（loader-based prompt
assembly 从未接入 AgentSession）。c280 保留了 trait impl 但改为 `#[allow(dead_code)]` 标注，
等待未来 prompt 重构时重新激活。

---

## 4. 已知技术债与后续候选

### 立即可做（纯卫生）
- `tests/bdd.rs` 有 ~16 个 `mut`/unused-import 卫生 warning（`cargo clippy --all-targets` 可见，
  `just lint` 不覆盖测试 crate）

### 短期（功能补齐）
- **`Command::GetMessages` 的 BDD 覆盖**：c280 dispatch 已实现，但缺少 BDD step（`bdd.rs` glue）
- **`Command::ExportJsonl`/`ImportJsonl` BDD 覆盖**：同上
- **会话树管理 UI**：`/tree` 命令的 navigate 功能——`Command` 枚举可能需要扩展，但当前无消费方
- **server 模式的 context_files**：c280 给 server 传了 `Vec::new()`——headless 场景下资源发现
  是调用方的职责，但未来可考虑 server 自行加载

### 中期（架构）
- **SessionManager 拆分**（1404 LOC）：`SessionPersister` + `SessionTree`
- **`Driver::abort` async 化**：当需要可靠 lifecycle 投递时
- **`LifecycleEvent` 扩容**：加 AgentEnd/TurnEnd 变体
- **settings/manager.rs 拆分**（924 LOC）

### 已归入 not-planning 的
- `c60-add-model-lock`：本地 GGUF 模型抢占锁（延后）
- `c85-add-dap-layer`：DAP 调试器（paused 2026-05-17）

---

## 5. 当前状态速查

| 指标 | 值 |
|---|---|
| arch_guard allowlist | **0** |
| clippy --lib | 0 warning |
| lib 测试 | 503 passed |
| BDD | 88 passed |
| 总 LOC | ~34k (-600 vs c278 前) |
| agent/ LOC | ~6.9k |
| infra/ LOC | ~15.8k |
| specs | 43 |
| archived changes | 103 |

---

## 6. 下一步

1. **提交 c280 + 归档**：commit 当前工作 + `llman sdd archive run c280-wire-session-commands`
2. **BDD 补齐**（可选）：为 export-jsonl/import-jsonl/get-messages 写 BDD
3. **server context_files**（可选）：server 模式接入 AGENTS.md
4. 长期：继续兑现架构红利——SessionManager 拆分、LifecycleEvent 扩容

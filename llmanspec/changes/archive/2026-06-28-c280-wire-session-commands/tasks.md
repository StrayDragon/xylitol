# c280 Tasks

> 顺序执行。P1（RPC 命令）→ P2（prompt 接入）→ P3（清理死系统）。

## P1 — RPC 会话命令补齐（interactive-protocol）

- [x] 1.1 检查 `RpcState` 的 agent 缓存失效逻辑；若无 `invalidate_cached_agent`，新增（switch 后必须重建 agent）
- [x] 1.2 `SwitchSession` 真实化：`store.exists` 校验 → 切 session_id → 失效缓存 → 返回会话元信息
- [x] 1.3 `GetMessages` 实现：`store.load_entries` 返回 `Vec<SessionEntry>`，移除 stub
- [x] 1.4 `Command` 枚举新增 `ExportJsonl` / `ImportJsonl` 变体（对称 `ExportHtml`）
- [x] 1.5 rpc.rs dispatch：`ExportJsonl`→`export_to_jsonl`，`ImportJsonl`→`import_from_jsonl`（返回新 session_id）
- [x] 1.6 protocol.rs `id_for` / 相关 match 补齐新变体

  验证：`cargo check --lib --bins`；`cargo nextest run -p xylitol --lib`

## P2 — AGENTS.md / 系统提示 / skills 接入（agent-session）

- [x] 2.1 恢复 `DefaultResourceLoader` 的 inherent getter：`get_agents_files`/`get_system_prompt`/`get_append_system_prompt`（trait impl 保留）
- [x] 2.2 cli/mod.rs：复用已构造的 loader，取 context_files/system_prompt/append_system
- [x] 2.3 `AgentSession::new` 接受 `context_files`/`append_system_prompt`（与现有 system_prompt 并列），填入 prompt_opts
- [x] 2.4 移除 cli/mod.rs 硬编码 fallback `"You are a helpful AI assistant."`，改 loader 无值时传 None（由 build_system_prompt 内置默认接管）
- [x] 2.5 同步更新 facade.rs::with_ports、rpc.rs、server/runtime.rs、所有测试 helper 的调用点

  验证：`cargo nextest run -p xylitol --lib`；手测 AGENTS.md 进入系统提示

## P3 — 删除死 slash 命令系统（agent-session）

- [x] 3.1 二次确认 `process_prompt`/`is_slash_command`/`Handled` 零调用方（grep 全仓）
- [x] 3.2 `process_prompt`：删除 `/command`→`Handled` 分支，保留 `!cmd`/`/template:`/`/skill:`/PassThrough
- [x] 3.3 删除 `PromptResult::Handled` 变体（若无消费方）
- [x] 3.4 删除 `commands.rs` 的 `is_slash_command`/`get_command_args`（若仅 process_prompt 用）；保留 `BUILTIN_COMMANDS`/`get_all_commands`/`find_command`（GetCommands 数据源）
- [x] 3.5 清理因删除产生的 dead import/测试

  验证：`cargo clippy -- -D warnings`；`cargo nextest run -p xylitol --lib`

## 收尾

- [x] 4.1 新增 BDD：switch_session（真实切换）、export_jsonl、import_jsonl、get_messages
- [x] 4.2 更新 API baseline snapshot（Command 枚举 + AgentSession::new 签名）
- [x] 4.3 跑全套验证：
  ```
  cargo nextest run -p xylitol --lib
  cargo test --test bdd -- --test-threads=1
  cargo clippy -- -D warnings
  cargo fmt --check
  ```
- [x] 4.4 `llman sdd validate c280-wire-session-commands --strict --no-interactive` 通过

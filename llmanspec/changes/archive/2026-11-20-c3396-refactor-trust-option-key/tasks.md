# Tasks

## 1. 在 trust 域新增按 key 的选择与应用入口

- [x] 1.1 在 `crates/xylitol-runtime/src/trust/store.rs` 新增纯查找函数 `select_option(options: &[TrustOption], key: &str) -> Option<&TrustOption>`，**不得读取 `label`**。
- [x] 1.2 在 `TrustManager` 上新增 `apply_option(&self, opt: &TrustOption, cwd: &str, audit: &TrustAuditContext) -> Result<TrustApplied, XySessionStoreError>`（类型名以实施为准）：有 `updates` 时 `apply_updates`，否则 `record_decision` `session_only`；返回信任决策与落盘路径。
- [x] 1.3 `apply_option` 返回 `Result`，**不得内部吞错**；错误映射由调用方各自决定。
- [x] 1.4 单测：改 `label` 后 `select_option` 仍命中正确 key；`apply_option` 对「有 updates」与「仅会话」两条分支各记一次审计事件（`trust_grant` / `trust_deny` / `session_only`）。

## 2. driver 路径改为按 key 选择

- [x] 2.1 在 `crates/xylitol-runtime/src/driver/in_process/mod.rs` 新增私有映射 `fn trust_mode_key(mode: ProjectTrustMode) -> &'static str`（`TrustCwd` → `"trust"`，`TrustParent` → `"trust_parent"`，`Deny` → `"deny"`）。
- [x] 2.2 `persist_project_trust` 改为 `select_option(&options, trust_mode_key(mode))` + `apply_option`；删除 `find_trust_option`、`apply_trust_option`、`session_only_event` 三个自由函数。
- [x] 2.3 保留既有错误语义：`TrustParent` 无父目录时报 `"no parent folder to trust"`，其余报 `"trust option unavailable"`；`ProjectTrustPersistReport` 字段与 `message` 文案不变。
- [x] 2.4 既有四个 trust 驱动测试（`trust_cwd_without_reload_state_uses_agent_cwd`、`trust_parent_persists_parent_decision`、`trust_parent_at_filesystem_root_errors`、`deny_persists_untrusted_decision`）断言不变、保持绿。
- [x] 2.5 新增一条测试：篡改 `TrustOption.label` 后 `persist_project_trust` 仍命中正确选项（证明 driver 不读 label）。

## 3. 门禁路径复用同一入口

- [x] 3.1 `resolve_project_trusted` 中「选中选项之后的应用」改为调用 `apply_option`，删除内联的 `apply_updates` / `record_decision` 分支（`resolve.rs:76-85`）。
- [x] 3.2 保留 `on_prompt` 回调返回**下标**的既有契约；`apply_option` 失败仍按现状吞错（`let _ =`），不改门禁错误语义。
- [x] 3.3 既有 `resolve.rs` 单测（`test_resolve_override_trusted`、`test_resolve_override_not_trusted`、`test_resolve_no_inputs_auto_trust`、`test_resolve_stored_decision`、`test_resolve_no_ui_fallback`、`test_resolve_user_prompt_choose_trust`、`test_resolve_user_prompt_cancelled`）断言不变、保持绿。

## 4. CLI 路径复用同一入口（深挖新增）

- [x] 4.1 `src/cli/trust_gate.rs:82-107` 的内联应用改为调用 `TrustManager::apply_option`。
- [x] 4.2 保留 CLI 错误文案映射：`apply_option` 出错时仍映射为 `TrustGateError::Other(format!("trust store write failed: {e}"))`；`Denied` / `Cancelled` 分支不变。
- [x] 4.3 `src/cli/trust_gate.rs` 不再 import `apply_updates` 相关分支逻辑；`resolve_trust_answer` 仍用于 key → 下标的解析。

## 5. 回归与门禁

- [x] 5.1 `cargo nextest run -p xylitol-runtime` 全绿，记录输出。
- [x] 5.2 `cargo nextest run --all-features --test bdd` 中 `domain-security`（r73/r74）与 `app-tui-trust`（r3307；r1375「信任选择主题与取消」，绑定 `tests/bdd/bindings_app_tui_trust_extra.rs:5`）场景绿，记录输出。
- [x] 5.3 `just fmt` + `just lint`，记录输出。
- [x] 5.4 前后对比（生产代码口径）：`rg -n 'label.starts_with|label ==' crates/xylitol-runtime/src/driver` 由 2 处（mod.rs:492/:494）降为 0 处——`store.rs:1027` 测试断言按 2.4 保留为预期残留；`rg -n 'apply_updates' src/cli/trust_gate.rs crates/xylitol-runtime/src/trust/resolve.rs crates/xylitol-runtime/src/driver/in_process/mod.rs` 生产代码只剩 `apply_option` 内部一处——`resolve.rs:174` 测试内调用按 3.3 保留为预期残留。
- [x] 5.5 `llman-sdd validate c3396-refactor-trust-option-key` failed 0。

# Tasks

垂直切片按「先加后删」排序（机械迁移扫约 40 个调用点，单点删除放最后）。完成判据中的计数一律在 change 分支相对 merge-base 测量。

## T1 sdk：port 新增 provided 方法

- [ ] `crates/xylitol-plugin-sdk/src/ports/session.rs`：`XySessionStore` 新增 `async fn load_validated(&self, session_id: &str, fallback_cwd: &str) -> Result<Vec<SessionEntry>, XySessionStoreError>`，provided 默认实现 = `self.load_entries(session_id).await`；doc 注释写明语义（磁盘实现须校验存储 cwd，见 agent-session-store r1094）。
- 验证：`cargo check -p xylitol-plugin-sdk`；既有 sdk 测试全绿（无行为变化）。

## T2 session 插件：校验迁入 LogSessionStore（只加不删）

- [ ] `assert_session_cwd_exists` 与 `load_validated` 迁入 `log` 模块，`LogSessionStore` 覆写 port 方法（`manager/mod.rs:446-473` 原样迁移）。
- [ ] `LogSessionStore::exists_sync` 升 `pub`（原 pub(crate)，log/mod.rs:243）。
- [ ] 新增单测 3 条：存储 cwd 可用直过；存储 cwd 缺失且 fallback 可用救回；双缺失报错同时携带两个路径。
- 验证：`cargo nextest run -p xylitol-official-plugin-session` 全绿。

## T3 runtime：switch_session 恢复前校验

- [ ] `crates/xylitol-runtime/src/driver/in_process/session.rs` `switch_session`：`exists` 检查后调用 `self.store.load_validated(session_id, &self.agent.cwd()).await?`，错误映射保持可操作文案（`XySessionStoreError → XyDriverError` 既有 flatten）。
- [ ] 新增单测 2 条：存储 cwd 双缺失时 switch 被拒且错误含双路径；fallback 可用时 switch 成功。
- 验证：`cargo nextest run -p xylitol-runtime` 全绿。

## T4 kernel / embed / 测试群迁移 + 删除 manager

- [ ] `bootstrap.rs` `restore_session`：改 `LogSessionStore::new/in_memory`；对已存在会话调 `load_validated`（fallback 取 bootstrap cwd），失败转 `BootstrapError`（文案保留校验原文）；`exists_sync` 改用 LogSessionStore。
- [ ] 其余生产调用点改 `LogSessionStore`：`composition.rs`（含 `default_dir`）、`occupancy/factory.rs`、`occupancy/runtime.rs`、`src/embed.rs`（`LogSessionStore::in_memory`）。
- [ ] 测试调用点机械迁移：`bootstrap_tests.rs`、`driver/in_process/tests/{mod,mcp,trust,bash,session}.rs` 及其他 `rg 'SessionManager'` 命中的测试文件。
- [ ] 删除 `plugins/xylitol-official-plugin-session/src/manager/` 模块、`lib.rs` 导出行、`pub backend` 字段消费（`SessionBackend` 类型保留，log/mod.rs backend() 在用）。
- 验证：`rg -c 'SessionManager' crates plugins src tests` = 0（相对 merge-base 从约 45 处降为 0）；`cargo nextest run -p xylitol-kernel` 与 embed 测试全绿。

## T5 BDD 与步骤文本同步

- [ ] `tests/bdd/steps_session.rs`、`steps_agent_session_extra.rs`：fixture `ensure_mgr`/`mgr` 改持 `LogSessionStore`（`load_validated` 经 dyn port 调用）。
- [ ] 孤儿步骤文本改名：`infra SessionManager 实现协议端口` → `infra 会话存储实现协议端口`（含配对 when 步骤），全仓无场景引用，改名后 `cargo nextest --all-features --test bdd` 步骤映射保持全绿。
- [ ] 确认 `scripts/check_layer_invariants.py` 无需改动（预期：0 处 SessionManager/cwd 引用，实现期复核）。
- 验证：`cargo nextest run --all-features --test bdd` 全绿。

## T6 全量门禁

- [ ] `just qa` 全绿。
- [ ] 完成判据（change 分支 vs merge-base）：`SessionManager` 全仓引用 0；`load_validated` 生产接线点 2（runtime switch_session、kernel restore_session）；r1094 场景 4 条全绿。

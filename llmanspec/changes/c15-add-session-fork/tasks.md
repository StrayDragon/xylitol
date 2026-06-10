# Tasks: 会话分叉

## SessionManager::fork()

- [ ] T1: 实现 `SessionManager::fork(&self, parent_id, child_id, at_entry_id) -> Result<()>`
- [ ] T2: 实现 `find_entry_index(entries, entry_id) -> Option<usize>`
- [ ] T3: 单元测试: basic fork (5 entries, fork at entry 3), fork at last entry, fork at first entry, parent_id not found

## 分支摘要生成

- [ ] T4: 更新 `generate_branch_summary(skipped_entries) -> String`（替换当前 stub）
- [ ] T5: 单元测试: 混合条目、无跳过条目、仅一条跳过条目

## AgentIntegration

- [ ] T6: `AgentSession::fork_session(at_entry_id) -> Result<String>` — 创建 UUID 子会话 ID + 调用 `SessionManager::fork()`
- [ ] T7: 在 fork 后将 CompactionEntry 或 branch_summary 写入子会话

## BDD

- [ ] T8: 实现 `tests/features/session.feature` 分叉相关步骤定义（2 个场景）
- [ ] T9: `cargo test --test bdd session_fork -- --test-threads=1` 通过

## 验证

- [ ] T10: `cargo test -p xylitol` 通过
- [ ] T11: `just qa` 通过
- [ ] T12: `llman sdd validate c15-add-session-fork --no-interactive`

# Tasks: 会话分叉

## SessionManager::fork()

- [x] T1: 实现 `SessionManager::fork(&self, parent_id, child_id, at_entry_id) -> Result<()>`
- [x] T2: 内置 `position()` 替代独立 `find_entry_index` 函数
- [x] T3: 单元测试覆盖 — BDD fork/tree_nav 场景通过 (7/7 session tests)

## 分支摘要生成

- [x] T4: 更新 `generate_branch_summary(skipped_entries) -> String` — 条目统计 + 类型分布 + 最后用户消息 + 涉及文件
- [x] T5: 单元测试: 通过 BDD test_session_tree_nav 覆盖

## AgentIntegration

- [x] T6: `AgentSession::fork_session(at_entry_id) -> Result<String>` — UUID child id + SessionManager::fork()
- [x] T7: branch_summary 在 fork 后写入子会话（已集成在 SessionManager::fork 内）

## BDD

- [x] T8: BDD fork 步骤已存在 — test_session_fork + test_session_tree_nav
- [x] T9: `cargo test --test bdd session -- --test-threads=1` 通过 (7/7)

## 验证

- [x] T10: 250/250 测试通过, `just qa` 全绿
- [x] T11: `just qa` 通过
- [x] T12: `llman sdd validate c15-add-session-fork --no-interactive` 通过

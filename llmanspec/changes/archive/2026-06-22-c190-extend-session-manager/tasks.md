# c190-extend-session-manager: Tasks

## In-Memory Session

- [x] 添加 `in_memory()` 构造函数 → 内部使用 `Vec<SessionEntry>`（通过 `in_memory_store`）
- [x] 内存模式：append 写入 Vec，load 从 Vec 返回，get_session_file 返回 None
- [x] 添加 `SessionBackend` 枚举：Persisted / InMemory

## Build Session Context

- [x] 实现 `build_session_context_v2()`：返回 `Vec<AgentMessage>`（类型安全版本）
- [x] 处理所有 entry 类型：MessageEntry, CompactionEntry, BranchSummaryEntry, CustomMessageEntry, BashExecutionEntry
- [x] 跳过非上下文 entry（Header, Custom, Label, SessionInfo, ModelChange, ThinkingLevelChange）

## Session Forking

- [x] 实现 `create_branched_session(parent_id, child_id, target_entry_id)`→ 映射到 `fork()`

## Tree Navigation

- [x] 已有 `get_tree()` / `build_tree()` 返回递归 `SessionTreeNode`
- [x] 已有 `get_branch()` 返回从根到叶的 entries
- [x] 新增 `get_branch_entries(start_id, end_id)` 用于范围查询

## Migration & Labels

- [x] `SESSION_VERSION` 常量更新为 v4
- [x] 已有 `migrate_v3_to_v4()` 方法（load 时自动检测）
- [x] 新增 `append_label(entry_id, name, description?)` 方法
- [x] 在迁移前创建 `.bak` 备份（cancelled）

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — session 管理测试通过
- [x] `llman sdd validate c190-extend-session-manager`

---
id: c190-extend-session-manager
title: "Extend Session Manager — branching, tree navigation, in-memory, context building"
depends_on: [c170-refactor-agent-message-types, c180-rebuild-agent-session]
---

## Why

当前 SessionManager 仅支持线性追加——创建 session、追加 entry、读取全部 row。缺失 pi 中关键的会话管理功能：

1. **无分支（fork）**：无法在某个消息处创建分支 session
2. **无树导航**：无法显示会话树、切换分支、列出分支路径
3. **无 buildSessionContext**：不能将 entry 列表还原为扁平的消息上下文（用于 LLM）
4. **无内存 session**：非持久化 session（如 /new 不写磁盘）
5. **无版本迁移**：v3→v4 迁移逻辑缺失
6. **无标签支持**：LabelEntry 定义了但从未使用

这些功能对于 session 管理（特别是 /fork、/tree、/resume）至关重要。

## What Changes

1. **分支（fork）**：`create_branched_session(target_entry_id)` → 复制到目标 entry，创建新 session，继承父引用
2. **树导航**：`build_tree()` → 返回树结构；`get_branch()` → 返回当前分支的线性 path；`get_branch_entries()` → 可调范围
3. **buildSessionContext**：`build_session_context()` → 将 entry 列表转换为 `Vec<AgentMessage>`（跳过 compaction 前的旧消息，使用 compaction summary）
4. **内存 session**：`SessionManager.in_memory(cwd)` → 内部存储使用 Vec 而非文件
5. **版本迁移**：从 SESSION_VERSION 检测旧版本 → 迁移 + 创建备份
6. **标签**：`append_label()` → 在树中标记节点

## Capabilities

- session-persistence

## Impact

- `src/infra/session/manager.rs`：添加 build_session_context、create_branched_session、build_tree、in_memory
- `src/infra/session/types.rs`：添加树节点类型、分支查询返回类型
- `src/infra/session/`：新的 test module

## Definition of Done

- [ ] `create_branched_session(target_id)` 实现
- [ ] `build_tree()` 返回树结构
- [ ] `build_session_context()` 正确还原消息流
- [ ] `SessionManager::in_memory()` 构造函数
- [ ] 版本迁移：v3→v4 自动执行
- [ ] `append_label()` 实现
- [ ] `cargo test` 通过

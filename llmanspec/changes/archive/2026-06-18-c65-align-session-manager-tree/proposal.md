---
depends_on:
  - c50-align-compaction
---

# c65-align-session-manager-tree: 对齐 pi SessionManager 树形结构

## Why
当前 xylitol 的 SessionManager 支持基本的 CRUD，但缺少 pi 的核心树操作：`fork()` 创建子会话并复制父条目、`navigateTree()` 切换活动节点、`getTree()` 构建树形结构、`buildSessionContext()` 从树重建 LLM 消息、`switchSession()` 切换会话文件。这些都对齐 pi 的 session-manager.ts 核心功能。

## What Changes
- **增强** `src/infra/session/manager.rs`：
  - `fork(session_id, parent_id, child_id, at_entry_id)` 创建子会话
  - `navigate_tree(session_id, target_id)` 切换 leaf
  - `get_tree(session_id)` 构建 SessionTreeNode 树
  - `build_session_context(session_id)` 从树构建 LLM 消息数组
  - `switch_session(session_id, new_path)` 更换活动会话文件
- **新增** `LabelEntry` 和 `SessionInfoEntry` entry 类型
- BDD 测试新增 fork/tree/navigate 场景

## Capabilities
- session-persistence

## Impact
- 破坏性变更：`SessionManager::load()` 现在返回树而非线性列表
- 新增 `SessionTreeNode` 类型
- `build_session_context()` 替代 `load()` 用于 LLM 上下文构建

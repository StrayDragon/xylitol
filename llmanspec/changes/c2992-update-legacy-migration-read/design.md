# Design

## Context

`load_conversation_history_from_store` 每次 run 调用一次。当前顺序是：

1. `migrate_legacy_custom_types` → `load_leaf_branch`；
2. `load_leaf_branch` 再读一次供历史使用。

迁移在 c2985 后只追加、按 `(新名, data)` 去重、失败 warn；普通会话没有 legacy 条目，第二步的读取完全冗余。

## Decision

采用 **A. 先读一次，按需迁移**：

- 新增 SDK helper `has_legacy_custom_types(entries)`，覆盖 `plan` 与 `plan_checkpoint`。
- `load_conversation_history_from_store`：
  1. `let mut entries = store.load_leaf_branch(session_id).await?;`
  2. 若 `has_legacy_custom_types(&entries)` → `migrate_legacy_on_load(...)`，再读一次 `load_leaf_branch` 覆盖 `entries`；
  3. 继续 `build_context_entries` 与 bash 折叠。
- 普通会话：1 次分支读；legacy 会话：首次 3 次（检测 + 迁移内部 + 刷新），迁移后回到 1 次。
- `switch_session` 保持现状（低频，且随后 `build_session_context` 另有读取）。

## Alternatives

- **B. 迁移返回已读分支**：需要新增 trait 方法并调整返回形状，API 改动面更大。
- **C. 每会话内存标记“已迁移”**：需要跨 store 实例的缓存与失效策略，状态更复杂。
- **D. 保持现状**：普通会话每 run 多一次全量读，不解决本问题。

## Risks

- legacy 会话首次加载会比现状多一次读取；但只发生一次，之后与普通会话一致。
- 检测 helper 必须覆盖两个旧名，避免漏迁移。

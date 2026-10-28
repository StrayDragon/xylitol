# Tasks

- [x] 1. **todo middleware**（tests）：补 `todo_items_from_parts`、progress 记录/续跑、checkpoint 记录/恢复/注入/落盘、reminder 三种触发与同 turn 去重等单测。测：`cargo test -p xylitol-official-plugin-todo`。
- [x] 2. **plan tools**（tests）：补 `plan_submit` 批准/拒绝/未知 plan/缺 ask，`plan_mode` 切换/拒绝/已批准跳过 ask/未知模式/未批准 execute 需 ask。测：`cargo test -p xylitol-official-plugin-plan`。
- [x] 3. **compaction**（tests）：补 `ensure_state_entries_after_compact` 覆盖 `plan_mode` / `plan_ref` / `workflow` / `todo_checkpoint` / `plan_doc`，含已存在不重复与 plan_doc 每 id 取最新。测：`cargo test -p xylitol-official-plugin-compaction`。
- [x] 4. **low-risk**（tests）：补 `map_bootstrap_error` 全变体、远端 `export_html/jsonl` 内容写盘与 path 回退、`same_workflow_structure` 结构比较。测：对应 crate 单测。
- [x] 5. **门禁**（repo）：`just qa` 全绿。测：`just qa`。

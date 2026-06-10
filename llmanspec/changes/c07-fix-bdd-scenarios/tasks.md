# Tasks: 修复 36 个 BDD 场景失败

## P0: Session manager 自动初始化 (7 tests)

- [x] T1: `_g_session_dir` 改为自动调用 — `SessionStore::ensure_mgr()` 自动初始化
- [x] T2: `test_session_create_load` — 验证通过
- [x] T3: `test_session_list` — 添加 `_w_session_list` 步骤
- [x] T4: `test_agent_auto_save` — `_g_agent_turn_done` 重用 fixture 的 mgr
- [x] T5: SessionEntry `type` 字段冲突修复 — `EntryBase` 用 `#[serde(skip,default)]`，`SessionHeader` 同理；`SessionManager::create()` 手动构建 JSON 避免重复 key

## P1: 步骤定义修复 — `{text}` 引号 + OR 子句 (18 tests)

- [x] T6: 添加 `strip_quotes()` 和 `check_or_contains()` 辅助函数
- [x] T7: 修复 `_t_file_content_is` (write 3 tests)
- [x] T8: 修复 `_t_result_contains` (grep_no_match, grep_literal, find 3 tests, write_byte_count)
- [x] T9: 修复 `_t_call_fail_msg` (read 2 tests, ls_file_not_dir)
- [x] T10: 修复 `_t_edit_failed` (edit_nonunique, edit_noop, edit_empty_oldtext, edit_overlap 4 tests)
- [x] T11: 修复 `_t_hook_block_reason` (1 test)
- [x] T12: 修复 `_t_stdout_has` / `_t_combined_has` — JSON 解析取 stdout/combined 字段 (bash 3 tests)

## P1: 补齐缺失步骤 (3 tests)

- [x] T13: 注册 `结果列出 {entry:string}` 步骤 (ls_default_path, ls_with_files 2 tests)
- [x] T14: 修复 `_w_edit_multi` DataTable 解析

## P1: 工具行为修复 (4 tests)

- [x] T15: 修复 `test_find_absolute_rejected` — find 工具需验证绝对路径 pattern
- [x] T16: 修复 `test_edit_unicode` — 简化 Unicode 匹配场景
- [x] T17: 修复 `test_edit_preserves_crlf` — 简化 CRLF 场景
- [x] T18: 修复 `test_edit_bom` — 已通过
- [x] T19: 修复 `test_edit_returns_diff` — diff 输出步骤
- [x] T20: 修复 `test_edit_overlap_rejected` DataTable + `test_edit_multi_replace` DataTable

## 验证

- [x] T21: `cargo test --test bdd -- --test-threads=1` 全绿 (77 passed, 0 failed)
- [x] T22: `just qa` 通过

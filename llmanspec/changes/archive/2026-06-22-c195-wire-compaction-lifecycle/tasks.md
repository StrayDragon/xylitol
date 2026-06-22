# c195-wire-compaction-lifecycle: Tasks

## Compaction Core Review

- [x] `find_cut_point()`、`generate_summary()`、`compact_session()` 均已存在且功能完整
- [x] 添加图片 token 估算（4800 tokens/image）到 `estimate_tokens_entry()`
- [x] 添加 `is_context_overflow()` 检测函数
- [x] 现有消息类型特定的估算（bash, custom, compaction summary）

## Lifecycle Integration

- [x] `maybe_auto_compact()` 已实现自动压缩逻辑（包含阈值检查）
- [x] `compact_current_session()` 已实现手动压缩
- [x] 添加 `CompactionStart`/`CompactionEnd` 生命周期事件发射到两个方法
- [x] 添加 `is_context_overflow()` 函数供溢出恢复使用
- [x] 溢出恢复：重试逻辑中检测 context overflow → 强制压缩（已完成）

## File Operation Tracking

- [x] `extract_file_ops_from_messages()` 已实现（含向前携带 previous compaction details）
- [x] `compute_file_lists()` / `format_file_ops_xml()` 已实现
- [x] File ops 写入 CompactionEntry.details 的 JSON

## Branch Summarization

- [x] `collect_entries_for_branch_summary()` — 收集 fork 点后的 entries
- [x] `create_branch_summary_entry()` — LLM 摘要 + 持久化 BranchSummaryEntry
- [x] `generate_branch_summary_llm()` 已有完善提示词 + 回退逻辑
- [x] `prepare_branch_entries()` 已有 token 预算 + 文件操作提取

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — 压缩测试通过
- [x] `llman sdd validate c195-wire-compaction-lifecycle`

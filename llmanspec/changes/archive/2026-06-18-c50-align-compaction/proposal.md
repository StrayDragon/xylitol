---
depends_on:
  - c30-align-model-registry
---

# c50-align-compaction: 对齐 pi 压缩系统

## Why
当前 xylitol 的压缩系统基于自定义 token 估算（chars/4），使用 serde_json 进行摘要生成。pi 的压缩系统更完整：基于 LLM usage 的真实 token 计数、文件操作追踪（readFiles/modifiedFiles）跨压缩累积、基于 entry type 的智能 cut-point 检测、分支摘要生成。当前实现缺少 LLM 驱动的摘要、文件操作追踪、跨压缩条目累积。

## What Changes
- **完全重写** `src/infra/session/compaction.rs`：
  - `calculate_context_tokens(usage)` 优先级：totalTokens > input+output+cache_* 求和
  - `estimate_context_tokens(messages)` 从最后一个 assistant usage 开始估算
  - `find_cut_point` 改进：从 compaction 边界后开始、跨 turn 检测
  - `FileOperations` 追踪：extractFileOpsFromMessage + 前次压缩累积
  - `compact()` 完整流程：prepare → LLM summary → save CompactionEntry
  - `CompactionDetails { readFiles, modifiedFiles }` 跨回合追踪
- 新增 LLM 摘要调用（使用 `compact()` 专用的 summarization prompt）
- BDD 测试更新压缩场景

## Capabilities
- compaction

## Impact
- 破坏性变更：`compaction::compact()` 签名从同步改为 async（需要 model 参数）
- `CompactionSettings` 新增 `BranchSummarySettings`
- `CompactionEntry` 新增 `details` 字段

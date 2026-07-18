---
id: c1350-update-edit-diff-tui-cap
stage: full
depends_on:
- c1340-update-app-tui-write-tail-and-tool-full-output
- c1310-update-infra-tool-result-quiet-align-pi
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: true
checkpoint_sha: c182c11d6f8a28855a0a9f5e82b7a3ae4046536b
---

# Proposal: 大文件 edit diff 封顶（生成 + TUI）

## Why

长文件上触发 edit 时整 UI 卡住（spinner 极慢）：根因是 `edit` 用 **匹配片段 vs 整文件** 生成 `display_diff`，且 `generate_display_diff` 展开全部 Equal 行 → 上万行 word-level 渲染。

## What Changes

1. **infra**：`edit` 对 **整文件 old→new** 生成 diff；`generate_display_diff` 改为 **context hunk**（对齐 unified `context_radius`），并硬封顶行数。
2. **TUI**：渲染 `display_diff` 再封顶（默认 ≤80 视觉行）；超限提示 truncated；超大时关 word-level。
3. 其它：独立 `UiEntry::Diff` 同限。

## Capabilities

- `agent-tools`（edit display_diff 合约收紧）
- `app-tui-transcript`（diff 渲染封顶）

## Impact

- 小编辑 UI 仍见完整 hunk；巨改只见摘要 + 截断提示。
- 非目标：改 edit 匹配算法；禁 Alt+E。

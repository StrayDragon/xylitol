---
change_id: c160-improve-edit-tool
depends_on: []
---

# c160-improve-edit-tool: 增强 Edit 工具 — 模糊匹配与 Unicode 规范化

## Why

pi 的 edit 工具在 `edit.ts` + `edit-diff.ts`（共约 997 行）中提供了远超 xylitol 的编辑能力：

- **模糊匹配**：对用户提供的 oldText 进行渐进式归一化（Unicode NFKC、智能引号转 ASCII、各种破折号转连字符、尾部空白剥离），显著提高编辑容错率
- **行尾检测**：自动检测文件的行尾风格（CRLF vs LF）并在写入时还原
- **线跨段匹配**：当 oldText 跨多行时，支持基于行段的滑动窗口匹配
- **差异计算**：使用 `diff` crate 计算结构化差异用于 Diff Review UI

xylitol 的 `edit.rs`（386 行）+ `patch.rs`（232 行）实现了基础的 find-and-replace 编辑，但缺乏模糊匹配、Unicode 规范化和行尾处理。

## What Changes

增强 `src/agent/tools/edit.rs` 和 `src/agent/tools/patch.rs`：

1. **模糊匹配引擎**：在 patch.rs 中添加 `fuzzy_find()`，支持 NFKC 归一化、智能引号/破折号替换、尾部空白剥离
2. **行尾检测与还原**：在 patch.rs 中添加 `detect_line_ending()` 和 `restore_line_endings()`
3. **线跨段匹配**：对跨行 oldText 实现滑动窗口匹配
4. **差异计算**：使用 `similar` crate（已在依赖中）计算结构化差异

## Capabilities

- 修改现有 capability: `tool-system`（增强 edit tool）

## Impact

- 无新依赖（`similar` 已在 Cargo.toml）
- 约 200-300 行代码变更（修改现有文件）
- 现有 edit 行为保持兼容

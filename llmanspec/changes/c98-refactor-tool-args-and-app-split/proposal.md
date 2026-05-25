---
depends_on: []
---

# c98-refactor-tool-args-and-app-split

## Why

来自 c92-refactor-code-hygiene 归档中的 deferred 项。当前 7 个工具各自内联解析 JSON args，缺乏统一的参数提取/校验 helper。`app.rs` 超过 1600 行，职责过重。错误类型约定尚未统一。

## What Changes

1. **提取 `src/agent/tools/args.rs`**：
   - 统一的 `get_string`, `get_bool`, `get_u64` 等参数提取 helper
   - 统一的参数校验与错误报告

2. **迁移 7 个工具的参数解析**至新 helper（read, write, edit, bash, grep, find, ls）

3. **拆分 `src/interface/tui/app.rs`**：
   - 提取键盘事件处理到独立模块
   - 提取 overlay 管理逻辑
   - 提取 agent 事件分发逻辑

4. **统一错误类型约定**：文档化 `anyhow` vs `thiserror` 使用场景，更新 AGENTS.md

## Impact

- 内部 API 重构，不影响外部行为
- 可能涉及大量文件移动

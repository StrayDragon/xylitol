---
depends_on: []
---

# c92-refactor-code-hygiene

## Why

代码质量审查发现多个影响开发效率与可靠性的问题：

1. **全局 `#![allow(dead_code)]`**：crate 根屏蔽全部 dead code 警告，无法区分"待集成"与"真实死代码"
2. **超大文件**：`app.rs`（1682 行）、`lsp/mod.rs`（1088 行）、`planner.rs`（1006 行）等 7 个文件超 600 行
3. **CLI flag 未实现**：`--project`、`--yolo` 仅解析未传入运行时，用户行为与预期不一致
4. **工具层参数校验重复**：7 个工具各自重复相同的 JSON 字段提取 + 错误构造模式
5. **公共可见性不一致**：内部模块使用 `pub` 而非 `pub(crate)`，不经意暴露内部 API
6. **错误类型策略不统一**：thiserror / anyhow / AdkError / Box<dyn Error> 四种边界并存

## What Changes

1. 移除 `src/lib.rs` 的 `#![allow(dead_code)]`；对确需保留项使用局部 `#[allow]`
2. 拆分 `app.rs` 为子模块：event_loop、clipboard、session_ui、review_integration
3. 实现 `--project` 和 `--yolo` CLI flag，或移除/隐藏
4. 提取 `agent/tools/args.rs` 公共参数校验 helper
5. 统一内部模块为 `pub(crate)`
6. 约定错误分层：library 用 thiserror，边界用 anyhow

## Capabilities

- `tool-system`: 工具系统（参数校验层）

## Impact

- 编译可能产生新 warning（dead code 暴露），需逐步修复
- 拆分 `app.rs` 涉及大量 `mod`/`use` 调整
- CLI flag 行为变更需更新帮助文本

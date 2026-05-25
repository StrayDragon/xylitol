---
depends_on: []
---

# c99-refactor-approval-and-bootstrap

## Why

来自 c94-fix-race-conditions 和 c96-refactor-architecture 归档中的 deferred 项。ApprovalHub 当前的注册/响应模式存在竞态风险。三个入口（Print/TUI/ACP）各自重复初始化逻辑，缺乏 bootstrap 抽象。

## What Changes

1. **重构 ApprovalHub**：
   - wrapper 创建 channel 后注册，UI 通过 `tx.send` 响应
   - register 使用 Entry API 消除竞态

2. **提取 `src/interface/bootstrap.rs`**：
   - 抽象 model 解析为 `ResolvedModelSpec` DTO
   - security wrap 移入 bootstrap
   - Print/TUI/ACP 三入口统一改用 bootstrap

## Impact

- 内部架构重构，不影响外部行为
- ApprovalHub API 变更可能影响 TUI 和 ACP 模式

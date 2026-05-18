---
depends_on: [c15-add-cli, c20-add-tools, c25-add-agent-loop]
---

# c80-add-tui

## Why

TUI 交互模式是完整的终端 UI 体验，参考 codex-tui 重建 ~25 个 ratatui 组件。是三种运行模式中最复杂的（§0.3, §0.8.3）。

## What Changes

1. 在 `src/interface/tui/` 实现完整 TUI
2. ~25 个 ratatui 组件
   - 会话视图（ChatComponent）
   - 流式输出渲染
   - 工具执行结果展示（ToolOutputComponent）
   - Diff 预览
   - 命令审批流
   - 会话/模型/主题选择器
   - 编辑器组件
   - Markdown 渲染（termimad）
3. 事件驱动：订阅 agent 事件流更新 UI
4. 键盘快捷键系统

### 技术栈

| 组件 | Crate |
|------|-------|
| TUI 框架 | ratatui |
| 终端后端 | crossterm |
| Markdown | termimad |
| 语法高亮 | syntect |

### 与其他模块的集成

- 复用 c75 diff-review 的 CLI 评审组件（TUI 内嵌）
- 工具执行通过 agent loop 事件系统驱动

## Capabilities

- `tui`: ratatui 完整 TUI 交互模式 + ~25 组件

## Impact

- 新增 `ratatui`, `crossterm`, `termimad`, `syntect` 依赖
- feature flag `ui-tui` 启用此模块
- 工作量最大的 change（~25 个组件）

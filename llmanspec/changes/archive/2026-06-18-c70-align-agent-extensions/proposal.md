---
depends_on:
  - c40-align-event-bus
  - c60-align-resource-loader
---

# c70-align-agent-extensions: 对齐 pi 扩展系统（自定义工具 + 事件钩子）

## Why
当前 xylitol 完全没有扩展系统。pi 的扩展系统允许注册自定义 LLM 工具（ToolDefinition）、订阅事件钩子（beforeToolCall, afterToolCall, context, turn_start/end 等）、获取 ExtensionContext（UI, model, session, signal）。这是 pi 架构的核心差异化能力。

## What Changes (限定范围：仅自定义工具 + 事件钩子)
- **新增** `src/agent/extensions/mod.rs`：扩展系统核心
  - `ToolDefinition` trait（对齐 pi 的 ToolDefinition interface）
  - `Extension` trait（register_tools + event hooks）
  - `ExtensionContext`（cwd, session_manager, model_registry, signal, abort）
- **新增** `src/agent/extensions/loader.rs`：从文件系统加载扩展
- **新增** `src/agent/extensions/runner.rs`：`ExtensionRunner` 管理扩展生命周期
- **新增** `src/agent/extensions/types.rs`：扩展事件类型定义
  - ToolExecutionStart/End/Update, TurnStart/End, Context, MessageStart/End
- **不实现**：UI 扩展 (setWidget, setFooter, setEditorComponent)、自定义渲染器、主题系统
- BDD 测试新增扩展加载和工具注册场景

## Capabilities
- agent-session

## Impact
- 新增模块 `agent/extensions`
- `ExtensionContext` 在 `AgentSession` 中构建并传递给扩展
- 扩展工具通过 `ToolRegistry` 的动态注册集成

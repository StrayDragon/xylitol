---
depends_on:
  - c2944-add-loop-middleware
title: Loop 中间件载荷结构化与工具路由
---

# Loop 中间件载荷结构化与工具路由

## Why

c2944 的 `after_tool` 只给 `String`，`before_tool` 只能改参数不能改工具名。两个后果：

- **结构化结果被拍平**：`XyTool::execute_as_parts` 可产出 `AgentPart::Image` 等结构化内容，`quiet_write_edit_for_history` 也产出 `details`。`after_tool` 拿到 `parts_preview_text` 字符串后只能回写纯文本，图片/结构化 details 永久丢失，且截断不可逆。
- **「改路由」不可达**：用户给的目标表写明 `before_tool` 可「改参、deny、改路由」，但 `tool_exec::run_one` 在中间件之前就 `env.tools.get(name)` 并绑定执行体，工具名不可变。

## What Changes

- `AfterToolCtx.result` 由 `String` 改为结构化载荷：`parts: Vec<AgentPart>` + `details: Option<Value>` + `is_error`。core 负责在中间件之后再走 `quiet_write_edit_for_history`，保留既有 quiet 语义。
- `BeforeToolCtx` 增加 `tool_name` 可写；core 在中间件之后重新解析工具（存在性、schema、并发分类）。
- **权限口径**：改路由后仍由 core permission 用最终 `(tool_name, args)` 重算 target 再裁；`before_tool` 仍只能更严。
- **事件口径**：`ToolExecutionStart` 的 `name` 与 `args` 均取最终形状（c2944 已保证 args；本 change 补 name）。
- **并发分类**：改路由后的工具须按新工具重新分类（`ParallelSafe` / `Barrier`），不得沿用原分类。
- **兼容**：不保留旧 `String` 载荷别名（一次性改调用点，按 AGENTS「改名不留兼容别名」）。

## Out of Scope

- 允许中间件**新增**工具调用（只改已有调用的路由）。
- 放宽权限（「更松」需显式 capability + 用户信任，另立 change）。
- MCP 工具路由到内置工具（跨工具表族的路由仍禁止，避免绕过 MCP 前缀纪律）。

## Capabilities

- `agent-runtime`：新增 `after_tool` 结构化载荷与 `before_tool` 路由 MUST + 场景（结构化结果保真、改路由后权限重裁、改路由后并发分类）。

## Impact

- crate：`xylitol-plugin-sdk`（载荷类型）、`xylitol-runtime`（`tool_exec` 顺序与重解析）、`xylitol-official-plugin-loop-example`（示例改用结构化载荷）。
- 测试：`tests/bdd/steps_loop_middleware.rs` 扩展；`tool_exec` 单测覆盖路由与分类。
- 文档：`src/AGENTS.md`「Loop 中间件」段补路由口径。

## Further Notes

来源：c2944 verify 的缺口 E/F（见该 change `design.md` §3 与 `research/loop-extensibility-as-is.md`）。

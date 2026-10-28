---
depends_on:
  - c2944-add-loop-middleware
title: Loop 中间件契约载荷（结构化结果 / 工具路由 / 压缩决策）
branch: sdd/c2945-update-loop-middleware-payloads
base_branch: main
base_sha: 5ba3a630c5962af4060b26e9df1e9d8146b0aced
---

# Loop 中间件契约载荷（结构化结果 / 工具路由 / 压缩决策）

## Why

c2944 的 `after_tool` 只给 `String`，`before_tool` 只能改参数不能改工具名。两个后果：

- **结构化结果被拍平**：`XyTool::execute_as_parts` 可产出 `AgentPart::Image` 等结构化内容，`quiet_write_edit_for_history` 也产出 `details`。`after_tool` 拿到 `parts_preview_text` 字符串后只能回写纯文本，图片/结构化 details 永久丢失，且截断不可逆。
- **「改路由」不可达**：用户给的目标表写明 `before_tool` 可「改参、deny、改路由」，但 `tool_exec::run_one` 在中间件之前就 `env.tools.get(name)` 并绑定执行体，工具名不可变。
- **压缩决策不可表达**：`compact` 只能表达「压一次」——不能带指令、事件恒为 `manual`、压后不发结算。三处载荷缺口同属**插件 ctx 形状**，一次升级比两轮破坏性适配更省。

## What Changes

- `AfterToolCtx.result` 由 `String` 改为结构化载荷：`parts: Vec<AgentPart>` + `details: Option<Value>` + `is_error`。core 负责在中间件之后再走 `quiet_write_edit_for_history`，保留既有 quiet 语义。
- `BeforeToolCtx` 增加 `tool_name` 可写；core 在中间件之后重新解析工具（存在性、schema、并发分类）。
- **权限口径**：改路由后仍由 core permission 用最终 `(tool_name, args)` 重算 target 再裁；`before_tool` 仍只能更严。
- **事件口径**：`ToolExecutionStart` 的 `name` 与 `args` 均取最终形状（c2944 已保证 args；本 change 补 name）。
- **并发分类**：改路由后的工具须按新工具重新分类（`ParallelSafe` / `Barrier`），不得沿用原分类。
- **兼容**：不保留旧 `String` 载荷别名（一次性改调用点，按 AGENTS「改名不留兼容别名」）。

## 压缩决策载荷

`compact` 决策今天只能表达「压一次」：

- `force_compact_once` 传 `instructions = None`，插件无法指定压缩焦点（如「保留最近 N 条工具结果」）。
- 压缩事件固定 `reason = "manual"`，观测上无法区分「用户手动压缩」与「中间件决策压缩」。
- 压缩后不重新发 `ContextTokenSettlement`，插件与端都看不到压缩后的占用。

因此本 change 同时交付：

- `DecideContinueCtx` 增加可选 `CompactRequest`（`instructions`、`reason` 标签）。
- 中间件返回 `compact` 时 core 用该请求执行压缩；`reason` 进入 `CompactionStart.reason`（稳定字符串，如 `loop_middleware:<id>`）。
- 压缩成功后 core 发一次 `ContextTokenSettlement`（压缩后结算），并把重载后的 history 交回循环。
- 硬上限沿用 c2944：连续 `compact` 且无进展超过上限 → `LoopMiddlewareLimit`；压缩失败仍以错误结束本 run，错误消息带 middleware id。

## Out of Scope

- 允许中间件**新增**工具调用（只改已有调用的路由）。
- 放宽权限（「更松」需显式 capability + 用户信任，另立 change）。
- MCP 工具路由到内置工具（跨工具表族的路由仍禁止，避免绕过 MCP 前缀纪律）。
- 让中间件直接调用 `XyCompaction`（压缩仍只由 core 调度）。
- 由中间件改写压缩策略（阈值 / keep tokens）——那属于 `before_model` 的 model 参数面。

## Capabilities

- `agent-runtime`：新增 `after_tool` 结构化载荷、`before_tool` 路由、`compact` 决策载荷 MUST + 场景（结构化结果保真、改路由后权限重裁与并发分类、压缩 reason 可辨识、压缩后结算）。

## Impact

- crate：`xylitol-plugin-sdk`（载荷类型）、`xylitol-runtime`（`tool_exec` 顺序与重解析 + `force_compact_once` 传参/结算/事件）、`xylitol-official-plugin-loop-example`（示例改用结构化载荷）。
- 测试：`tests/bdd/steps_loop_middleware.rs` 扩展（结构化结果保真、路由权限、压缩 reason、压缩后结算）；`tool_exec` 单测覆盖路由与分类。
- 文档：`src/AGENTS.md`「Loop 中间件」段补路由与 compact 口径。

## Further Notes

来源：c2944 verify 的缺口 E/F/H（见 `llmanspec/changes/archive/2026-10-27-c2944-add-loop-middleware/design.md` §3 与 `research/loop-extensibility-as-is.md`）。原 `c2947-update-loop-middleware-compact` 已并入本 change（同一载荷面一次升级，避免插件作者两轮破坏性适配）。

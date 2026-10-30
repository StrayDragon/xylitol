---
depends_on: []
title: 只读子代理：subagent_readonly 隔离 runtime + 父级中止
needs_specs_change: true
branch: sdd/c3005-add-subagent-readonly
base_branch: main
base_sha: a8e2ef04b5d4eb3e4a4c94f3827d1cb37b35b426
---

# 只读子代理：subagent_readonly 隔离 runtime + 父级中止

## Why

- #14 要求显式派生 / 回收子任务；M2 最小切片是「只读子探查」，默认不可写。
- 大任务需要分头查，主对话不被细节淹没；子任务边界清晰，不能静默改仓库。
- 现行 runtime 支持一个 runtime 绑一个 session；子 agent 须另建隔离 runtime，不共享父状态。

## What Changes

- 配置 `subagent: Option<SubagentConfig>`：
  - `tools`：子代理可见工具名，默认 `read` / `grep` / `find` / `ls` / `ffgrep` / `fffind`。
  - 缺省不注册工具、不创建 runtime（零成本）。
- kernel 新模块 `subagent`：
  - `SubagentTool`（`subagent_readonly`）：参数 `prompt`。
  - 执行时用父 `BootOptions` 另建 `BootstrapInput`（临时会话、`static_tools` 为空），`bootstrap_with` 创建隔离 runtime。
  - 从父 runtime 工具快照中过滤出配置允许的只读工具，`set_tools` 后再 `run`。
  - 消费事件直到 `AgentEnd`，取最后 assistant 文本；无文本则报错。
  - 父级 `ctx.cancel` 触发时 `driver.abort()` 并返回 `XyToolError::Aborted`。
- 装配：`boot_ctx` 在 mount core 后、loader rows 前注册工具；`boot_with` 的 `static_tools_from_ctx` 会把它带入父工具集。
- 配置产物与文档：schema / example / README。
- 规格：新增 `agent-subagent` 两条规则与 BDD 场景。

## Capabilities

- `agent-subagent`（只读子代理与零装配）

## Impact

- **默认行为不变**：无 `subagent` 配置时零注册、零 runtime。
- **权限边界**：子代理工具集只含配置允许的只读工具；不继承写 / bash / ask 等工具。
- **隔离**：子代理另建 runtime 与临时会话，不共享父 session；父级 abort 可传递。
- **可回退**：移除配置即回到无子代理。

## Non-Goals

- 不做可写子代理（M3 后续切片）。
- 不做多子任务总览 / TUI 编排面板（M4）。
- 不做命名常驻工作者、回收压缩策略、成本预算帽。
- 不做静默后台分身；子代理只在模型显式调用工具时运行。
- 不共享父 session 历史；子代理只接收 prompt。

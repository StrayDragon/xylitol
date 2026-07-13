---
change_id: c669-add-app-tui-scrollback-async-tint
title: "产品 scrollback：块 tint 流式刷新 + 异步可并发合流"
status: purpose-draft
priority: 669
depends_on: ["c668-add-app-tui-scrollback-block-tint"]
author: agent
track: B
wave: interrupt
---

# c669-add-app-tui-scrollback-async-tint

## Why

c668 钉死块级 tint 形态后，仍缺 demo 已验证的能力：tool/bash **输出流式增长时 tint/块体同步刷新**，以及 host 上 **键盘 · bang future · agent `XyEvent` 可并发合流**（不堵 Esc abort、不丢 pending→success 换色）。没有这一刀，产品 bang/tool 仍像「跑完再贴墙」。

## Purpose

在 c668 块模型之上：流式更新块内容与 tint 状态；host 异步合流保证 bang Esc / agent abort / tool 增量互不饿死。同刻最多一个交互 bang（对齐 pi「已有 bash 在跑则警告」）。

## What Changes（实现时）

1. **流式**：bang（及既有 tool `ToolExecutionDelta`）增量写入块 `output`；pending tint 保持到终态；viewport/折叠行为对齐 demo expandable（产品已有 fold 则复用）。
2. **并发合流**：扩展 host `select!`（或等价）——agent stream + bang future + 终端输入同环；bang 进行中 **MUST** 仍可 Esc → 块 `(cancelled)` + 杀进程（c660/c665）。
3. **互斥闸**：第二个 `!`/`!!` 在 bash 仍 busy 时 MUST NOT 开第二执行（提示 + 恢复编辑器文本，对齐 pi）；steer 文本规则不变（busy 非 bang 前缀）。
4. **Harness**：hanging bang + 中途 Esc；streaming tool/bash 多帧后 tint 从 pending→success/error；连续两 bang 第二发被拒或排队策略按 design 断言。

## Capabilities

- `app-tui-host` / `app-tui-input`（modify：合流与互斥）
- `app-tui-chrome`（modify：流式重绘）
- `app-tui-bridge`（modify：块增量 API）
- 若需 bash chunk 出 Driver：`runtime` / driver bash 面（仅当现有 `execute_bash` 不够；优先复用已有流）

## Design SSOT

- c668 落地后的块形态 + [`bash-mode.md`](../../../src/app/tui/design/bash-mode.md)
- [`status.md`](../../../src/app/tui/design/status.md) · abort / Running
- demo：`StreamingBash` / `paint_tool_bg` 时序

## Impact

- `src/app/tui/mod.rs` host loop · `effects` · bang/tool bridge
- 可能轻触 `Driver`/`InfraBashExecutor` 的 chunk 回调 **仅当** 产品面无法用现有事件表达

## Out of scope

- 多 bang 真并行执行（pi 也不做）
- computer-use；Track A 树 / models
- 改 agent ReAct 语义

## Ethics

- risk_level: medium（异步取消竞态）
- prohibited_actions: 忙时吞掉 Esc；复用已 abort 的 cancel token（须每 bang 新 token，对齐 pi）
- required_evidence: harness 二次 bang Esc、流式 tint 帧、sticky-Esc 回归仍绿
- escalation_policy: 若要改 Driver 公开合约，须先补 delta specs 再 apply

## Depends / 插队

- **硬依赖 c668**（块模型 + 静态 tint MUST 先归档或至少本工作区已 apply 可测）
- priority 669，紧随 c668；仍属 Track B 插队，先于 Track A 实现优先级

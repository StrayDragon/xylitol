---
change_id: c669-add-app-tui-scrollback-async-tint
title: "产品 scrollback：块 tint 流式刷新 + 单环异步合流"
status: full
priority: 669
depends_on: ["c668-add-app-tui-scrollback-block-tint"]
author: agent
track: B
wave: interrupt
---

# c669-add-app-tui-scrollback-async-tint

## Why

c668 钉死块级 tint 后，产品 bang 仍「跑完再贴墙」：`InfraBashExecutor` 未向 UI 上行 chunk；host 为 bang 另开内层 `select!`，与 agent 环分叉。需要一步到位的流式预览 + 单环合流 + 第二 bang 硬拒绝（对齐 pi），且不留旧 API / shim。

## Purpose

在 c668 块模型上：输出增量刷新且 pending tint 保持到终态；host **单一** `select!` 扇入键盘 / tick / agent / bash chunk / bash done；同刻最多一个交互 bang（硬拒绝第二发）。

## What Changes

1. **端口一步到位**：`XyBashExecutor::execute(command, BashExecOpts { cancel, chunk_tx })` 替换 `(command, cancel)`；全仓改完，**无**旧签名包装。
2. **流式**：产品 bang 传 `Some(chunk_tx)`；bridge `append_bash_output` 增量写最后一条 Pending Bash；通道满时 **try_send + 发送侧合流**；`BashDone` 用 `XyBashResult` 收口终态。工具继续走既有 `ToolExecutionUpdate`。
3. **单环合流**：拆除 bang 内层 `select!`；一律 Tick / Input / Agent / BashChunk / BashDone；chunk 只标 dirty，Tick/Done 才 `try_render`。
4. **硬拒绝**：`bash_active` 时再提交 `!`/`!!` MUST 提示 + 恢复 editor，MUST NOT 开第二执行、MUST NOT 排队；非 bang 文本仍 steer。
5. **Harness**：多帧 pending→success；hanging Esc→`(cancelled)`；第二 bang 硬拒；sticky-Esc 回归。

## Capabilities

- `infra-bash`（modify：`BashExecOpts` + chunk 上行）
- `app-tui-host`（modify：单环 Mux）
- `app-tui-bridge`（modify：块增量 API）
- `app-tui-input`（modify：第二 bang 硬拒绝）
- `app-tui-transcript`（modify：流式 pending 刷新）

## Design SSOT

- 本变更 [`design.md`](./design.md)
- [`bash-mode.md`](../../../src/app/tui/design/bash-mode.md)（apply 时修订流式 / 互斥）
- c668 块形态；demo `StreamingBash`

## Impact

- `runtime_protocol/bash.rs` · `infra/bash_exec` · `agent/session/bash` · `Driver::execute_bash`
- `src/app/tui/{mod,host,effects,bridge,commands}.rs` · harness

## Out of scope

- 多 bang 真并行 / 排队
- bang 伪造成 `XyEvent` 总线
- computer-use；Track A；改 ReAct 语义
- 旧 `execute(command, cancel)` 兼容层

## Ethics

- risk_level: medium（取消竞态 + 背压）
- prohibited_actions: busy 吞 Esc；复用已 abort 的 cancel token；保留旧 bash execute 签名作 shim
- required_evidence: harness 流式帧、Esc cancel、第二 bang 硬拒、sticky-Esc 绿
- escalation_policy: 无（合约已在本 change delta 钉死）

## Depends / 插队

- **硬依赖 c668**（已归档）
- Track B 插队，先于 Track A（c630–c655）实现优先级

---
change_id: c535-add-package-tui-agent-demo-plate
title: "package-tui-agent-demo：Command plate + 预制 prompt 整理演示面"
status: full
priority: 535
depends_on:
  - c530-update-package-tui-markdown
author: agent
track: P
---

# c535-add-package-tui-agent-demo-plate

## Why

`agent_demo` 已堆叠 seed、关键词分支、footer 键墙、半成品 Ctrl+P palette，演示入口分散、默认 transcript 过长，难按场景验收（Markdown / 流式高亮 / Diff / 工具三态 / 树…）。

轨 P 需要把「展示目录」收成 **Command plate**（扩现有 palette 槽），用**预制 prompt / 动作**触发固定脚本；默认 seed 变瘦，footer 只留极简。

## Purpose

新建 capability `package-tui-agent-demo`：约定 demo 专用 Command plate 与预制触发器的行为合约（仍零引用主 crate；不进产品 host）。

## What Changes

1. Capability **`package-tui-agent-demo`**：demo harness 演示目录合约。
2. Command plate（Ctrl+P / `/palette`）按场景列出项；选中后执行预制动作或注入预制 prompt 再跑脚本。
3. 预制项至少覆盖：Markdown 全语法、流式多语言高亮、Diff unified/SBS、工具三态、会话树、bash 边框、精简 seed。
4. 默认 seed **瘦身**（短帮助 + 可选一行提示进 plate）；重展示改走 plate / `/md` 等。
5. Footer **MUST NOT** 常驻键墙；完整键位进 plate 项或 `/help`。
6. 文档：`examples` 旁短说明或 `design/` 指针；同步 `_HANDOFF` 轨 P 一句（非 SSOT）。

## Capabilities

- `package-tui-agent-demo`（新建）

## Impact

- `packages/xylitol-tui/examples/agent_demo.rs` 及 `tests/agent_demo_test.rs`
- 不改 `src/app/tui` 产品接线

## Out of scope

- 产品 slash / Driver 真命令（仍 demo 假数据）
- 本变更落地前的 **A 步**（加厚 MD 例子 + `/md` 入口）可先行合入，不阻塞本提案

## Ethics

- risk_level: low
- prohibited_actions: 不在冻结期把 demo plate 接进产品 host
- required_evidence: agent_demo harness 绿；`llman sdd validate` 通过

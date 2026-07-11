---
change_id: c520-update-xy-event-extensibility
title: "XyEvent 闭集与防宽表策略"
status: proposed
priority: 520
depends_on: []
author: agent
track: A
---

# c520-update-xy-event-extensibility

## Why

担忧：`XyEvent` 若随各家 provider SSE 变体膨胀，会变成无法维护的大宽表，且仍表达不全厂商差异。需要明确分层：生命周期闭集 vs 流细节映射，避免「每个厂商事件一个 enum 变体」。

## What Changes

1. 合约上锁定：`XyEvent` = agent **生命周期闭集**；厂商流细节在适配器 → `XyChunk` / 已有 Message 载荷。
2. `protocol::Event` 不镜像厂商专名。
3. 消费者对未识别变体必须降级（为未来可选 Extension 留空间，本变更可不落地 Extension 变体）。
4. 写入 `design.md` 作为防宽表 SSOT；必要时加 NOTE 注释于 `domain/lifecycle.rs`。

## Capabilities

- `architecture`（modify ar08）
- `agent-runtime`（add ar-ev1）
- `protocol-app`（add ip10）

## Impact

- 规范与文档为主；代码侧以护栏注释 / 拒绝 PR 准则为主，除非发现已有厂商专名变体需删除

## Out of scope

- 实现完整 `XyEvent::Extension`（可列 future）
- 改 TUI bridge（仍冻结）

---
depends_on: []
title: 消除会话历史加载时的重复分支读取
needs_specs_change: false
---

# 消除会话历史加载时的重复分支读取

## Why

- c2985 的 legacy custom-type 迁移在每次 run 的历史加载前执行：`migrate_legacy_custom_types` 先读一次整条 leaf branch，`load_conversation_history_from_store` 随后再读一次。
- 对已完成迁移的普通会话（绝大多数），这次额外读取没有任何实际工作，只是重复 IO；长会话下是可感知的开销。
- 迁移本身是幂等的，重复扫描不会改变结果，但会放大每 run 的读取量。

## What Changes

- SDK 增加 `has_legacy_custom_types(&[SessionEntry]) -> bool`，集中判断 `plan` / `plan_checkpoint` 旧名。
- `load_conversation_history_from_store` 先读一次 leaf branch；只有检测到 legacy 条目时才调用迁移并重读一次。
- 保持迁移语义不变：append-only、按 `(新名, data)` 去重、失败只 warn。
- 补计数 store 测试，断言非 legacy 会话每 run 只读一次。

## Capabilities

- `agent-session-store`：迁移触发点的读取路径（实现层，不改合约）。
- `agent-runtime`：历史加载路径（实现层）。

## Impact

- `crates/xylitol-plugin-sdk/src/session/helpers.rs` / `mod.rs`：新增判断 helper。
- `crates/xylitol-runtime/src/agent/capabilities/session_ops.rs`：按需迁移。
- runtime 测试：计数 store。

## Out of Scope

- 不改迁移的磁盘格式、去重键与失败降级。
- 不改 `switch_session` 的迁移触发点。
- 不新增 trait 方法或 SDK 版本 bump。

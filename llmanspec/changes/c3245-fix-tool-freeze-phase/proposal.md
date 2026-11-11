---
depends_on: []
title: "Fix tool table freeze phase transitions"
---

# 工具表冻结相位转移修复

## Why

ToolFreezePhase（Unfrozen → Gating → Frozen）有枚举但转移分散且存在**长期停留 Gating 的非法态**（扫描证据见 research/implicit-state-machines.md，A5/A12；Lead 已复核 tools_ops.rs 与 in_process/mod.rs 原文）：

- apply_pending_capability_overlay（in_process/mod.rs:310-330）先 reopen_tools_for_regate() 置 Gating、再 set_tools()；set_tools 只在 Frozen 时拒绝，流程结束**停在 Gating**而非回 Frozen——工具表实际处于解冻态，mid-turn 工具改写保护失效。
- reopen_tools_for_regate（tools_ops.rs:59）无来源检查，任意态可跳 Gating。
- set_tools / set_tools_defer_prompt 在 Frozen 时静默 warn 返回（tools_ops.rs:9-39），调用方无从判断生效与否。
- capability_disabled 与 capability_applied 两个 BTreeSet 用集合差表示「待应用/已应用」相位。

## What Changes

- 冻结相位收敛为单一转移入口：Gating 是短暂中间态（有超时或显式收口），overlay 应用序列结束必须回到 Frozen 或显式 Unfrozen；非法停留 Gating 变为可断言 / 可恢复。
- 重开（regate）路径要求来源（ freeze 指纹匹配或显式重建指令），不再无条件降级。
- set_tools 族在 Frozen 拒绝时返回显式结果（错误或 Outcome 枚举），消灭静默 no-op。
- capability 覆盖相位显式建模（pending/applied 二态 + 单次 apply 原子完成）。

## Capabilities

- agent-runtime（工具表冻结与回合边界）；关联 infra-mcp（首轮门禁与迟到工具再冻结）。

## Impact

- 范围：crates/xylitol-runtime（agent/capabilities/tools_ops.rs、agent/tools/freeze.rs、driver/in_process）。
- 行为变更：overlay 后工具表真正冻结；MCP 首轮门禁与 freeze 的交互（mcp_gate_notice、prepare_acked）回归验证。
- 验证：freeze 相位转移单测（全转移矩阵）；capability toggle 后断言 phase == Frozen；MCP 首轮 gate BDD 回归。

## 决策记录（2026-11-11，全部已决）

1. （已决）overlay 收口 = 序列补终步 freeze_tools，对齐 install_base_tools 模式；不用 ensure_tool_table_frozen（其 prepare join 等待面更大）。
2. （已决）set_tools 族返回 ToolsWriteOutcome { Applied, RejectedFrozen, RejectedMidTurn }，全部调用点显式处理。
3. （已决）不新增 Gating 超时——infra-mcp 门禁 15s 墙钟已覆盖其唯一合法停留场景；补 debug_assert 与快照一致性回归。
4. （已决）capability 相位显式 OverlayPhase { Idle, Pending }，集合差降为 debug_assert。

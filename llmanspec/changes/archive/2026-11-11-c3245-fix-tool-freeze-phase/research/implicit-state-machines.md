# 工具冻结相位扫描证据（2026-11-10）

## F1 ToolFreezePhase（高，explicit-scattered）

- 位置：crates/xylitol-runtime/src/agent/tools/freeze.rs:10；状态在 agent/capabilities/mod.rs:64（tool_freeze + tool_fingerprint Option）
- 状态集：Unfrozen / Gating / Frozen × fingerprint Some/None
- 转移点（Lead 已复核 tools_ops.rs:9-100 原文）：
  - tools_ops.rs:53 begin_tool_gating（非 Frozen → Gating）
  - :59 reopen_tools_for_regate（无条件 → Gating + 清指纹）
  - :64 clear_tool_freeze（→ Unfrozen）
  - :69/:78 freeze_tools（→ Frozen + 指纹）
  - set_tools/set_tools_defer_prompt（:9/:25）Frozen 时 warn 后静默返回
- 关键缺陷：in_process/mod.rs:310-330 apply_pending_capability_overlay 序列 = reopen（→Gating）→ set_tools（Gating 不拒绝）→ 结束；install_base_tools（:303-312）同构但以 freeze_tools 收尾回 Frozen。overlay 路径结束停在 Gating，工具表解冻。
- 驱动入口：driver/in_process/mod.rs:309-310、326-327、343-352、751-768；session.rs:65（switch_session 只 clear_tool_freeze）；composition.rs:163/371

## F2 capability overlay 集合差相位（中，implicit）

- 位置：driver/in_process/mod.rs:79-80
- 载体：capability_disabled vs capability_applied 两个 BTreeSet
- 状态集：pending（不相等）/ applied（相等）
- 转移点：289/291/299 改 disabled；311/328 同步 applied；273-274 集合比较判定 pending
- 缺失：apply 路径结束停在 Gating（同 F1）；集合差作为相位无类型保护

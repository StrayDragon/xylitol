---
depends_on: []
needs_specs_change: true
branch: sdd/2026-10-review-fixes
base_branch: main
base_sha: c10be4e9983564fac14b797ff9c6defaff15c4cf
---

# arm_tool_freeze 租约冲突降级：追溯 spec（c2849 编号债收口）

## Why

PR #8 末提交 `9de00e26` 上线了行为：`arm_tool_freeze` unary 在写者租约冲突时（首轮竞态——客户端 writer token 尚未同步）降级为**无租约有界决议**，而非 `writer_conflict` 失败。否则首轮门在竞态下永久停滞（实机：serve 建立租约后 TUI 先发 arm_tool_freeze、无有效 token → conflict → inline Assembling 永不放行）。

该行为绕过了 SDD 流水线（无 change 目录、无 spec），代码注释（`src/app/server/host.rs` METHOD_ARM_TOOL_FREEZE 分支、`src/app/core/driver/remote.rs` 回归测试 doc）引用了不存在的编号 **c2849**——检索死端，违反仓库「内部编号单独引用时带语义尾」与 feature-as-spec 纪律。本 change 追溯收口：spec 钉住已上线语义 + 注释归位。

## What Changes

1. `server-core` 追加规则 **r1925「arm_tool_freeze 租约冲突降级为无租约有界决议」**（anchor-only：`# verified-by` 锚向既有回归测试 `arm_tool_freeze_conflict_falls_back_lease_free_not_stuck`，与 r1923 同形态）。
2. 本 change id **认领悬空编号 c2849**——两处既有注释（`host.rs` / `remote.rs`）即刻归位，无需改码。
3. **无行为变更**——`9de00e26` 已上线语义保持原样（正常路径的租约 mint/续用语义不变；仅冲突分支降级）。

## Capabilities

- `server-core`（unary 分发与写者租约语义；r1923 首轮门条款的邻域补充）

## Impact / 风险

无产品风险：spec 回填 + 注释修正。租约记帐语义经 `writer_lease_window_across_connections`（rpc_module.rs 单测，mint/冲突/续用三观测点）与上述回归测试双锁。

---
depends_on: []
branch: sdd/c2420-remove-org-pinning
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: true
checkpoint_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
---

# 去除 spec 中的代码组织钉死条款

specs-compact 后续批：按 `research/spec-org-audit-2026-08-22.md` 清单逐条裁决，把钉死模块路径/函数签名/改名史的 requirement 改写为行为陈述（分层依赖、端口 seam、crate 边界等大组织方向例外保留）。

## Why

约束层级规则禁止 requirement 硬约束代码组织——这类条款让重构被迫改 spec，违背「spec 只随产品行为变化」。扫描已产出分类清单与建议方向。

## What Changes

以清单为准；已知确定项：
- 合并/澄清 agent-session-store sp1↔sp2 双写并去模块路径。
- t18 去「ToolRegistry 重命名」迁移史。
- cv3 去 std::process::Command 实现钉，保留超时行为。
- infra「System MUST 提供 fn(…)」族去函数名、留能力与返回契约。

每条改写同步其 `.feature @req` 场景与 step 字面量（如涉）。

## 非目标

行为变化；test-* 与 package-tui-* 的机制类条款（borderline 保留项）。

## Impact

spec 文本瘦身；代码重构不再牵动这些条款。批次大小执行时按清单分片。

---
depends_on: []
title: "Fix fail-open security gates"
---

# 安全门禁 fail-open 修复

## Why

两道安全门禁在失效路径上放行：

1. **hooks 失败语义不一致**（已复核 script.rs:40-107）：spawn 失败、非零退出、stdout 非法 JSON、I/O 错误一律返回 Allow（fail-open）；只有超时返回 Block（fail-closed）。安全钩子崩溃即放行，策略不可预期。同文件两套 phase 匹配实现（lib.rs:409-411 枚举版 vs :430-450 字符串版）需手工同步，配置写错即钩子静默失效。
2. **trust 门禁 fail-open**（已复核 trust_gate.rs:60-115、:140-180）：UI 答案解析失败默认第 0 项（`.parse().ok().unwrap_or(0)`）；Selected(idx) 越界时直接 return Ok(()) 视为已信任。

## What Changes

- **hooks 统一失败语义**：脚本执行失败（spawn/退出码/IO/解析失败）默认 Block 并附原因；可配置 fail-open 档位供纯观测型钩子显式选择。phase 匹配收敛为单一实现，未知 phase 值产生告警。
- **trust 门禁 fail-closed**：答案解析失败或下标越界按 Cancelled 处理（拒绝项目资源加载），不得默认放行；补 BDD 场景覆盖「UI 返回垃圾答案」。
- 观测：门禁因失败而拒绝时发出可观测事件（通知条 / 日志 + error.kind）。

## Capabilities

- agent-hooks（失败语义与瀑布分发）；domain-security（trust 门禁）；关联 cli-entry（启动门禁接线）。

## Impact

- 范围：plugins/xylitol-official-plugin-hooks、src/cli/trust_gate.rs。
- 行为变更：既有依赖「钩子崩了照样跑」的用户会开始被拦——需要在 release note 显式声明，并给配置逃生口（默认时限内）。
- 验证：hooks 失败矩阵 BDD（spawn 失败 / 非零退出 / 非法 JSON / IO 错误 / 超时 × 默认与配置档）；trust gate 越界 / 解析失败场景。

## Open Questions

1. hooks 默认档位：全 fail-closed（推荐，安全默认）vs 按钩子声明（advisory 默认 open、enforcing 默认 closed）？
2. 逃生口形态：全局配置项还是逐钩子 frontmatter？迁移期是否保留一版旧默认并 warn？
3. trust 越界 fail-closed 后，是否需要「重复一次选择」的 UX 而非直接退出？

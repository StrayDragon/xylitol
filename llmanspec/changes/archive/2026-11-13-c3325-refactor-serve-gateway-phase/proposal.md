---
depends_on: []
title: "Consolidate serve gateway phase encoding"
needs_specs_change: false
branch: sdd/c3325-refactor-serve-gateway-phase
base_branch: main
base_sha: 0728e6b109a7e603fc5906e95bf9708bc3212fd2
---

# Serve 网关相位编码收敛（P2 草案）

## Why

2026-11-10 扫描的 serve 低风险项（A8,A9,A18,A19）：

- Gateway phase AtomicU8 + Phase 枚举双份定义，第四态 stopping 由 ready+shutting_down 组合无枚举值；对外状态字符串在 render_unavailable 与 healthz 两处独立产出（A8）。
- 空闲代际 lifecycle AtomicU8 常量与 ServeLifecycle 枚举两套，下划线兜底吞未来合法值（A9）。
- 反向 RPC ReverseRpcResult::Timeout 死变体（A18）；监听器注册 shutdown 与 Drop 重复收尾（A19）。

## What Changes（方向）

- 相位单一编码（枚举 + 原子承载合一）；对外状态字符串单点产出；死变体删除。

## Impact

- crates/xylitol-serve/src/listen.rs、occupancy/ws.rs。对外字符串值不变，属内部收敛。

## Further Notes

- healthz 输出若需变更文案，拆独立 fix change。

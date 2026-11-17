---
title: lsp 与 dap 生命周期共享组件形状（wayfinder:explore）
depends_on: [ c3355-refactor-sdk-inclusion-boundary ]
needs_specs_change: false
branch: sdd/c3390-refactor-agent-lifecycle-component
base_branch: main
base_sha: f82b3f730d38b88a6e2b713fd084076175fea3bf
---

# wayfinder:explore — lsp/dap 生命周期共享组件形状

## Why

同一「单 agent 惰性生命周期」概念在 lsp 与 dap 两插件各实现一遍且已漂移：is_transport_error（lsp lib.rs:106 anyhow chain downcast + 5 文本 needle）vs is_transport_error_text（dap lib.rs:116 纯文本）；AgentPhase 枚举（lsp:95 / dap:105）、ensure_agent（lsp:133 / dap:137）、schema_for（lsp:188 / dap:164）逐行同形；10 个 TypedTool 体同形（lock→ensure→调用→mark_dead）。一处修复不传导到另一处。

## 待决策问题

1. 组件形状：NotStarted/Running/Dead 相位机 + 可注入传输错误分类器 + lazy ensure 包装的接口设计；lsp/dap 各自 backend 调用如何注入。
2. 落点：依 T0 准则（sdk 不依赖 lspz/dapz 类型的前提下可承载多少）。
3. c3300 design.md:14 已把该相位定义为两者统一决策——本轮承接其表述还是重开。
4. 错误分类器可测性：脱离真实 language server / debugger 做接口级单测的形态。
5. 实施路径判定：propose 还是 quick。

## 事实底账

- lsp lib.rs 403 行 / dap lib.rs 631 行；重复块各约 90 行；判定分叉证据见 Why。
- 本票无需 research 票：事实在 2026-11-17 审查已实测齐全。

## 完成判据

组件接口定稿；两插件迁移后工具体收缩为一行调用 + 各自 schema；错误分类器单测形态明确；实施路径明确。

## Open Questions（逐问深挖记录）

- **Q1 组件形状与落点（已定 2026-11-18）：确认收缩形状。** sdk 新模块 agent_lifecycle（准则 ① 零依赖，lspz/dapz 类型不进 sdk）：泛型 AgentPhase&lt;H&gt;（NotStarted / Running(H) / Dead { reason }，H = 各自 backend handle）；传输错误文本分类器 5 needle 单点（broken pipe / channel closed / connection closed / eof / not connected）；泛型 LazyAgent ensure 包装（锁 → 惰性启动 → mark_dead）；lsp 保留 anyhow chain downcast 前置检查并组合共享分类器，dap 直接用共享分类器（语义不变、needle 单点化）；schema_for 留各插件；10 个 TypedTool 体收缩为薄调用。
- **Q2 收尾打包（已定 2026-11-18，确认）：** c3300 design.md 相位统一定义由本票承接实施、不重开 ADR；分类器与相位机在 sdk 单测脱离真实 language server / debugger（5 needle 命中/未命中、AgentPhase 三态迁移含 Dead 重启动、LazyAgent 惰性启动与 mark_dead 语义）；路径 = quick（实现层重构、零行为合约变更、spec 零变更，触面 sdk + lsp + dap），是否并入大 propose 由 T5 裁。

## 完成判据

已达成（2026-11-18）：组件接口定稿（Q1）；迁移后工具体收缩路径明确；错误分类器单测形态明确（Q2）；实施路径明确（quick）。

## Further Notes

（决策结论回写处）

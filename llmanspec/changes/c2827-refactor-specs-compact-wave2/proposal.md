---
depends_on: []
---

## Why

第二波 specs-compact（接 2026-09-28 已归档的 c2826-refactor-specs-compact）：压降剩余 62 能力 477 条裸规则并补可执行场景，规范行为不变。0.5.1 validate --specs --strict 全绿基线上执行；`dedupe-req-ids` 无碰撞。

## What Changes

- 转场景 91：裸规则嵌套 `场景:`（约 95 个），优先复用既有 BDD 步骤短语；逐条决策与理由见 research/compaction-plan.md 与 research/decisions.md。
- 合并 3：r1758→r1727、r1756→r1841（跨能力并入 user-experience）、r1768+r1769→r1767（标题取存活规则）。
- 移除 10（零实现，经用户批准）：infra-network 整系 r1455–r1460（capability 一并删除）、r1754、r1757、r31、r1089。
- 保留 373：裸规则维持，理由码 (a) 规范自declared 单测覆盖 / (b) 已被既有场景覆盖 / (c) 缺断言面 / (d) 结构与流程契约。
- 预计 pending：477 → 373（全部落地后）。

## Scope

- `llmanspec/specs/**/*.feature`（含删除 `llmanspec/specs/infra-network/`）
- `tests/bdd/**`（新增步骤实现与 `#[scenario]` 绑定；不改既有步骤短语）

## Out of Scope

- 不改任何规则的规范性正文（合并/移除项除外，均为独立批准项）。
- 不新增生产插桩（缺断言面的 (c) 项全部保留不动）。

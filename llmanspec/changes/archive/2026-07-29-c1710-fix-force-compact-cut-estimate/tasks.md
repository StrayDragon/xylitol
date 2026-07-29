# Tasks: c1710-fix-force-compact-cut-estimate

## 1. Specs

- [x] 1.1 修订 live `domain-compaction`：`c8`/`c17`（及必要时新增 c25）写明 leaf 分支输入 + 切点计量对齐 pi `estimateTokens`；更新 `domain-compaction.feature` `@req:c17`（误报 too small / 旁支不泄漏 / Already compacted）

## 2. Implementation

- [x] 2.1 Session 侧暴露 compact 用 leaf 分支加载（复用 `get_branch`；port 若缺则最小扩展）；Orchestrator / `compact_session` / prepare 改吃分支
- [x] 2.2 实现与 pi 同构的 message 级切点 token 估计；`find_cut_point` 改用该估计并 skip 零贡献
- [x] 2.3 单测：低估回归样例、旁支不泄漏、Already compacted、真短会话

## 3. Gate

- [x] 3.1 相关 lib/BDD 绿；`llman sdd validate c1710-fix-force-compact-cut-estimate --strict`

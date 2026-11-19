# Tasks

地图推进（每票完成后在本图 Decisions so far 追加一行；顺序即依赖解锁序，T2–T4 互相独立可换序）。

## T1 关闭 c3375 provider 脚手架归属

- [x] 逐问深挖：分件归属清单、thinking 收敛深度、is_opencode_host 处置、新方言成本基线、实施路径。
- [x] 决策回写 c3375 proposal，change start/archive 关闭，地图索引追加。
- 验证：archive/2026-11-18-c3375-refactor-provider-scaffold-home 已存在；`llman-sdd validate c3350-refactor-dedup-convergence-map` failed 0。

## T2 关闭 c3380 截断策略模块

- [x] 逐问深挖：接口形状、常数归属、文案路线（组合式 vs propose 改 r1431/r1345）、r1140 处置、迁移顺序。
- [x] 决策回写 c3380 proposal，关闭并归档，地图索引追加。
- 验证：archive/2026-11-18-c3380-refactor-truncation-policy-module 已存在；`llman-sdd validate c3350-refactor-dedup-convergence-map` failed 0。

## T3 关闭 c3385 dispatch 合约归属

- [x] 逐问深挖：sdk 落点确认、数据类型下沉清单、错误 kind 化最小面、改动点数预期。
- [x] 决策回写 c3385 proposal，关闭并归档，地图索引追加。
- 验证：archive/2026-11-18-c3385-refactor-dispatch-contract-home 已存在；`llman-sdd validate c3350-refactor-dedup-convergence-map` failed 0。

## T4 关闭 c3390 lsp/dap 生命周期组件

- [x] 逐问深挖：相位机接口、错误分类器注入与单测形态、c3300 表述承接。
- [x] 决策回写 c3390 proposal，关闭并归档，地图索引追加。
- 验证：archive/2026-11-18-c3390-refactor-agent-lifecycle-component 已存在；`llman-sdd validate c3350-refactor-dedup-convergence-map` failed 0。

## T5 实施分组决策

- [x] 依四票产出决定收拢方式（一张 propose vs 每票各走），记录于本图 Decisions so far。
- 验证：本图尚未清晰区已清零（2026-11-18）。

## T6 图收口

- [x] 收拢方式由 T5 裁定：四条独立实施线（c3375 / c3380 / c3385 propose、c3390 quick），不建单张收拢计划；本图绑定分支后 finalize/archive。
- 验证：archive/ 下存在本图目录；active changes 不再含本图。

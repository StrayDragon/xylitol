---
depends_on: []
title: compaction 触发阈值场景收敛为场景大纲（Scenario Outline）
---

## Why

`domain-compaction` r1406 下 5 个触发阈值场景（need-compact / no-compact / disabled-no-compact / floor-threshold-holds / floor-cross-triggers）步骤完全同构、仅参数不同，以 5 个独立 `场景:` + 5 个 `#[scenario]` binding 表达存在重复：参数矩阵改动（如新增一档边界值）须同步改 5 处。rstest-bdd 0.6 已原生支持 Scenario Outline（编译期按 Examples 行展开为独立 `#[case]`，占位符编译期校验），llman-sdd 0.8.2 spec-parsing 亦原生解析 `场景大纲` / `例子` IR——基础设施就绪，先以本组单点验证效果。

## What Changes

- `llmanspec/specs/domain-compaction/domain-compaction.feature` r1406：5 个同构 `场景:` 收敛为 2 个 `场景大纲:`（reserve 闸组 3 行 / floor 组 2 行），语义与参数矩阵不变。
- `tests/bdd/bindings_domain_compaction.rs`：对应 5 个 `#[scenario]` 收敛为 2 个（outline 形态，签名带 Examples 列参数）。
- `tests/bdd/steps_compaction.rs`：新增 1 条参数化 given `compaction enabled 为 {flag}`（bool），替代 true/false 两条硬编码 given 在大纲内的使用；原两条 given 保留供其余场景使用。
- 行为合约语义不变：5 个场景的输入输出矩阵逐一保持；`cargo test --test bdd` 行展开后用例数不减。

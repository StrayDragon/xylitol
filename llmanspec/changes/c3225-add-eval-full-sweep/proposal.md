---
depends_on:
  - c3006-add-minimal-eval-harness
title: "Single representative eval baseline"
needs_specs_change: false
---

# Single representative eval baseline

> **narrowed（2026-11-15）**：原「DeepSWE 10 + TB4 10 全量 sweep」收窄为单 task
> baseline。全套留到 1.0.0 前另开 change；本 change 不锁 full task 集，避免提前锁定。
> 目录名仍为 `c3225-add-eval-full-sweep`，保留历史 id。

## Why

全套 sweep 的成本与锁定成本都高：task id、checksum、镜像、报告口径一旦钉死，后续
调整要重做。当前只需要一个能代表 coding agent 行为的回归信号，用来判断 xylitol 改动
是否退步；全套对比留到 1.0.0 发布前。

## What Changes

- 只保留一个代表性 DeepSWE task：`actionlint-action-pinning-lint`。
- 该 task × N=3 × xylitol；1800s wall、80 steps；同模型 `group/glm-5.3-flash`。
- 新增 `just eval-one`；移除 `eval-typical` / `eval-typical-both`。
- 用 `examples/eval-one.yaml` 替换 `examples/eval-terminal-bench.yaml`；同步改测试。
- `model-lock.yaml` 只保留 `deepswe.baseline_id` 与该 task 的 image digest；不写
  `full_ids` / `full_checksums` / `terminal_bench.ids`。
- README 改为单 task 用法；删除 TB4 与 5-task 段。
- 报告只需一页：task、N、pass@1、wall、cost、commit/SHA、Harbor 版本。
- 保留 `just eval-smoke` 作为框架 canary；它不是分数，不替代 baseline。

## 为什么选 actionlint

- 5-task baseline 中 xylitol 与 Pi 都是 1/3：有 headroom，不饱和（`abs` 3/3），
  不触底（`adaptix` 0/3）。
- 0 exceptions；无 timeout 噪声（`arcane` 有 Pi 1800s 超时）。
- 真实 Go 仓库 + lint 修复 + 测试执行，覆盖 repo 编辑与验证链路。
- 备选：`anko-typed-variable-bindings`（同样 1/3，Go 解释器类型绑定）。

## Non-goals

- 不做 DeepSWE 10 / TB4 10 全套。
- 不跑 Pi；Pi 对比留到 1.0.0 全套。
- 不锁 full task ids / checksums / TB4 ids。
- 不做 CSV / 报告模板 / 发布表；单 task 用现有 `summarize_harbor.py` 足够。
- 不做 CI / 观测 / 云。

## Acceptance

- `just eval-one` 可跑 `actionlint-action-pinning-lint` × N=3 × xylitol。
- 一页 baseline 记录 task、N、pass@1、wall、cost、commit/SHA、Harbor 版本。
- 同 commit + lock 重跑，结果可由 `summarize_harbor.py` 重建。
- `just eval-test` 与 `just eval-smoke --check` 绿。

## Risks

- 单 task N=3 噪声大；只作回归 canary，不作能力结论。
- task 可能饱和或触底；选 1/3 的 actionlint 降低该风险。
- 移除 5-task / TB4 后，历史报告仍可读，但日常入口只剩 `eval-one` 与 `eval-smoke`。
- `actionlint` 随镜像/verifier 变化时需更新单条记录，不涉及全量集。

## Further Notes

- 全套 sweep 在 1.0.0 前另开 change；届时再钉 task 集与 checksum。
- 历史 baseline：`llmanspec/changes/archive/2026-11-09-c3006-add-minimal-eval-harness/research/baseline-2026-11.md`。

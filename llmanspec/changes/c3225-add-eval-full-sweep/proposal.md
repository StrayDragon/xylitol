---
depends_on:
  - c3006-add-minimal-eval-harness
title: "Full eval sweep + reproducible release report SOP"
needs_specs_change: false
---

# Full eval sweep + reproducible release report SOP

> **promoted（2026-11-15）**：从 `delayed-changes/` 迁回；按当前 `tools/eval`、
> `model-lock.yaml` 与 `justfile` 重适配。本 change 只交付方案，不进入实现。

## Why

第一个公开版需要可对比的指标表：同一模型、同一任务、同一 verifier 下，xylitol 与 Pi 的
pass@1、flip rate、cross-scaffold gap、cost/wall。c3006 交付 5-task typical；
公开版需要 DeepSWE 10 + TB4 10，并且报告必须能按固定步骤复现，不能靠手工拼数。

## What Changes

- 固定 DeepSWE 10 task 集：5 个 typical + 5 个补充 task；task id 与 checksum 写入
  `tools/eval/model-lock.yaml`。
- 跑 DeepSWE 10 task × N=3 × xylitol/Pi，1800s wall、80 steps，同模型同预算。
- 跑 TB4 10 task × xylitol N=2；不跑 Pi；floor 结论单列，不并入主分。
- 新增 `just eval-full`（DeepSWE 10）与 `just eval-tb4`（TB4 10）配方。
- 扩展 `tools/eval/scripts/summarize_harbor.py`：CSV 输出、无效 trial 计数、N、
  commit/SHA、per-benchmark 汇总。
- 新增报告模板与 SOP：固定输入、运行、无效 trial 处理、聚合、发布表格。
- 发布报告包含：per-task pass@1、overall pass@1、flip rate、cross-scaffold gap、
  token/cost、wall time、N、异常 trial 数、模型/任务/verifier 口径。

## Report SOP（必须可复现）

1. 固定输入：model lock、task ids + checksums、Harbor 版本、binary SHA-256、git commit。
2. 固定预算：DeepSWE 1800s wall、80 steps、N=3；TB4 1800s wall、80 steps、N=2；
   xylitol 与 Pi 同预算。
3. 运行：每个 scaffold/benchmark 单独 job；记录 `jobs/<job>/result.json` 与 per-trial
   `result.json`。
4. 无效 trial：quota、provider error、sidecar 平台错误、verifier infra error 标为无效并重跑；
   agent timeout 计为失败，不剔除；无改动导致 reward 0 计为失败，不剔除。
5. 聚合：先按 task 聚合 pass@1，再取 scaffold 总体；flip rate 与 cross-scaffold gap
   分开算；TB4 单列。
6. 发布：生成 Markdown 表 + CSV；原始 `jobs/` 与报告一起归档；注明 commit / SHA / N。
7. 复核：同一 commit + 同一 lock 重跑一次，确认表格可由脚本重建；不要求 trial 级完全一致。

## Non-goals

- 不做 CI / 观测 / 云（Phase 5.1–5.3 不在本 change）。
- 不跑 TB4 Pi；不把 TB4 floor 混进 DeepSWE 主分。
- 不自研 judge；分数只来自 Harbor verifier。
- 不承诺“xylitol 比模型强”；报告只写 scaffold 对比。
- 不改产品默认行为；eval 是开发者工具链。

## Acceptance

- DeepSWE 10 × N=3 × 2 scaffold 完成，无未解释异常。
- TB4 10 × xylitol N=2 完成；floor 结论单列。
- 报告可由脚本从 `jobs/` 重建；表格含 N、异常数、口径。
- 同一 commit + lock 重跑，聚合结果可由脚本复现。
- `just eval-full` 与 `just eval-tb4` 可直接复跑。

## Risks

- 全量 token / wall 成本高；先估算预算，必要时分批跑。
- TB4 floor 明显，区分度低；报告必须标注。
- N=3 仍有噪声；公开表要给 N 与异常数，不把单次差异写成能力结论。
- 5 个补充 DeepSWE task 的镜像与 verifier 稳定性未验证；先做单 task 冒烟。
- quota / 网络不稳定；无效 trial 规则必须先定。

## Further Notes

- 依赖 c3006 的 5-task baseline、预装 Pi 镜像、`just eval-typical`。
- 现有聚合脚本：`tools/eval/scripts/summarize_harbor.py`；已有 pass@1 / flip /
  cross-scaffold / cost / wall 计算，缺 CSV 与无效 trial 计数。
- 公开版报告是发布物；SOP 与模板落在 `tools/eval/`，不写进 live spec。

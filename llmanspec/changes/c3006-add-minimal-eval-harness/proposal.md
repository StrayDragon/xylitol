---
depends_on: []
title: 最小 Agent Eval 底座（SWE-bench headless）
needs_specs_change: false
---

# 最小 Agent Eval 底座（SWE-bench headless）

> **purpose-draft**：记录调研与分阶段候选，未绑定分支、未落 live specs、不可 apply。
> 落实时经 `llman-sdd-propose` 正式化并补 tasks。

## Why

换模型、改系统提示或工具策略后，目前只能靠主观感受判断 xylitol 是否进步；没有可复现、
可审计的分数。社区已有官方 grader（SWE-bench `run_evaluation` / Harbor verifier），
不需要自研判分器。缺的是「headless 驱动 + 每实例隔离 + 交卷 + 官方判分」这条最小链路。

## What Changes

- 新增 Python eval driver（`tools/eval/` 或先落 `scripts/lab_eval_swebench.py`）：
  - 固定 dataset revision + 显式 instance ids；
  - 每实例独立 Docker 容器（官方/预构建 SWE-bench 镜像）；
  - 容器内 headless 运行 `xylitol print`；
  - 收集 `git add -A && git diff --cached` 为 unified diff；
  - 写 `preds.jsonl`（`instance_id` / `model_name_or_path` / `model_patch`）；
  - 调官方 `python -m swebench.harness.run_evaluation` 出 `results.json`；
  - 产出 `run.json` / `summary.md`（binary hash、config hash、dataset revision、harness 版本）。
- M0 只跑 **SWE-bench Verified 5 实例**；验收是 5/5 被 grade（不是 5/5 resolved）。
- M1 补 eval profile：墙钟超时、`session.max_turns`、无 ask、config 锁定、成本/token 报告；
  超限 autosubmit 交卷协议若需 runtime stop hook，另开 change。
- M2 扩到固定 Verified 50 并跑 mini-SWE-agent 同口径 baseline；M3 再接 Harbor /
  Terminal-Bench，单独提案。
- 不改变产品默认行为；eval 是开发者/CI 后置能力，不进开箱体验。

## 决策

- **官方 grader 为唯一分数源**；Langfuse 只作旁路元数据/失败实例回归，不作分数。
- **只经 `print` / headless 驱动**；禁止 PTY / 自动按键驱动 TUI 作为 eval 前置。
- **Python 编排**，不在 Rust runtime 引入 Docker/benchmark 依赖；Rust 侧只提供稳定 binary。
- M0 用 **Verified 5** 冒烟；M2 再扩到固定 50 子集并跑 mini-SWE-agent 同口径基线。
- Terminal-Bench 走 Harbor（当前 4.0，66 tasks）单独提案，不塞进 M0。

## Capabilities

- 无 live spec 行为变更（`needs_specs_change: false`）；eval 是开发工具链。
- 若 M1 新增 runtime 停止条件（cost/token），届时按 `agent-runtime` 走 SDD。

## Impact

- 新增 `tools/eval/`（或 `scripts/lab_eval_swebench.py`）+ 测试。
- 不改线协议、不改产品 TUI / Print 行为。
- 可能新增 `just qa-eval-smoke`（手动/夜间，非默认门禁）。

## Further Notes

一手调研与选型对照：[research/minimal-eval-harness.md](./research/minimal-eval-harness.md)。

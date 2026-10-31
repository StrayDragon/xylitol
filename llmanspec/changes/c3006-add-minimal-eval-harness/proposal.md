---
depends_on: []
title: 最小 Agent Eval 底座（SWE-bench headless）
needs_specs_change: false
---

# 最小 Agent Eval 底座（SWE-bench headless）

> **planned**：调研、设计与任务已收敛；按用户要求暂不建分支、不 push、不开 PR。
> 落实时走 apply；确认后再开 PR。

## Why

换模型、改系统提示或工具策略后，目前只能靠主观感受判断 xylitol 是否进步；没有可复现、
可审计的分数。社区已有官方 grader（SWE-bench `run_evaluation` / Harbor verifier），
不需要自研判分器。缺的是「headless 驱动 + 每实例隔离 + 交卷 + 官方判分」这条最小链路。

## What Changes

- 新增 `tools/eval/` Python 包（一步到位骨架，阶段验收）：
  - 固定 dataset revision + 显式 instance ids；
  - 每实例独立 Docker 容器（数据集自带 `image` 字段）；
  - 容器内 headless 运行 `xylitol print`；
  - 收集 `git add -A && git diff --cached` 为 unified diff；
  - 写 `preds.jsonl`（`instance_id` / `model_name_or_path` / `model_patch`）；
  - 调官方 `python -m swebench.harness.run_evaluation` 出报告；
  - 产出 `run.json` / `summary.md`（binary hash、config hash、dataset revision、harness 版本）。
- 新增仅 `workflow_dispatch` 的 Linux binary CI（不做 nightly）；构建产物供本地评测下载。
- 确认前不 push，CI 暂不可用；M0 的 Linux binary 先在本地 Docker 构建。
- 新增 `just eval-test` / `eval-dryrun` / `eval-smoke`（均非默认 `just qa`）。
- 不新增 live spec；工具契约写在 `tools/eval/README.md` 与本 change 的 `design.md`。
- 不改变产品默认行为；eval 是开发者/CI 后置能力，不进开箱体验。

## 决策

- **官方 grader 为唯一分数源**；Langfuse 只作旁路元数据/失败实例回归，不作分数。
- **只经 `print` / headless 驱动**；禁止 PTY / 自动按键驱动 TUI 作为 eval 前置。
- **Python 编排**，不在 Rust runtime 引入 Docker/benchmark 依赖；Rust 侧只提供稳定 binary。
- **只用 magpie 提供的模型**；不使用 `DEEPSEEK_API_KEY`。模型 id 直接用网关 catalog 输出（如 `zhipu/glm-5.3`），不手写别名。
- **一步到位设计 `tools/eval/`**；阶段验收，接口先定，后续适配器不预埋空壳。
- **数据集 / harness 组合**：`SWE-bench/SWE-bench_Verified` + `swebench==5.0.2` + 数据集自带
  `image` 字段；不再使用 `--cache_level` / `--namespace`（5.0.2 已移除）。
- **实例 id 与镜像名分离**：`instance_id` 用数据集字段（如 `astropy__astropy-12907`）；
  镜像取 `image` 字段（如 `swebench/sweb.eval.x86_64.astropy_1776_astropy-12907:latest`）。
- **CI 仅手动触发**：`workflow_dispatch` 构建 Linux binary；不做 nightly。
- **模型与密钥不外露**：仓库只写占位符（`{{ secret.NAME }}`）；真实 key 只在本地 env / CI secrets。
- **确认前不开 PR、不 push**；避免未审查代码与密钥泄露。

## Capabilities

- 无 live spec 行为变更（`needs_specs_change: false`）；eval 是开发工具链。
- 若后续新增 runtime 停止条件（cost/token），届时按 `agent-runtime` / `runtime-config` 另开 SDD change。

## Impact

- 新增 `tools/eval/`（Python 包 + 测试 + 示例配置）。
- 新增 `.github/workflows/eval-bin.yaml`（仅手动触发）。
- 新增 `just` 配方（非默认门禁）。
- 不改线协议、不改产品 TUI / Print 行为。

## 阶段

| 阶段 | 交付 | 验收 |
|---|---|---|
| M0 | `tools/eval` 骨架 + Docker + Verified 5 + 官方 grader | 5/5 被处理；error=0；报告可复现 |
| M1 | eval profile：墙钟 / max_turns / 配置锁 / 无 ask | 超限仍收 patch；config hash 稳定 |
| M2 | Verified 50 + mini-SWE-agent baseline | 同模型同数据对比 |
| M3 | Harbor `BaseInstalledAgent` + Terminal-Bench 10–20 | reward 来自 verifier |
| M4 | 失败实例 → Dataset；小集 CI | 回归集可跑 |
| M5 | 云并行全量 / leaderboard 提交 | 全量可复现 |

## 风险

- 模型网关 key 只在本地 env；仓库不得出现真实 key。
- Linux binary 与 SWE-bench 镜像 glibc 兼容性需实跑确认。
- M2 50 实例镜像磁盘占用高，需逐实例清理或扩容。
- 官方榜是 bash-only / mini-SWE-agent 口径，不能直接当 xylitol 分数。
- GLM-5.3 价格约为 GLM-5.3-Flash 的 10 倍；小集/全量前需确认预算。

## Further Notes

- 一手调研与选型对照：[research/minimal-eval-harness.md](./research/minimal-eval-harness.md)。
- 2026-10-31 官方评分与候选模型复核：[research/benchmark-scores-2026-10.md](./research/benchmark-scores-2026-10.md)。

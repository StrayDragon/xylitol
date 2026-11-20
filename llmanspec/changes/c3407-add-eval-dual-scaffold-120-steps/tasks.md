# Tasks

## T0. 确认已落地配置

- [ ] T0.1 确认 `model-lock.yaml` compare=120、`eval-one.yaml` agent=120、`justfile` 两个 scaffold 都是 120。
- [ ] T0.2 确认 `preinstalled_agents.py` 存在且 `just eval-one pi` 分支可达。
- [ ] T0.3 `just eval-test` 22 passed。
- [ ] T0.4 `just --dry-run eval-one xylitol` / `pi` 都显示 120。

## T1. 准备 Pi

- [ ] T1.1 重新准备 `/tmp/deep-swe-agents/actionlint-action-pinning-lint` 与 Pi preinstalled 镜像。
- [ ] T1.2 记录 Pi 任务目录与镜像 digest。

## T2. 跑 baseline

- [ ] T2.1 `just eval-one xylitol`，N=3，记录 jobs 路径。
- [ ] T2.2 `just eval-one pi`，N=3，记录 jobs 路径。
- [ ] T2.3 用 `summarize_harbor.py` 汇总两者；记录 pass@1、F2P/P2P、wall、commit/SHA。
- [ ] T2.4 更新 `tools/eval/reports/actionlint-baseline.md` 与 #21。

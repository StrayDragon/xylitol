---
title: "Eval dual-scaffold 120-step baseline"
depends_on:
  - c3225-add-eval-full-sweep
needs_specs_change: false
---

# c3407 — Eval dual-scaffold 120-step baseline

## Why

c3225 的单 task baseline 在 80 步、xylitol only 下跑出 0/3。日志显示 3 个 trial 都触发 `[End] step_limit`（Turn 79），未提交 commit；collect hook 取到空 patch，verifier 在 base 仓库上得到 F2P 0/55、P2P 145/145。直接原因是 step 预算耗尽，不是 verifier 或环境错误。

用户决定把 step 上限放宽到 120，并让 xylitol 与 Pi 都参与。旧 5-task baseline（同模型、N=3）中 xylitol 7/15、Pi 6/15；actionlint 两边都是 1/3。两边大致打平，120 步是给两者同预算的公平对比。

## What Changes

- `model-lock.yaml` 的 `compare.max_steps_per_turn` 改为 120；`smoke` 保持 8。
- `examples/eval-one.yaml` 的 `agent.max_steps_per_turn` 改为 120。
- `just eval-one` 增加 `scaffold` 参数：`xylitol` / `pi`；两者都用 120 步。
- 恢复 `tools/eval/harbor/preinstalled_agents.py`；Pi 走 `preinstalled_agents:PiPreinstalled`。
- README 改为双 scaffold、120 步；注明 Pi 需要 `/tmp/deep-swe-agents/actionlint-action-pinning-lint`。
- `tests/test_agent.py` 断言改为 120。
- 下次跑 actionlint × N=3 × xylitol/Pi，更新 baseline 报告与 #21。

## 已落地

代码已在 `308b687`（120 步 + Pi）与 `53f1d70`（README 对齐）落地；本 change 记录计划与待跑验证，不重复改代码。

## Non-goals

- 不做 DeepSWE 10 + TB4 10 全套。
- 不换模型、不改 task、不改 verifier。
- 不绑定分支；计划文档只在 main。

## Acceptance

- `just eval-one xylitol` 与 `just eval-one pi` 都使用 120 步。
- `just eval-test` 22 passed。
- 配额允许时，actionlint × N=3 × xylitol/Pi 各跑一次；报告记录 pass@1、F2P/P2P、wall、commit/SHA。
- 报告与 #21 更新；Pi 前置镜像/任务目录已记录。

## Risks

- 120 步仍可能不够；若再次 `[End] step_limit`，按 c3408 的 guardrail 处理。
- Pi 镜像与 `/tmp/deep-swe-agents` 已在上次清理删除；下次跑 Pi 前需重新准备。
- 单 task N=3 噪声大；只作回归 canary，不作能力结论。

## Further Notes

- 旧 5-task baseline：`llmanspec/changes/archive/2026-11-09-c3006-add-minimal-eval-harness/research/baseline-2026-11.md`。
- 0/3 诊断：`tools/eval/reports/actionlint-baseline.md`。

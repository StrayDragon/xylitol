---
title: "Eval baseline guardrails and debug runbook"
depends_on:
  - c3225-add-eval-full-sweep
needs_specs_change: false
---

# c3408 — Eval baseline guardrails and debug runbook

## Why

c3225 的 0/3 不是 verifier 或环境错误：3 个 trial 都触发 `[End] step_limit`（Turn 79），未提交 commit；collect hook 的 `git diff base..HEAD` 为空，`model.patch` 被跳过；verifier 在 base 仓库上得到 F2P 0/55、P2P 145/145。当前 harness 把「没有 patch」与「patch 失败」都压成 0 分，报告会误导。

## What Changes

- 空/缺失 `model.patch` 标为 invalid，不计 0；报告单列 invalid trials。
- collect 前 commit guard：工作树 dirty 时 `git add -A && git commit`（或 collect 工作树 diff + untracked）。
- `summarize_harbor.py` 识别 `[End] step_limit` / wall timeout / empty patch，报告区分。
- README 增加 Harbor debug runbook：`harbor view jobs`、`harbor analyze`、`harbor job resume/regrade`、per-trial artifacts、`xylitol tui --session`。
- 保留 `tools/eval/reports/actionlint-baseline.md` 的 Diagnosis 段（`8d907dc` 已落地）；本 change 验证并固化。

## Non-goals

- 不改 task、verifier、模型；不做全量 sweep。
- 不改变正常 patch 的评分语义。

## Acceptance

- 空 patch trial 标 invalid，不产生 0 分。
- dirty tree 在 collect 前被提交，或明确拒绝并标 invalid。
- debug runbook 可照做；`just eval-test` 绿。

## Risks

- commit guard 可能把 agent 的未完成改动提交；须在 collect 前判断，并在报告标注。
- invalid 语义改变历史对比；旧 0/3 保留为 invalid，不重算为 0。
- 任务侧 collect hook 在 task.toml 内，改动可能触及第三方 task 副本；优先在 wrapper/summarizer 层实现。

## Further Notes

- 0/3 诊断：`tools/eval/reports/actionlint-baseline.md`。
- 日志证据：`jobs/.../artifacts/manifest.json`（`model.patch` skipped）、`verifier/ctrf.json`（200 tests/145 passed/55 failed）、`agent/xylitol.txt`（`[End] step_limit`）。

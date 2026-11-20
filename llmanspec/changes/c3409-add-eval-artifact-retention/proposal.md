---
title: "Eval artifact retention and cleanup"
depends_on:
  - c3225-add-eval-full-sweep
needs_specs_change: false
---

# c3409 — Eval artifact retention and cleanup

## Why

2026-11-15 清理删除了 `jobs/one-actionlint-action-pinning-lint`、`/tmp/deep-swe`、`/tmp/deep-swe-agents` 与约 36GB 未用镜像；最新报告保留。没有保留策略时，删除 job 会丢掉 0/3 诊断证据；下次跑 Pi 又需要重新拉镜像。需要一个可重复、先 dry-run 的清理脚本与保留规则。

## What Changes

- 新增 `tools/eval/scripts/cleanup_artifacts.sh`（或 `just eval-clean`）：dry-run 列出将删除的 jobs/tmp/images；`--apply` 只删过期项。
- 保留策略：最新报告 + 最新 job（默认 14 天）；actionlint task 镜像 + egress sidecar 常驻；Pi 镜像按需。
- README 记录保留策略与清理命令。
- 记录 2026-11-15 清理：jobs/tmp 删除，报告保留；Pi 镜像删除，下次重拉。

## Non-goals

- 不删最新报告；不删当前 binary；不删 `/tmp/harbor-venv` 与 `/tmp/magpie-harbor.env`。
- 不做自动定时清理；脚本手动跑。

## Acceptance

- dry-run 不删除；`--apply` 只删过期；最新报告保留。
- README 有保留策略；`just eval-test` 绿。

## Risks

- 误删正在使用的 job/image；脚本先 dry-run、显式 `--apply`、路径校验。
- Pi 镜像删除后下次跑 Pi 需重拉；README 注明。
- 保留期太长会占磁盘；默认 14 天，可配置。

## Further Notes

- 2026-11-15 清理记录：`jobs/` 删除；`/tmp/deep-swe` / `/tmp/deep-swe-agents` 删除；未用镜像删除；最新报告 `tools/eval/reports/actionlint-baseline.md` 保留。

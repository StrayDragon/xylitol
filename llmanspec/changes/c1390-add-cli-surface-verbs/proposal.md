---
change_id: c1390-add-cli-surface-verbs
title: CLI 统一入口：surface 动词 tui/print + 默认 TUI 不变
status: purpose-draft
priority: 1390
depends_on:
  - c1380-add-cli-tokenizer-cache
author: agent
track: B
wave: cli-surface
domain: cli
apply_band: P1-after-c1380
branch: feat/c1380-add-cli-tokenizer-cache
base_sha: a481fdb0bb9ffe675daa024edab5f5d7c9310105
checkpointed: false
---

# c1390-add-cli-surface-verbs

> **purpose-draft（已钉树，待 c1380 后再升 full / apply）**
> 命令树已产品约定：默认 TUI 不变；`tui`/`print` 为 surface；`resources`/`server`/`tokenizer` 为顶层 ops。
> live `cli-entry` 已预写 **ce16** 与场景，避免遗忘；实现前将本 change 升为 `full` 并勾 tasks。

## Why

1. xylitol CLI 将是 **多端统一入口**（TUI / print / server / 未来 inspect·Web），今日把面切换挤在 flat `--tui` / `--print`，与 ops（`resources` / `server` / `tokenizer`）混层，难扩展。
2. 对照 pi：管理类用顶层 Commands、默认进交互——可学；但 pi **没有**多表面命名空间。我们需要更清晰的 **surface vs ops**，且**不照搬** pi 扩展包模型。
3. 产品已约定命令树；本 change 把约定写成可验证合约，默认裸跑进 TUI **行为不变**。

## Purpose

落地统一入口分层：

```text
xylitol                         → 默认 TUI（MUST 保持）
xylitol tui [run]               → 显式进 TUI（与默认同义）
xylitol tui <专有…>             → 仅 TUI 的运维/诊断占位（本波可极少叶子）
xylitol print [--] <prompt>     → 一次性对话（收编 --print / -p / 位置 prompt）
xylitol resources|server|tokenizer …  → ops（已有或由 c1380 交付；本 change 不搬迁）
共享：--model / --session / --config / --trust
```

兼容：短期内保留 `--tui` / `--print` / `-p` 为别名（文档主推动词树）；弃用节奏见 design。

## What Changes

- `CliCommand` 增加 `Tui` / `Print` 表面动词；解析与 dispatch 分层
- 默认无子命令且 TTY → TUI（ce12 保持）
- `print` 子命令要求非空 prompt（对齐 ce13）
- Specs：`cli-entry` 表面 vs ops 合约；BDD：裸跑、`tui`、`print`、ops 仍在顶层
- 文档：`--help` Commands 分区可读（surface / ops）

## Capabilities

- `cli-entry`（modify）

## Out of scope

- 实现 c1380 tokenizer 行为（depends_on；本 change 只要求 ops 顶层位稳定）
- Web / inspect 表面动词（后置 roadmap）
- 删除 slash；改 TUI 内部命令
- 写入 architecture 未兑现 Web 面

## Ethics

- risk_level: low
- prohibited_actions: 改变「TTY 裸跑进 TUI」默认；把 tokenizer/resources 塞进 `tui` 子树
- required_evidence: help/BDD 证明默认 TUI、`print` 需 prompt、ops 仍顶层
- escalation_policy: 若 clap 位置参数与 `print` 子命令冲突难解，升级用户确认迁移窗口

## Depends

- `c1380-add-cli-tokenizer-cache`（tokenizer 顶层 ops 先落地或并行合约；apply 本 change 前 c1380 应已归档或同分支已实现顶层 `tokenizer`）

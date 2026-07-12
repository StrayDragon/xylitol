---
change_id: c474-update-cli-default-tui
title: "CLI：无参默认进 TUI；print 仅 --prompt/位置参数"
status: ready
priority: 474
depends_on: []
author: agent
track: B
---

# c474-update-cli-default-tui

## Why

产品默认入口应是交互 TUI（对齐个人 coding agent 体感），不应依赖 `--tui` 心智。当前虽有「无 prompt + TTY → TUI」分支，但 print 路径仍可无 prompt 落到 `"Hello!"`，且 `--prompt` 未固化为显式 one-shot 旗标，文档/帮助仍像「要加 --tui」。

## What Changes

1. **默认**：无 one-shot prompt、非 `--list-models`、stdin 为 TTY → `app::tui::run`（**不必**传 `--tui`）。
2. **print one-shot**：仅当提供位置参数 `PROMPT` 和/或 `--prompt <TEXT>`（或显式 `--print` 且有 prompt/管道 stdin）时进入 print。
3. **禁止**：无 prompt 的 print 路径默认 `"Hello!"`。
4. `--tui` 保留为显式强制进 TUI（可与 prompt 并存时仍进 TUI，prompt 可忽略或作首条预填——MVP：**强制 TUI，忽略 one-shot 跑 print**）。
5. 更新 `tui10` / cli-entry 相关合约与 `--help` 文案。

## Capabilities

- `cli-entry`
- `app-tui`（修改 tui10）

## Impact

- `src/app/cli/mod.rs`
- 帮助/文档指针

## Out of scope

- c490 trust ChoicePrompt
- c476 scrollback 富渲染
- 改 server/resources 子命令

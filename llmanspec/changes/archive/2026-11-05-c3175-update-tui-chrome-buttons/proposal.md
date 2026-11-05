---
depends_on: []
title: 固定输入框角标与按钮配色
branch: sdd/c3175-update-tui-chrome-buttons
base_branch: main
base_sha: 6b0f14445c813f9837684c2529fcbc1ccf7c9ba3
---

## Why

`◨` 替代 `╯` 导致右下角缺角；`❯` / `[▤ 摘要]` / `◨` 跟随 thinking 强度变色，与“按钮非边框”的语义不符。

## What Changes

- `◨` 固定到末行右端、`╯` 上方；下边框保留 `╯`。
- 窄档 `<80` 隐藏 `◨`，末行右端仍为 `│`。
- `❯`、`[▤ 摘要]`、`◨`（未激活）固定 thinking-off 色；边框/角仍跟随 thinking/bash 色。
- 侧栏开启时 `◨` 仍用 accent 反色。

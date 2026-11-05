---
depends_on: []
title: 摘要 chip 改为步骤显示级别并原地切换
branch: sdd/c3200-update-tui-display-level-chip
base_branch: main
base_sha: 16d24dde1159012b5016a6de6caaa03c94007f40
---

## Why

`[▤ 摘要]` 名称不表达当前级别；点击后只靠右上角通知，用户希望直接看到级别变化。

## What Changes

- chip 文案改为 `[步骤显示级别: <摘要|最近|完整|安静>]`。
- 左键点击原地循环级别并重绘 chip，不再推右上角通知。
- 右键重置折叠仍保留通知。

---
depends_on: []
title: 修复摘要通知、Thinking 预览与时长
branch: sdd/c3180-update-tui-thinking-notices
base_branch: main
base_sha: 571cd2ef47ef342981b555af7e84512680011912
---

## Why

`[▤ 摘要]` 左键在正文存在时不通知；activity fold 下折叠 Thinking 缺最新一行预览；live `Worked for` 没有时长，Thinking 时长在不足 1s 或 live 场景丢失。

## What Changes

- `[▤ 摘要]` 左键始终推右上角通知（显示等级中文名）。
- activity fold 折叠 Thought 头 追加该 cluster 最新 thinking 行预览。
- live `AgentStart` / `AgentEnd` 记录 segment 时钟；`Worked for` 至少显示秒。
- Thinking 时长允许 0s，并在 live / 历史 / activity fold 中显示与持久化。
- `❯` 改用 user 色（紫）；`[▤ 摘要]` 与未激活 `◨` 用 muted；边框仍跟随 thinking/bash。

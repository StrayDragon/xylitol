---
depends_on: []
title: 固定 status/spinner 行高
branch: sdd/c3195-update-tui-fixed-status-row
base_branch: main
base_sha: de262c25e7b9d713432ac4eba3c4f6ce32ffcbbe
---

## Why

busy 时 Loader 渲染“前导空行 + spinner”两行；idle 只返回一行空白，spinner 消失后固定区少一行，上方文字整体下移并抖动。

## What Changes

- idle status 槽也保留两行：前导空行 + 空 spinner 行。
- busy / idle / next-turn cue 三种状态 status 行高恒为 2。
- 长段落输出结束前后 editor 顶行不移动。

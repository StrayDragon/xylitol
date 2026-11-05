---
depends_on: []
title: 不足一秒改为 <1s 表达
branch: sdd/c3190-update-tui-subsecond-wording
base_branch: main
base_sha: 92684e8bb0142f90d9f3ac0d684e126435678f0e
---

## Why

`~1s` 仍暗示约 1 秒；用户希望直接表达“小于 1s”。

## What Changes

- `format_elapsed_secs(0)` 返回 `<1s`。
- `Worked for` / `Thought` 不足 1s 显示 `<1s`。

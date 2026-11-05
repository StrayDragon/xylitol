---
depends_on: []
title: 不足一秒统一显示 ~1s
branch: sdd/c3185-update-tui-worked-for-approx
base_branch: main
base_sha: 493feb414fb4053f982d337ea484890390606299
---

## Why

`Worked for 0s` / `Thought 0s` 读起来像没有计时；用户期望不足 1s 统一显示 `~1s`。

## What Changes

- `format_elapsed_secs(0)` 返回 `~1s`；`Worked for` 与 `Thought` 共用该口径。
- `format_duration` 不再单独判 0，统一走 `format_elapsed_secs`。
- 其余仍按 `Ns` / `Nm Ss`。

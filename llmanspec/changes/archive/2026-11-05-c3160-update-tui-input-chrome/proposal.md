---
depends_on: []
title: 调整输入区 chrome 与多行提示
branch: sdd/c3160-update-tui-input-chrome
base_branch: main
base_sha: d65dffc7c3bf267016de188ac70899b2697981a4
---

## Why

实机截图暴露输入区问题：`[侧栏]` chip 占上边框且与 `◨` 语义重复；`[重置折叠]` 无用且占下边框；圆角不跟输入框边框色；多行输入超过 5 行时 `↑/↓ N more` 被 chip 覆盖，用户看不到还有几行；spinner 脉冲过快。

## What Changes

- 删除 `[侧栏]` chip；`◨` 左键切换侧栏；窄档隐藏 `◨` 即无侧栏入口。
- 删除 `[重置折叠]` chip；右键 `[▤ 摘要]` 重置折叠到 summary 初始态，右上角通知成功/失败。
- 圆角 `╭╮╰╯` 与边框同色（跟随输入框 thinking/bash 色）。
- 多行输入保留编辑器 `↑/↓ N more` 提示，再拼阶段名/摘要 chip；边框不断裂。
- `[▤ 摘要]` 点击在无正文时给出通知反馈。
- spinner 改为 1.2s 正弦亮暗脉冲，100ms tick，不换字形。

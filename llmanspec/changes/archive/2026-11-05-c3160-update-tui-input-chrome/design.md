# Design — 调整输入区 chrome 与多行提示

## 目标

输入区上/下边框只保留必要信息；多行提示不丢；颜色与输入框一致；减少重复入口。

## 入口

- 上边框不再画 `[侧栏]`；`◨` 命中最后 2 列时调用 `FooterAction::Sidebar`。
- 下边框不再画 `[重置折叠]`；右键 `[▤ 摘要]` 调用 `reset_fold_to_initial()`。
- 左键 `[▤ 摘要]` 仍循环 display level；无正文时 push client notice 反馈当前档位。

## 边框组合

- `append_border_chips` 接收编辑器原始上边框；若含 `↑ N more` 保留该提示，再在右侧拼阶段名。
- `paint_bottom_chrome` 接收原始下边框；若含 `↓ N more` 保留，再在右侧拼 `[▤ 摘要]`。
- 角 `╭╮╰╯` 用 `operation_zone_paint()` 着色，不写裸字符。

## 重置

- `reset_fold_to_initial` 清全部 overrides 并应用 `ActivityFoldSettings::default()`；返回是否有变化。
- 成功 push `✓ 重置折叠成功`；已是初始态 push `◆ 折叠已是初始状态`。

## spinner

- 颜色闭包按 `Instant` 计算 `sin(t * 2π / 1.2)`，在 dim/accent 间插值；不再每调用翻转。
- Loader frames 仍只有 `⬢`，ANSI 不进 frame。

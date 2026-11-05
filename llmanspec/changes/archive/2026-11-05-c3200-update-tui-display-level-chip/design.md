# Design — 摘要 chip 改为步骤显示级别并原地切换

- `display_level_label()` 提供中文级别名。
- `bottom_chip_actions()` 动态返回 `[步骤显示级别: <label>]`；命中与绘制共用同一标签。
- `click_footer_action(FoldSummary)` 只 `cycle_display_level()`，不 push client notice。

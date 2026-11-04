# Tasks — TUI 活动折叠等级预设与子项可见性

- [ ] 1. 设置模型：新增 `DisplayLevel`（quiet/summary/recent/full）与 `VisibleItems`（thinking/tool_header/tool_output/diff）；`ActivityFoldSettings::from_preset`；默认 summary。
- [ ] 2. 配置：`tui.activity_fold.display_level` 与 `visible_items` 类型 / 默认 / 校验 / schema；显式旧字段覆盖预设。
- [ ] 3. ScrollbackFold：增加 thinking/tool_header/tool_output/diff 模式；`thinking_effective` 按模式；保留逐块覆盖。
- [ ] 4. 渲染：流式 thinking header 一行；cluster header summary/hidden；output/diff preview/full/hidden；full 档全部展开。
- [ ] 5. 动作与命令：`app.activity.displayLevel` 循环；`/fold` 查看/切档/reset/单项。
- [ ] 6. Specs：r1356/r1372/r1364/r1739/r3032 更新；新增预设与可见性规则。
- [ ] 7. 测试：四档 harness/snapshot；流式 thinking 高度稳定；逐项覆盖；配置非法值。

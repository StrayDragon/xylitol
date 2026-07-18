# Tasks: c1300-update-app-tui-write-edit-process-chrome

## Propose

- [x] proposal/design/tasks + live specs + attach

## 实施

- [ ] Tool 模型：write content → body；edit display_diff 合入同块
- [ ] scrollback：write Head-10 + ctrl+o；edit 默认显 diff；去 `[ok]`；路径缩短
- [ ] 停止成功 edit 的第二 `UiEntry::Diff`；更新 atb11 与单测
- [ ] session_tree 重建对齐
- [ ] `cargo test` bridge/scrollback + validate

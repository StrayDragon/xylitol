# Tasks: c1300-update-app-tui-write-edit-process-chrome

## Propose

- [x] proposal/design/tasks + live specs + attach

## 实施

- [x] Tool 模型：write content → write_content；edit display_diff 合入同块
- [x] scrollback：write Head-10 + ctrl+o；edit 默认显 diff；去 `[ok]`；路径 `~/`
- [x] 停止成功 edit 的第二 `UiEntry::Diff`；更新 atb11 与单测
- [x] session_tree 重建对齐（details.display_diff）
- [x] bridge/preview 拆分过预算；`cargo test` app::tui + validate

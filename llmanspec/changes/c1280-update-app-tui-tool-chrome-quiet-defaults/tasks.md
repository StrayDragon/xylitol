# Tasks: c1280-update-app-tui-tool-chrome-quiet-defaults

## Propose

- [x] live specs：强化 att13；新增 atb11 + feature:false 场景
- [x] `llman sdd change attach c1280-update-app-tui-tool-chrome-quiet-defaults`
- [x] proposal `status: full` + design/tasks

## 实施

- [x] `human_tool_args_preview`：write 行数；edit 从 edits 推断 path；禁止 JSON 回退
- [x] `ToolExecutionEnd`：write/edit 成功 quiet `Tool.output`
- [x] bridge 单测（缺 path / edits / content lines / quiet end）
- [x] `cargo test -p xylitol --lib app::tui::bridge` + `llman sdd validate` change

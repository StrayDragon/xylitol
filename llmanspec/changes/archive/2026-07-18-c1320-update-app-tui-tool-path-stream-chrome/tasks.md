# Tasks: c1320-update-app-tui-tool-path-stream-chrome

## Propose

- [x] proposal/design/tasks + live specs + attach

## 实施

- [x] `human_tool_args_preview`：`file_path`；空 path → `...`；read `:start[-end]`；write `write ... (N lines)`
- [x] upsert 粘性 path（`tool_path`）+ MessageUpdate 渐进刷新
- [x] ToolExecutionEnd 从 result 回填 path 并刷新 preview
- [x] 单测：path 流式 / 粘性 / read range / End 回填 / edit header 无 `:1`
- [x] `llman sdd validate` + `cargo test` bridge/preview/scrollback

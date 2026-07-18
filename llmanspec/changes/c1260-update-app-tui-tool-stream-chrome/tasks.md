# Tasks: c1260-update-app-tui-tool-stream-chrome

status: purpose-draft — promote 后再 apply。

## Promote 前

- [ ] live `app-tui-*` specs：工具旁路渲染、流式 args、人类摘要 MUST
- [ ] `llman sdd change attach c1260-update-app-tui-tool-stream-chrome`
- [ ] 确认 c1255 事件契约已稳定

## 实施

- [ ] bridge：MessageUpdate 中 tool 块 → 挂载/更新 Tool UI
- [ ] Assistant 渲染跳过 toolCall
- [ ] 内置摘要（bash/read/ls/edit 最小集）
- [ ] 执行 Update/End 状态与输出流式
- [ ] 快照或组件测试；`just test-tui` 相关
- [ ] validate change

# c80-add-tui Tasks

- [x] 实现 TUI app 主循环（crossterm event loop + ratatui 渲染）
- [x] 实现 ChatComponent（流式文本显示）
- [x] 实现 ToolOutputComponent（工具执行结果展示）
- [x] 实现 DiffPreviewComponent（变更预览）
- [x] 实现 ApprovalOverlay（命令审批流）
- [x] 实现会话/模型/主题选择器
- [x] 实现 Markdown 渲染（termimad 集成 + syntect 语法高亮）
- [x] 实现事件驱动 UI 更新（订阅 AgentEvent）
- [x] 编写 TUI 组件测试（TestBackend + insta 快照）— 基础单元测试已覆盖（组件测试见 Phase 2）
- [x] 编写 VT100Backend 完整渲染管线集成测试 — 由 c88 负责，此变更不做（design Non-Goals）
- [x] 图片协议预留（Kitty/iTerm2/Sixel，Phase 2）— design Non-Goals
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c80-add-tui --strict --no-interactive`

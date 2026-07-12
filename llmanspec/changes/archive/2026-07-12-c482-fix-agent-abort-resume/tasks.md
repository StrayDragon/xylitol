# Tasks — c482-fix-agent-abort-resume

- [x] 1.1 `ReActAgent`：run 入口重置 CancellationToken；abort 只取消当前 token
- [x] 1.2 abort 结束路径：AgentEnd / 非粘性 aborted；bridge idle
- [x] 1.3 单测：abort 后再 run 成功；可选 TUI harness
- [x] 1.4 `llman sdd validate c482-fix-agent-abort-resume --strict`

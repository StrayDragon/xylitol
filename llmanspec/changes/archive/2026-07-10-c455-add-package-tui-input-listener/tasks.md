# Tasks — c455-add-package-tui-input-listener

- [x] 1. `InputListenerResult` + `TUI` 存储 + `add_input_listener` / `remove_input_listener`
- [x] 2. `dispatch_event`：listener 循环置于 overlay/focus 之前
- [x] 3. 单元测试：consume-before-focus、FIFO、Continue 放行、Paste 路径
- [x] 4. `lib.rs` 按需 re-export
- [x] 5. `agent_demo`：注册 Ctrl+C / Esc listener；瘦身 `handle_input`
- [x] 6. demo 行为：Ctrl+C 清/退；Esc 流中 abort
- [x] 7. `agent_demo_test`（或 tui 单测）覆盖优先级
- [x] 8. 更新 `PI_DELTAS.md`；`llman sdd validate c455-add-package-tui-input-listener --strict`
- [x] 9. `cargo test -p xylitol-tui` + 相关 lint

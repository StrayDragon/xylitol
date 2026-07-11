# Tasks — c465-add-app-tui-bridge

- [ ] 1. 定义 UI-only 消息/状态类型（无 XyEvent）
- [ ] 2. 实现 `apply_xy_event` 单缝 + 未知变体降级
- [ ] 3. host 接线：`Driver::run` EventStream 进入 select! 合流
- [ ] 4. QueueUpdate → 状态/徽章；Turn/Agent 生命周期复位规则
- [ ] 5. 单测 / harness：事件序列 → UI 模型快照
- [ ] 6. `llman sdd validate c465-add-app-tui-bridge --strict --no-interactive`
- [ ] 7. `just lint` + TUI 相关测试（`test-tui-harness` 指针）

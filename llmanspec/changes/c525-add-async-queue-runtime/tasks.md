# Tasks — c525-add-async-queue-runtime

- [ ] 1. 按 c525 `design.md` 落地 QueueChannel（短锁 VecDeque + 活跃 EventStream 的 QueueUpdate）
- [ ] 2. steer/follow_up 两通道；并发 enqueue / mode / abort 单测
- [ ] 3. 删除生产路径 EventBus 队列旁路
- [ ] 4. Driver/Agent/ReAct 接线；`QueueStats` 结构化
- [ ] 5. 确认产品文档无实现细节泄漏（`docs/architecture/queue-and-interrupt.md`）
- [ ] 6. `llman sdd validate c525-add-async-queue-runtime --strict --no-interactive --stage spec`
- [ ] 7. `just lint` + queue/react 相关测试

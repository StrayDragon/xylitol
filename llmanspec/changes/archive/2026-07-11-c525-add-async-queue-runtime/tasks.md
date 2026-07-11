# Tasks — c525-add-async-queue-runtime

- [x] 1. 按 c525 `design.md` 落地 QueueChannel（短锁 VecDeque + 活跃 EventStream 的 QueueUpdate）
- [x] 2. steer/follow_up 两通道；并发 enqueue / mode / abort 单测
- [x] 3. 删除生产路径 EventBus 队列旁路
- [x] 4. Driver/Agent/ReAct 接线；`QueueStats` 结构化
- [x] 5. 确认产品文档无实现细节泄漏（`docs/architecture/插话续跑与中止.md`）
- [x] 6. `llman sdd validate c525-add-async-queue-runtime --strict --no-interactive --stage spec`
- [x] 7. `just lint` + queue/react 相关测试

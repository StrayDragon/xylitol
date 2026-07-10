# Tasks — c461-expose-steer-followup-seam

- [x] 1. 新增 `PendingMessageQueue`（mode / enqueue / drain / clear / len）+ 单元测试（all vs one-at-a-time）
- [x] 2. `Agent` 持有 steer/follow_up 队列（`Arc<Mutex<…>>`）；暴露 `steer` / `follow_up` / `clear_*` / `queue_stats`
- [x] 3. ReAct：迭代前 drain steering 注入 history；将停前 drain follow-up；发射 `QueueUpdate`
- [x] 4. `abort`：清空 steer，保留 follow_up；单测覆盖
- [x] 5. `Driver` + `InProcessDriver` 实现队列方法；`RemoteDriver` 预留或 Command 转发
- [x] 6. `protocol::Command` 增加 Steer/FollowUp/ClearQueue；`dispatch` 分支 + 单测
- [x] 7. settings `steering_mode` / `follow_up_mode` 经 builder/composition 传入 Agent
- [x] 8. 处理 `SteeringHooks`：文档化次要路径或删除死接线；避免双路径行为分歧
- [x] 9. `llman sdd validate c461-expose-steer-followup-seam --strict --no-interactive`
- [x] 10. `cargo test` 覆盖 queue + react 注入 + driver；`just lint` 相关目标

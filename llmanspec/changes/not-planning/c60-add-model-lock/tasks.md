# c60-add-model-lock Tasks

- [ ] 实现 ModelRegistry（模型实例生命周期管理）
- [ ] 实现 LockManager（Idle/Busy/Preemptible/Preempting 状态机）
- [ ] 实现优先级排队（priority + deadline）
- [ ] 实现 CheckpointManager（保存/恢复被中断任务上下文）
- [ ] 实现 SchedulerHint（任务声明资源占用）
- [ ] 编写测试（状态机转换、抢占恢复、死锁防护）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c60-add-model-lock --strict --no-interactive`

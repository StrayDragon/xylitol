# Tasks — c468-demo-steer-followup

> 先实现后补规格：下列任务在提案时已完成（见 `ab54a95`）。

- [x] 1. Delta specs：`app-tui-input` 增补 demo steer/follow-up 形态学 requirements
- [x] 2. `agent_demo`：`steer_queue` / `follow_up_queue`；忙碌 Enter→steer；Alt+Enter→follow-up
- [x] 3. 轮末/空闲 `drain_message_queues`；footer 队列计数；steer 不打断当前轮
- [x] 4. harness：`agent_demo_steer_does_not_abort_busy_turn`、`agent_demo_follow_up_queues_while_busy`
- [x] 5. 回写 `design/queue-steer.md`（demo 已验证）与 vs-pi / keys 提示
- [x] 6. `cargo test -p xylitol-tui --test agent_demo_test steer_does`；`follow_up_queues`；`llman sdd validate c468-demo-steer-followup --strict --no-interactive`

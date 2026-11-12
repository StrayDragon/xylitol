# Tasks — turn 模型复核收尾

> 实施者导读：spec 真值 = agent-runtime r3322；review 证据见验收记录。收口走 finalize，不列为任务。

- [ ] T1 C-1：steer 双路径路由（在途 → steer 队列；非在途 → follow_up FIFO）+ joins_next_submission 测试改写为新语义（steer 独立成 turn）
- [ ] T2 abort 对齐：FIFO 非空时 abort 后立即弹下一条开新 turn + 单测
- [ ] T3 补测：steer 在途注入当前 turn（runtime 级，断言无额外 AgentStart）
- [ ] T4 补测：context_reminder fork 祖先链保留（fork 于用户消息，断言提醒仍在祖先链）
- [ ] T5 app-tui-input 场景文本 run→turn ×2 + bindings 同步 + 该场景 BDD 绿
- [ ] T6 回归：just fmt / lint / test；llman-sdd validate c3268 --strict 绿

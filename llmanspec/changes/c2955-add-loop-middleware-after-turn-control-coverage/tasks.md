# Tasks

- [ ] 1. **新增场景**（spec）：在 `agent-runtime.feature` 的 `r2921` 下新增「after_turn 非法控制被忽略」：假如 注册 after_turn 中间件返回 stop 且入队 follow_up；当 运行 AgentRuntime 并收集事件；那么 after_turn 中间件已被调用且 run 继续到下一轮。测：`llman sdd validate c2955-add-loop-middleware-after-turn-control-coverage --strict`。
- [ ] 2. **实现 BDD 覆盖**（tests）：新增 `StopAfterTurn` 中间件、given/then step 与 scenario binding；断言调用计数 `>=1`、`TurnStart` 计数 `==2`、无 `LoopMiddleware` 错误。测：`cargo test --all-features --test bdd test_lm_after_turn_stop_ignored`。
- [ ] 3. **全量门禁**（repo）：`just qa`；确认测试数较基线 +1、BDD 场景较基线 +1。测：`just qa`。
